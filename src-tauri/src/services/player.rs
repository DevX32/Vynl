use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, mpsc};
use std::time::{Duration, Instant};

use rodio::source::SeekError;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

static VOLUME: AtomicU32 = AtomicU32::new(75);
static PAUSED: AtomicBool = AtomicBool::new(false);

pub const PLAYBACK_TICK_EVENT: &str = "vynl:player:tick";
const TICK_INTERVAL: Duration = Duration::from_millis(100);
const DEVICE_POLL_INTERVAL: Duration = Duration::from_secs(2);
static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn set_app_handle(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

#[derive(Clone, Serialize)]
pub struct PlaybackTick {
    pub generation: u64,
    pub position: f64,
    pub playing: bool,
    pub finished: bool,
    pub duration: Option<f64>,
}

type TickKey = (u64, bool, bool);

fn reported_pos(sink: Option<&Sink>, floor: Option<Duration>) -> f64 {
    let pos = sink.map(|s| s.get_pos()).unwrap_or_default();
    pos.max(floor.unwrap_or_default()).as_secs_f64()
}

fn emit_playback_tick(
    generation: u64,
    sink: Option<&Sink>,
    seek_floor: Option<Duration>,
    duration_secs: Option<f64>,
    last_key: &mut Option<TickKey>,
) {
    if generation == 0 {
        return;
    }

    let has_sink = sink.is_some();
    let empty = sink.map(|s| s.empty()).unwrap_or(false);
    let paused = PAUSED.load(Ordering::Relaxed);
    let playing = has_sink && !empty && !paused;
    let finished = has_sink && empty && !paused;

    let key: TickKey = (generation, playing, finished);
    if !playing && *last_key == Some(key) {
        return;
    }
    *last_key = Some(key);

    let Some(app) = APP.get() else { return };
    let position = reported_pos(sink, seek_floor);
    let _ = app.emit(
        PLAYBACK_TICK_EVENT,
        PlaybackTick {
            generation,
            position,
            playing,
            finished,
            duration: duration_secs,
        },
    );
}

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

const EQ_QS: [f32; EQ_BAND_COUNT] = [0.7, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.4, 1.2, 1.0];
const EQ_LOW_SHELF: usize = 0;
const EQ_HIGH_SHELF: usize = EQ_BAND_COUNT - 1;
const EQ_SHAPE: f32 = 1.7;

const EQ_FLAT_DB: f32 = 0.05;
const EQ_RAMP_SECS: f32 = 0.04;
const EQ_LIMIT_KNEE: f32 = 0.9;
const EQ_LIMIT_CEIL: f32 = 0.999;
const EQ_PREAMP_TRIM: f32 = 0.5;

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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EqDesign {
    pub freqs: [f32; EQ_BAND_COUNT],
    pub qs: [f32; EQ_BAND_COUNT],
    pub low_shelf: usize,
    pub high_shelf: usize,
    pub shape: f32,
    pub flat_db: f32,
    pub preamp_trim: f32,
    pub min_db: f32,
    pub max_db: f32,
}

pub fn eq_design() -> EqDesign {
    EqDesign {
        freqs: EQ_FREQS,
        qs: EQ_QS,
        low_shelf: EQ_LOW_SHELF,
        high_shelf: EQ_HIGH_SHELF,
        shape: EQ_SHAPE,
        flat_db: EQ_FLAT_DB,
        preamp_trim: EQ_PREAMP_TRIM,
        min_db: EQ_MIN_DB,
        max_db: EQ_MAX_DB,
    }
}

pub fn eq_response_db(gains: &[f32], freq: f32, sample_rate: f32) -> f32 {
    if sample_rate <= 0.0 || !freq.is_finite() || freq <= 0.0 {
        return 0.0;
    }
    let w = 2.0 * std::f32::consts::PI * freq / sample_rate;
    let (sin, cos) = w.sin_cos();
    let cos2 = 2.0 * cos * cos - 1.0;
    let sin2 = 2.0 * sin * cos;
    let mut db = 0.0f32;
    for band in 0..EQ_BAND_COUNT {
        let gain = gains.get(band).copied().unwrap_or(0.0);
        let c = Coeffs::design(band, gain, sample_rate);
        let num = ((c.b0 + c.b1 * cos + c.b2 * cos2).powi(2)
            + (c.b1 * -sin + c.b2 * -sin2).powi(2))
        .sqrt();
        let den = ((1.0 + c.a1 * cos + c.a2 * cos2).powi(2) + (c.a1 * -sin + c.a2 * -sin2).powi(2))
            .sqrt();
        if num <= 0.0 || den <= 0.0 {
            return 0.0;
        }
        db += 20.0 * (num / den).log10();
    }
    db
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

#[derive(Clone, Copy, PartialEq, Debug)]
struct Coeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Coeffs {
    const FLAT: Self = Self {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };

    fn peaking(w0: f32, q: f32, gain_db: f32) -> [f32; 6] {
        let a = 10f32.powf(gain_db / 40.0);
        let cos = w0.cos();
        let alpha = w0.sin() / (2.0 * q);
        [
            1.0 + alpha * a,
            -2.0 * cos,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * cos,
            1.0 - alpha / a,
        ]
    }

    fn shelf(w0: f32, shape: f32, gain_db: f32, high: bool) -> [f32; 6] {
        let amp = 10f64.powf(f64::from(gain_db) / 20.0);
        let m = f64::from(shape);
        let k = (f64::from(w0) * 0.5).tan();
        let k2 = k * k;
        let mid = m * amp.sqrt() * k;
        let (b0, b1, b2) = if high {
            (amp + mid + k2, 2.0 * (k2 - amp), amp - mid + k2)
        } else {
            (
                1.0 + mid + amp * k2,
                2.0 * (amp * k2 - 1.0),
                1.0 - mid + amp * k2,
            )
        };
        let (a0, a1, a2) = (1.0 + m * k + k2, 2.0 * (k2 - 1.0), 1.0 - m * k + k2);
        let inv = (1.0 / k2).min(f32::MAX as f64);
        [b0, b1, b2, a0, a1, a2].map(|v| (v * inv) as f32)
    }

    fn design(band: usize, gain_db: f32, sample_rate: f32) -> Self {
        let freq = EQ_FREQS[band];
        if gain_db.abs() < EQ_FLAT_DB || sample_rate <= 0.0 || freq >= sample_rate * 0.49 {
            return Self::FLAT;
        }

        let w0 =
            (2.0 * std::f32::consts::PI * freq / sample_rate).min(std::f32::consts::PI * 0.999);
        let high = band == EQ_HIGH_SHELF;
        let [b0, b1, b2, a0, a1, a2] = if matches!(band, EQ_LOW_SHELF | EQ_HIGH_SHELF) {
            Self::shelf(w0, EQ_SHAPE, gain_db, high)
        } else {
            Self::peaking(w0, EQ_QS[band], gain_db)
        };

        let coeffs = Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        };
        if [coeffs.b0, coeffs.b1, coeffs.b2, coeffs.a1, coeffs.a2]
            .iter()
            .any(|v| !v.is_finite())
            || coeffs.a1.abs() > 2.0
            || coeffs.a2.abs() >= 1.0
        {
            return Self::FLAT;
        }
        coeffs
    }

    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            b0: self.b0 + (other.b0 - self.b0) * t,
            b1: self.b1 + (other.b1 - self.b1) * t,
            b2: self.b2 + (other.b2 - self.b2) * t,
            a1: self.a1 + (other.a1 - self.a1) * t,
            a2: self.a2 + (other.a2 - self.a2) * t,
        }
    }
}

