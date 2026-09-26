use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{mpsc, Mutex, MutexGuard};
use std::time::Duration;

use rodio::source::SeekError;
use rodio::{Decoder, OutputStream, Sink, Source};

static VOLUME: AtomicU32 = AtomicU32::new(75);
static PLAYING: AtomicBool = AtomicBool::new(false);
static PAUSED: AtomicBool = AtomicBool::new(false);

fn scale_volume(raw: u32) -> f32 {
    let v = raw as f32 / 100.0;
    (v * v).min(0.8)
}

pub const EQ_BAND_COUNT: usize = 10;

pub const EQ_FREQS: [f32; EQ_BAND_COUNT] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

pub const EQ_MIN_DB: f32 = -12.0;
pub const EQ_MAX_DB: f32 = 12.0;

const EQ_Q: f32 = 1.2;
const EQ_FLAT_DB: f32 = 0.05;

#[derive(Clone, Copy)]
struct EqConfig {
    enabled: bool,
    gains: [f32; EQ_BAND_COUNT],
}

impl Default for EqConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            gains: [0.0; EQ_BAND_COUNT],
        }
    }
}

static EQ_CONFIG: once_cell::sync::Lazy<Mutex<EqConfig>> =
    once_cell::sync::Lazy::new(|| Mutex::new(EqConfig::default()));

static EQ_GEN: AtomicU64 = AtomicU64::new(0);

fn eq_config() -> MutexGuard<'static, EqConfig> {
    EQ_CONFIG.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn set_eq(enabled: bool, gains: &[f32]) {
    let normalized = normalize_eq_bands(gains);
    {
        let mut cfg = eq_config();
        cfg.enabled = enabled;
        cfg.gains.copy_from_slice(&normalized);
    }
    EQ_GEN.fetch_add(1, Ordering::Release);
}

pub fn normalize_eq_bands(gains: &[f32]) -> Vec<f32> {
    (0..EQ_BAND_COUNT)
        .map(|i| {
            let raw = gains.get(i).copied().unwrap_or(0.0);
            if raw.is_finite() {
                raw.clamp(EQ_MIN_DB, EQ_MAX_DB)
            } else {
                0.0
            }
        })
        .collect()
}

#[derive(Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    active: bool,
}

impl Biquad {
    fn flat() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            active: false,
        }
    }

    fn peaking(freq: f32, gain_db: f32, q: f32, sample_rate: f32) -> Self {
        let mut filter = Self::flat();
        if gain_db.abs() < EQ_FLAT_DB || sample_rate <= 0.0 {
            return filter;
        }

        let a = 10f32.powf(gain_db / 40.0);
        let w0 = (std::f32::consts::PI * freq / sample_rate).min(std::f32::consts::PI * 0.999);
        let sin = w0.sin();
        let cos = w0.cos();
        let alpha = sin / (2.0 * q);

        let (b0, b1, b2) = (1.0 + alpha * a, -2.0 * cos, 1.0 - alpha * a);
        let (a0, a1, a2) = (1.0 + alpha / a, -2.0 * cos, 1.0 - alpha / a);

        filter.b0 = b0 / a0;
        filter.b1 = b1 / a0;
        filter.b2 = b2 / a0;
        filter.a1 = a1 / a0;
        filter.a2 = a2 / a0;
        filter.active = true;
        filter
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        if !self.active {
            return x;
        }
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

struct EqualizerSource<S> {
    inner: S,
    filters: Vec<[Biquad; EQ_BAND_COUNT]>,
    gains: [f32; EQ_BAND_COUNT],
    gen: u64,
    enabled: bool,
    channels: usize,
    sample_rate: u32,
    frame: u64,
}

impl<S> EqualizerSource<S>
where
    S: Source<Item = f32>,
{
    fn new(inner: S) -> Self {
        let sample_rate = inner.sample_rate().max(1);
        let channels = inner.channels().max(1) as usize;
        let cfg = *eq_config();
        let mut source = Self {
            inner,
            filters: Vec::new(),
            gains: cfg.gains,
            gen: EQ_GEN.load(Ordering::Acquire),
            enabled: cfg.enabled,
            channels,
            sample_rate,
            frame: 0,
        };
        source.rebuild();
        source
    }

    fn rebuild(&mut self) {
        self.filters.clear();
        let sample_rate = self.sample_rate as f32;
        for _ in 0..self.channels {
            self.filters.push(std::array::from_fn(|band| {
                Biquad::peaking(EQ_FREQS[band], self.gains[band], EQ_Q, sample_rate)
            }));
        }
    }

    #[inline]
    fn refresh(&mut self) {
        if EQ_GEN.load(Ordering::Acquire) == self.gen {
            return;
        }
        self.gen = EQ_GEN.load(Ordering::Acquire);
        let cfg = *eq_config();
        self.enabled = cfg.enabled;
        let sample_rate = self.inner.sample_rate().max(1);
        let channels = self.inner.channels().max(1) as usize;
        if cfg.gains != self.gains || sample_rate != self.sample_rate || channels != self.channels {
            self.gains = cfg.gains;
            self.sample_rate = sample_rate;
            self.channels = channels;
            self.rebuild();
        }
    }
}

impl<S> Iterator for EqualizerSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        self.refresh();
        if !self.enabled {
            return Some(sample);
        }

        let channel = (self.frame % self.channels as u64) as usize;
        self.frame = self.frame.wrapping_add(1);

        let mut value = sample;
        if let Some(filters) = self.filters.get_mut(channel) {
            for band in filters.iter_mut() {
                value = band.process(value);
            }
        }
        Some(value.clamp(-1.0, 1.0))
    }
}