struct EqBand {
    cur: Coeffs,
    target: Coeffs,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl EqBand {
    fn settled(band: usize, gains: &[f32; EQ_BAND_COUNT], sample_rate: f32) -> Self {
        let coeffs = Coeffs::design(band, gains[band], sample_rate);
        Self {
            cur: coeffs,
            target: coeffs,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn aim(&mut self, band: usize, gains: &[f32; EQ_BAND_COUNT], sample_rate: f32) {
        self.target = Coeffs::design(band, gains[band], sample_rate);
    }

    #[inline]
    fn glide(&mut self, step: f32) {
        self.cur = self.cur.lerp(self.target, step);
    }

    #[inline]
    fn settle(&mut self) {
        self.cur = self.target;
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let y = self.cur.b0 * x + self.cur.b1 * self.x1 + self.cur.b2 * self.x2
            - self.cur.a1 * self.y1
            - self.cur.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

fn preamp_gain(gains: &[f32; EQ_BAND_COUNT]) -> f32 {
    let peak = gains
        .iter()
        .copied()
        .filter(|g| *g > 0.0)
        .fold(0.0f32, f32::max);
    if peak <= 0.0 {
        return 1.0;
    }
    10f32.powf(-(peak * EQ_PREAMP_TRIM) / 20.0)
}

#[inline]
fn soft_limit(x: f32) -> f32 {
    let mag = x.abs();
    if mag <= EQ_LIMIT_KNEE {
        return x;
    }
    let head = EQ_LIMIT_CEIL - EQ_LIMIT_KNEE;
    let shaped = EQ_LIMIT_KNEE + head * (1.0 - (-(mag - EQ_LIMIT_KNEE) / head).exp());
    x.signum() * shaped.min(EQ_LIMIT_CEIL)
}

struct EqualizerSource<S> {
    inner: S,
    bands: Vec<[EqBand; EQ_BAND_COUNT]>,
    gains: [f32; EQ_BAND_COUNT],
    generation: u64,
    channels: usize,
    sample_rate: u32,
    frame: u64,
    ramp: u32,
    wet: f32,
    wet_target: f32,
    preamp: f32,
    preamp_target: f32,
}

impl<S> EqualizerSource<S>
where
    S: Source<Item = f32>,
{
    fn new(inner: S) -> Self {
        let sample_rate = inner.sample_rate().max(1);
        let channels = inner.channels().max(1) as usize;
        let cfg = *eq_config();
        let preamp = preamp_gain(&cfg.gains);
        let wet = if cfg.enabled { 1.0 } else { 0.0 };
        let mut source = Self {
            inner,
            bands: Vec::new(),
            gains: cfg.gains,
            generation: EQ_GEN.load(Ordering::Acquire),
            channels,
            sample_rate,
            frame: 0,
            ramp: 0,
            wet,
            wet_target: wet,
            preamp,
            preamp_target: preamp,
        };
        source.build();
        source
    }

    fn build(&mut self) {
        let sample_rate = self.sample_rate as f32;
        self.bands = (0..self.channels)
            .map(|_| std::array::from_fn(|band| EqBand::settled(band, &self.gains, sample_rate)))
            .collect();
    }

    #[inline]
    fn refresh(&mut self) {
        if EQ_GEN.load(Ordering::Acquire) == self.generation {
            return;
        }
        self.generation = EQ_GEN.load(Ordering::Acquire);
        let cfg = *eq_config();
        let sample_rate = self.inner.sample_rate().max(1);
        let channels = self.inner.channels().max(1) as usize;

        self.wet_target = if cfg.enabled { 1.0 } else { 0.0 };
        self.preamp_target = preamp_gain(&cfg.gains);

        if sample_rate != self.sample_rate || channels != self.channels {
            self.sample_rate = sample_rate;
            self.channels = channels;
            self.gains = cfg.gains;
            self.wet = self.wet_target;
            self.preamp = self.preamp_target;
            self.build();
            self.ramp = 0;
            return;
        }

        self.gains = cfg.gains;
        let rate = sample_rate as f32;
        for chain in self.bands.iter_mut() {
            for (band, eq) in chain.iter_mut().enumerate() {
                eq.aim(band, &self.gains, rate);
            }
        }
        self.ramp = (rate * EQ_RAMP_SECS * channels as f32).round().max(1.0) as u32;
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

        let channel = (self.frame % self.channels as u64) as usize;
        self.frame = self.frame.wrapping_add(1);

        if self.ramp == 0 && self.wet == 0.0 {
            return Some(sample);
        }
        let Some(chain) = self.bands.get_mut(channel) else {
            return Some(sample);
        };

        if self.ramp > 0 {
            let step = 1.0 / self.ramp as f32;
            self.wet += (self.wet_target - self.wet) * step;
            self.preamp += (self.preamp_target - self.preamp) * step;
            for band in chain.iter_mut() {
                band.glide(step);
            }
            self.ramp -= 1;
            if self.ramp == 0 {
                self.wet = self.wet_target;
                self.preamp = self.preamp_target;
                for band in chain.iter_mut() {
                    band.settle();
                }
            }
        }

        let mut filtered = sample * self.preamp;
        for band in chain.iter_mut() {
            filtered = band.process(filtered);
        }

        Some(soft_limit(sample + self.wet * (filtered - sample)))
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

#[derive(Clone)]
struct FadeControl(Arc<FadeOrder>);
struct FadeOrder {
    to: AtomicU32,
    ramp: AtomicU32,
    serial: AtomicU32,
    live: AtomicU32,
}

impl FadeControl {
    fn settled(gain: f32) -> Self {
        let gain = gain.clamp(0.0, 1.0);
        Self(Arc::new(FadeOrder {
            to: AtomicU32::new(gain.to_bits()),
            ramp: AtomicU32::new(0),
            serial: AtomicU32::new(0),
            live: AtomicU32::new(gain.to_bits()),
        }))
    }

    fn ramp_to(&self, to: f32, secs: f64, sample_rate: u32) {
        let samples = if secs.is_finite() && secs > 0.0 && sample_rate > 0 {
            (secs * sample_rate as f64).round().min(u32::MAX as f64) as u32
        } else {
            0
        };
        self.0
            .to
            .store(to.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
        self.0.ramp.store(samples, Ordering::Relaxed);
        self.0.serial.fetch_add(1, Ordering::Release);
    }

    fn take_order(&self, since: Option<u32>) -> Option<(u32, f32, u32)> {
        let serial = self.0.serial.load(Ordering::Acquire);
        if serial != self.0.serial.load(Ordering::Acquire) || Some(serial) == since {
            return None;
        }
        Some((
            serial,
            f32::from_bits(self.0.to.load(Ordering::Relaxed)),
            self.0.ramp.load(Ordering::Relaxed),
        ))
    }

    fn current(&self) -> f32 {
        f32::from_bits(self.0.live.load(Ordering::Relaxed))
    }

    fn publish(&self, gain: f32) {
        self.0
            .live
            .store(gain.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
}

struct FadeSource<S> {
    inner: S,
    control: FadeControl,
    gain: f32,
    to: f32,
    step: f32,
    left: u32,
    serial: Option<u32>,
}

impl<S> FadeSource<S>
where
    S: Source<Item = f32>,
{
    fn new(inner: S, control: FadeControl) -> Self {
        let gain = control.current();
        Self {
            inner,
            control,
            serial: None,
            gain,
            to: gain,
            step: 0.0,
            left: 0,
        }
    }

    fn retarget(&mut self) {
        let Some((serial, to, ramp)) = self.control.take_order(self.serial) else {
            return;
        };
        self.serial = Some(serial);
        self.to = to;
        self.left = ramp;
        self.step = if ramp > 0 {
            (to - self.gain) / ramp as f32
        } else {
            0.0
        };
    }

    #[inline]
    fn glide(&mut self, sample: f32) -> f32 {
        if self.left > 0 {
            self.gain += self.step;
            self.left -= 1;
            if self.left == 0 {
                self.gain = self.to;
                self.control.publish(self.gain);
            }
        } else if self.gain != self.to {
            self.gain = self.to;
            self.control.publish(self.gain);
        }
        sample * self.gain
    }
}

impl<S> Iterator for FadeSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        self.retarget();
        let sample = self.inner.next()?;
        Some(self.glide(sample))
    }
}

impl<S> Source for FadeSource<S>
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

struct TrackSink {
    sink: Sink,
    fade: FadeControl,
    sample_rate: u32,
}

impl TrackSink {
    fn open(
        handle: &OutputStreamHandle,
        path: &str,
        fade_secs: f64,
    ) -> Result<(Self, Option<f64>), String> {
        let file = File::open(path).map_err(|e| format!("open: {e}"))?;
        let decoder = Decoder::new(BufReader::new(file)).map_err(|e| format!("decode: {e}"))?;
        let total = decoder.total_duration().map(|d| d.as_secs_f64());
        let samples = decoder.convert_samples::<f32>();
        let sample_rate = samples.sample_rate().max(1);
        let fade_secs = if fade_secs.is_finite() {
            fade_secs
        } else {
            0.0
        };
        let control = FadeControl::settled(if fade_secs > 0.0 { 0.0 } else { 1.0 });
        if fade_secs > 0.0 {
            control.ramp_to(1.0, fade_secs, sample_rate);
        }
        let sink = Sink::try_new(handle).map_err(|e| format!("sink: {e}"))?;
        sink.append(FadeSource::new(
            EqualizerSource::new(samples),
            control.clone(),
        ));
        Ok((
            Self {
                sink,
                fade: control,
                sample_rate,
            },
            total,
        ))
    }
}

fn open_track(
    handle: Option<&OutputStreamHandle>,
    path: &str,
    fade_secs: f64,
) -> Result<(TrackSink, Option<f64>), String> {
    let handle = handle.ok_or_else(|| "No audio output device".to_string())?;
    TrackSink::open(handle, path, fade_secs)
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
    Crossfade(String, u64, f64),
    Stop(u64),
    Pause(u64),
    Resume(u64),
    Seek(f64, u64),
    Volume(u32),
    GetPosition(u64),
    IsPlaying(u64),
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

fn default_output_device_name() -> Option<String> {
    use rodio::cpal::traits::{DeviceTrait, HostTrait};
    let host = rodio::cpal::default_host();
    let device = host.default_output_device()?;
    device.name().ok()
}

pub const MAX_CROSSFADE_SECS: f64 = 20.0;

struct FadingOut {
    track: TrackSink,
    until: Duration,
}

fn stop_fading_out(fading: &mut Option<FadingOut>) {
    if let Some(out) = fading.take() {
        out.track.sink.stop();
    }
}

fn seek_floor_for(time: f64) -> Option<Duration> {
    valid_seek_time(time).then(|| Duration::from_secs_f64(time))
}

fn begin_crossfade(
    handle: &Option<OutputStreamHandle>,
    sink: &mut Option<TrackSink>,
    fading: &mut Option<FadingOut>,
    path: &str,
    generation: u64,
    fade_secs: f64,
) -> Result<Option<f64>, String> {
    let handle = handle
        .as_ref()
        .ok_or_else(|| "No audio output device".to_string())?;
    if generation == 0 {
        return Err("no active generation".into());
    }
    if !fade_secs.is_finite() || fade_secs <= 0.0 {
        return Err("invalid fade length".into());
    }
    if PAUSED.load(Ordering::Relaxed) {
        return Err("player is paused".into());
    }
    let start = match sink.as_ref().map(|t| &t.sink) {
        Some(s) if !s.empty() => s.get_pos(),
        _ => return Err("nothing playing".into()),
    };

    let fade_secs = fade_secs.min(MAX_CROSSFADE_SECS);
    let (incoming, total) = TrackSink::open(handle, path, fade_secs)?;
    let volume = scale_volume(VOLUME.load(Ordering::Relaxed));
    incoming.sink.set_volume(volume);
    let outgoing = sink.replace(incoming).ok_or("nothing playing")?;
    outgoing.sink.set_volume(volume);
    outgoing.fade.ramp_to(0.0, fade_secs, outgoing.sample_rate);
    stop_fading_out(fading);
    *fading = Some(FadingOut {
        track: outgoing,
        until: start + Duration::from_secs_f64(fade_secs),
    });

    Ok(total)
}

fn reopen_output(
    output: &mut Option<OutputStream>,
    handle: &mut Option<OutputStreamHandle>,
    sink: &mut Option<TrackSink>,
    fading: &mut Option<FadingOut>,
    seek_floor: &mut Option<Duration>,
    current_path: &Option<String>,
) {
    let resume_pos = sink
        .as_ref()
        .map(|t| &t.sink)
        .filter(|s| !s.empty() && !PAUSED.load(Ordering::Relaxed))
        .map(|s| reported_pos(Some(s), *seek_floor));
    if let Some(t) = sink.take() {
        t.sink.stop();
    }
    stop_fading_out(fading);
    drop(handle.take());
    drop(output.take());

    let (new_output, new_handle) = match OutputStream::try_default() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("device change: reopen output failed: {e}");
            return;
        }
    };
    *output = Some(new_output);
    *handle = Some(new_handle);

    let (Some(path), Some(pos)) = (current_path.clone(), resume_pos) else {
        return;
    };

    match open_track(handle.as_ref(), &path, 0.0) {
        Ok((track, _)) => {
            track
                .sink
                .set_volume(scale_volume(VOLUME.load(Ordering::Relaxed)));
            if let Err(e) = seek_sink(&track.sink, pos) {
                eprintln!("device change: could not resume {path}: {e}");
            } else {
                *seek_floor = seek_floor_for(pos);
            }
            *sink = Some(track);
        }
        Err(e) => eprintln!("device change: could not reopen {path}: {e}"),
    }
}

fn player_thread(rx: mpsc::Receiver<PlayerRequest>) {
    let (out, h) = match OutputStream::try_default() {
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

    let mut output = Some(out);
    let mut handle = Some(h);
    let mut sink: Option<TrackSink> = None;
    let mut fading: Option<FadingOut> = None;
    let mut current_path: Option<String> = None;
    let mut output_device_name = default_output_device_name();
    let mut playback_generation = 0u64;
    let mut active_generation = 0u64;
    let mut pending_seek: Option<(f64, u64)> = None;
    let mut seek_floor: Option<Duration> = None;
    let mut duration_secs: Option<f64> = None;
    let mut last_tick_key: Option<TickKey> = None;
    let mut last_device_poll = Instant::now();

    loop {
        match rx.recv_timeout(TICK_INTERVAL) {
            Ok(PlayerRequest { cmd, reply_tx }) => {
                let resp = match cmd {
                    PlayerCmd::Play(path, seek_to, generation) => {
                        if generation < playback_generation {
                            PlayerResp::Ok
                        } else {
                            playback_generation = generation;
                            if handle.is_none()
                                && let Ok((o, h)) = OutputStream::try_default()
                            {
                                output = Some(o);
                                handle = Some(h);
                            }
                            if handle.is_none() {
                                let _ = reply_tx
                                    .send(PlayerResp::Err("No audio output device".to_string()));
                                continue;
                            }
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
                            stop_fading_out(&mut fading);
                            if let Some(t) = sink.take() {
                                t.sink.stop();
                            }
                            current_path = Some(path.clone());
                            active_generation = 0;
                            PAUSED.store(false, Ordering::Relaxed);
                            match open_track(handle.as_ref(), &path, 0.0).and_then(
                                |(track, total)| {
                                    track
                                        .sink
                                        .set_volume(scale_volume(VOLUME.load(Ordering::Relaxed)));
                                    let seek_result = requested_seek
                                        .map(|seek| seek_sink(&track.sink, seek))
                                        .transpose();
                                    match seek_result {
                                        Ok(Some(())) | Ok(None) => Ok((track, total)),
                                        Err(e) => Err(e),
                                    }
                                },
                            ) {
                                Ok((track, total)) => {
                                    sink = Some(track);
                                    duration_secs = total;
                                    active_generation = generation;
                                    seek_floor = requested_seek.and_then(seek_floor_for);
                                    PAUSED.store(false, Ordering::Relaxed);
                                    PlayerResp::Ok
                                }
                                Err(e) => PlayerResp::Err(e),
                            }
                        }
                    }
                    PlayerCmd::Crossfade(path, generation, fade_secs) => {
                        if generation <= playback_generation {
                            let _ = reply_tx.send(PlayerResp::Err("stale generation".to_string()));
                            continue;
                        }
                        match begin_crossfade(
                            &handle,
                            &mut sink,
                            &mut fading,
                            &path,
                            generation,
                            fade_secs,
                        ) {
                            Ok(total) => {
                                playback_generation = generation;
                                active_generation = generation;
                                pending_seek = None;
                                seek_floor = None;
                                current_path = Some(path);
                                duration_secs = total;
                                PAUSED.store(false, Ordering::Relaxed);
                                PlayerResp::Ok
                            }
                            Err(e) => PlayerResp::Err(e),
                        }
                    }
                    PlayerCmd::Stop(generation) => {
                        if generation < playback_generation {
                            PlayerResp::Ok
                        } else {
                            playback_generation = generation;
                            active_generation = 0;
                            pending_seek = None;
                            seek_floor = None;
                            duration_secs = None;
                            current_path = None;
                            stop_fading_out(&mut fading);
                            if let Some(t) = sink.take() {
                                t.sink.stop();
                            }
                            PAUSED.store(false, Ordering::Relaxed);
                            PlayerResp::Ok
                        }
                    }
                    PlayerCmd::Pause(generation) => {
                        if generation < playback_generation || generation != active_generation {
                            PlayerResp::Ok
                        } else {
                            if let Some(t) = &sink {
                                t.sink.pause();
                            }
                            if let Some(out) = &fading {
                                out.track.sink.pause();
                            }
                            PAUSED.store(true, Ordering::Relaxed);
                            PlayerResp::Ok
                        }
                    }
                    PlayerCmd::Resume(generation) => {
                        if generation < playback_generation || generation != active_generation {
                            PlayerResp::Bool(false)
                        } else if let Some(t) = sink.as_ref().filter(|t| !t.sink.empty()) {
                            t.sink.play();
                            if let Some(out) = &fading {
                                out.track.sink.play();
                            }
                            PAUSED.store(false, Ordering::Relaxed);
                            PlayerResp::Bool(true)
                        } else {
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
                                Some(track) if generation == active_generation => {
                                    match seek_sink(&track.sink, time) {
                                        Ok(()) => {
                                            seek_floor = seek_floor_for(time);
                                            PlayerResp::Ok
                                        }
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
                        let scaled = scale_volume(clamped);
                        VOLUME.store(clamped, Ordering::Relaxed);
                        if let Some(t) = &sink {
                            t.sink.set_volume(scaled);
                        }
                        if let Some(out) = &fading {
                            out.track.sink.set_volume(scaled);
                        }
                        PlayerResp::Ok
                    }
                    PlayerCmd::GetPosition(generation) => {
                        let pos = if generation == active_generation {
                            reported_pos(sink.as_ref().map(|t| &t.sink), seek_floor)
                        } else {
                            0.0
                        };
                        PlayerResp::F64(pos)
                    }
                    PlayerCmd::IsPlaying(generation) => PlayerResp::Bool(
                        generation == active_generation
                            && sink.as_ref().is_some_and(|t| !t.sink.empty())
                            && !PAUSED.load(Ordering::Relaxed),
                    ),
                };

                let _ = reply_tx.send(resp);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if last_device_poll.elapsed() >= DEVICE_POLL_INTERVAL {
                    last_device_poll = Instant::now();
                    let now = default_output_device_name();
                    if now != output_device_name {
                        output_device_name = now;
                        reopen_output(
                            &mut output,
                            &mut handle,
                            &mut sink,
                            &mut fading,
                            &mut seek_floor,
                            &current_path,
                        );
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if fading
            .as_ref()
            .is_some_and(|out| out.track.sink.get_pos() >= out.until)
        {
            stop_fading_out(&mut fading);
        }
        emit_playback_tick(
            active_generation,
            sink.as_ref().map(|t| &t.sink),
            seek_floor,
            duration_secs,
            &mut last_tick_key,
        );
    }

    drop(output);
    drop(handle);
}

pub fn play(path: &str, seek_to: Option<f64>, generation: u64) -> Result<(), String> {
    to_result(send(PlayerCmd::Play(path.to_string(), seek_to, generation)))
}

pub fn crossfade(path: &str, generation: u64, fade_secs: f64) -> Result<(), String> {
    to_result(send(PlayerCmd::Crossfade(
        path.to_string(),
        generation,
        fade_secs,
    )))
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