impl<S> Source for EqualizerSource<S>
where
    S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.inner.try_seek(pos)
    }
}

fn valid_seek_time(time: f64) -> bool {
    time.is_finite() && time >= 0.0 && time <= Duration::MAX.as_secs_f64()
}

fn seek_sink(sink: &Sink, time: f64) -> Result<(), String> {
    if !valid_seek_time(time) {
        return Err("invalid seek time".into());
    }
    sink.try_seek(Duration::from_secs_f64(time))
        .map_err(|e| format!("seek: {e}"))
}

enum PlayerCmd {
    Play(String, Option<f64>, u64),
    Stop(u64),
    Pause(u64),
    Resume(u64),
    Seek(f64, u64),
    Volume(u32),
    GetPosition(u64),
    IsPlaying(u64),
    CheckFinished(u64),
}

#[derive(Clone)]
enum PlayerResp {
    Ok,
    F64(f64),
    Bool(bool),
    Err(String),
}

struct PlayerRequest {
    cmd: PlayerCmd,
    reply_tx: mpsc::Sender<PlayerResp>,
}

struct PlayerChannel {
    cmd_tx: mpsc::Sender<PlayerRequest>,
}

static CHANNEL: once_cell::sync::Lazy<PlayerChannel> = once_cell::sync::Lazy::new(|| {
    let (cmd_tx, cmd_rx) = mpsc::channel::<PlayerRequest>();

    std::thread::spawn(move || player_thread(cmd_rx));

    PlayerChannel { cmd_tx }
});

const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

fn to_result(resp: Result<PlayerResp, String>) -> Result<(), String> {
    match resp {
        Ok(PlayerResp::Ok) => Ok(()),
        Ok(PlayerResp::Err(e)) => Err(e),
        Err(e) => Err(e),
        _ => Err("unexpected".into()),
    }
}

fn send(cmd: PlayerCmd) -> Result<PlayerResp, String> {
    let (reply_tx, reply_rx) = mpsc::channel::<PlayerResp>();
    CHANNEL
        .cmd_tx
        .send(PlayerRequest { cmd, reply_tx })
        .map_err(|e| format!("send: {e}"))?;
    match reply_rx.recv_timeout(REPLY_TIMEOUT) {
        Ok(resp) => Ok(resp),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err("player unresponsive: command timed out".to_string())
        }
        Err(e) => Err(format!("recv: {e}")),
    }
}

fn player_thread(rx: mpsc::Receiver<PlayerRequest>) {
    let output_result = OutputStream::try_default();
    let (output, handle) = match output_result {
        Ok(v) => v,
        Err(e) => {
            eprintln!("audio output unavailable: {e}");
            while let Ok(req) = rx.recv() {
                let _ = req
                    .reply_tx
                    .send(PlayerResp::Err(format!("No audio output device: {e}")));
            }
            return;
        }
    };
    let mut sink: Option<Sink> = None;
    let mut playback_generation = 0u64;
    let mut active_generation = 0u64;
    let mut pending_seek: Option<(f64, u64)> = None;

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(PlayerRequest { cmd, reply_tx }) => {
                let resp = match cmd {
                    PlayerCmd::Play(path, seek_to, generation) => {
                        if generation < playback_generation {
                            PlayerResp::Ok
                        } else {
                            playback_generation = generation;
                            if pending_seek.as_ref().is_some_and(|(_, queued_generation)| {
                                *queued_generation != generation
                            }) {
                                pending_seek = None;
                            }
                            let queued_seek = pending_seek
                                .take()
                                .filter(|(_, queued_generation)| *queued_generation == generation)
                                .map(|(time, _)| time);
                            let requested_seek = seek_to.or(queued_seek);
                            if let Some(s) = sink.take() {
                                s.stop();
                            }
                            active_generation = 0;
                            PLAYING.store(false, Ordering::Relaxed);
                            PAUSED.store(false, Ordering::Relaxed);
                            match File::open(&path) {
                                Ok(file) => match Decoder::new(BufReader::new(file)) {
                                    Ok(decoder) => match Sink::try_new(&handle) {
                                        Ok(new_sink) => {
                                            new_sink.append(EqualizerSource::new(
                                                decoder.convert_samples::<f32>(),
                                            ));
                                            new_sink.set_volume(scale_volume(
                                                VOLUME.load(Ordering::Relaxed),
                                            ));
                                            let seek_result = requested_seek
                                                .map(|seek| seek_sink(&new_sink, seek))
                                                .transpose();
                                            match seek_result {
                                                Ok(Some(())) | Ok(None) => {
                                                    sink = Some(new_sink);
                                                    active_generation = generation;
                                                    PLAYING.store(true, Ordering::Relaxed);
                                                    PAUSED.store(false, Ordering::Relaxed);
                                                    PlayerResp::Ok
                                                }
                                                Err(e) => PlayerResp::Err(e),
                                            }
                                        }
                                        Err(e) => PlayerResp::Err(format!("sink: {e}")),
                                    },
                                    Err(e) => PlayerResp::Err(format!("decode: {e}")),
                                },
                                Err(e) => PlayerResp::Err(format!("open: {e}")),
                            }
                        }
                    }
                    PlayerCmd::Stop(generation) => {
                        if generation < playback_generation {
                            PlayerResp::Ok
                        } else {
                            playback_generation = generation;
                            active_generation = 0;
                            pending_seek = None;
                            if let Some(s) = sink.take() {
                                s.stop();
                            }
                            PLAYING.store(false, Ordering::Relaxed);
                            PAUSED.store(false, Ordering::Relaxed);
                            PlayerResp::Ok
                        }
                    }
                    PlayerCmd::Pause(generation) => {
                        if generation < playback_generation || generation != active_generation {
                            PlayerResp::Ok
                        } else {
                            if let Some(s) = &sink {
                                s.pause();
                                PAUSED.store(true, Ordering::Relaxed);
                            }
                            PlayerResp::Ok
                        }
                    }
                    PlayerCmd::Resume(generation) => {
                        if generation < playback_generation || generation != active_generation {
                            PlayerResp::Bool(false)
                        } else if let Some(s) = sink.as_ref().filter(|sink| !sink.empty()) {
                            s.play();
                            PLAYING.store(true, Ordering::Relaxed);
                            PAUSED.store(false, Ordering::Relaxed);
                            PlayerResp::Bool(true)
                        } else {
                            PLAYING.store(false, Ordering::Relaxed);
                            PAUSED.store(false, Ordering::Relaxed);
                            PlayerResp::Bool(false)
                        }
                    }
                    PlayerCmd::Seek(time, generation) => {
                        if !valid_seek_time(time) {
                            PlayerResp::Err("invalid seek time".into())
                        } else if generation < playback_generation {
                            PlayerResp::Ok
                        } else {
                            playback_generation = generation;
                            match sink.as_ref() {
                                Some(sink) if generation == active_generation => {
                                    match seek_sink(sink, time) {
                                        Ok(()) => PlayerResp::Ok,
                                        Err(e) => PlayerResp::Err(e),
                                    }
                                }
                                _ => {
                                    pending_seek = Some((time, generation));
                                    PlayerResp::Ok
                                }
                            }
                        }
                    }
                    PlayerCmd::Volume(vol) => {
                        let clamped = vol.min(100);
                        VOLUME.store(clamped, Ordering::Relaxed);
                        if let Some(s) = &sink {
                            s.set_volume(scale_volume(clamped));
                        }
                        PlayerResp::Ok
                    }
                    PlayerCmd::GetPosition(generation) => {
                        let pos = if generation == active_generation {
                            sink.as_ref()
                                .map(|s| s.get_pos().as_secs_f64())
                                .unwrap_or(0.0)
                        } else {
                            0.0
                        };
                        PlayerResp::F64(pos)
                    }
                    PlayerCmd::IsPlaying(generation) => PlayerResp::Bool(
                        generation == active_generation
                            && sink.as_ref().is_some_and(|sink| !sink.empty())
                            && !PAUSED.load(Ordering::Relaxed),
                    ),
                    PlayerCmd::CheckFinished(generation) => {
                        if generation != active_generation {
                            PlayerResp::Bool(false)
                        } else {
                            let finished = sink.as_ref().map(|s| s.empty()).unwrap_or(true);
                            if finished && PLAYING.load(Ordering::Relaxed) {
                                PLAYING.store(false, Ordering::Relaxed);
                            }
                            PlayerResp::Bool(finished)
                        }
                    }
                };

                let _ = reply_tx.send(resp);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = output;
}

pub fn play(path: &str, seek_to: Option<f64>, generation: u64) -> Result<(), String> {
    to_result(send(PlayerCmd::Play(path.to_string(), seek_to, generation)))
}

pub fn stop(generation: u64) -> Result<(), String> {
    to_result(send(PlayerCmd::Stop(generation)))
}

pub fn pause(generation: u64) -> Result<(), String> {
    to_result(send(PlayerCmd::Pause(generation)))
}

pub fn resume(generation: u64) -> Result<bool, String> {
    match send(PlayerCmd::Resume(generation)) {
        Ok(PlayerResp::Bool(resumed)) => Ok(resumed),
        Ok(PlayerResp::Err(e)) => Err(e),
        Err(e) => Err(e),
        _ => Err("unexpected".into()),
    }
}

pub fn seek(time: f64, generation: u64) -> Result<(), String> {
    to_result(send(PlayerCmd::Seek(time, generation)))
}

pub fn set_volume(vol: u32) {
    let _ = send(PlayerCmd::Volume(vol));
}

pub fn get_position(generation: u64) -> Result<f64, String> {
    match send(PlayerCmd::GetPosition(generation)) {
        Ok(PlayerResp::F64(v)) => Ok(v),
        Ok(PlayerResp::Err(e)) => Err(e),
        Ok(_) => Err("unexpected".into()),
        Err(e) => Err(e),
    }
}

pub fn is_playing(generation: u64) -> Result<bool, String> {
    match send(PlayerCmd::IsPlaying(generation)) {
        Ok(PlayerResp::Bool(v)) => Ok(v),
        Ok(PlayerResp::Err(e)) => Err(e),
        Ok(_) => Err("unexpected".into()),
        Err(e) => Err(e),
    }
}

pub fn check_finished(generation: u64) -> Result<bool, String> {
    match send(PlayerCmd::CheckFinished(generation)) {
        Ok(PlayerResp::Bool(v)) => Ok(v),
        Ok(PlayerResp::Err(e)) => Err(e),
        Ok(_) => Err("unexpected".into()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::valid_seek_time;

    #[test]
    fn seek_time_validation_rejects_invalid_values() {
        assert!(valid_seek_time(0.0));
        assert!(valid_seek_time(12.5));
        assert!(!valid_seek_time(-0.1));
        assert!(!valid_seek_time(f64::NAN));
        assert!(!valid_seek_time(f64::INFINITY));
        assert!(!valid_seek_time(f64::NEG_INFINITY));
    }
}
