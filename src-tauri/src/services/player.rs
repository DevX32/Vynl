use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock, mpsc};
use std::time::{Duration, Instant};

use rodio::source::SeekError;
use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::services::catalogue::stream::{PreparedAudio, StreamSession, StreamSource};

mod eq;

pub(crate) use eq::EqualizerSource;
pub use eq::{
    EQ_BAND_COUNT, EQ_FREQS, EQ_MAX_DB, EQ_MIN_DB, EqDesign, eq_design, normalize_eq_bands, set_eq,
};

static VOLUME: AtomicU32 = AtomicU32::new(75);
static PAUSED: AtomicBool = AtomicBool::new(false);
static SPEED: AtomicU32 = AtomicU32::new(1.0f32.to_bits());
static REPLAY_GAIN: AtomicBool = AtomicBool::new(false);

const MIN_SPEED: f32 = 0.5;
const MAX_SPEED: f32 = 2.0;
const REPLAY_GAIN_MIN_LINEAR: f32 = 0.25;
const REPLAY_GAIN_MAX_LINEAR: f32 = 4.0;

pub const PLAYBACK_TICK_EVENT: &str = "vynl:player:tick";
const TICK_INTERVAL: Duration = Duration::from_millis(100);
const DEVICE_POLL_INTERVAL: Duration = Duration::from_secs(2);
static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn set_app_handle(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

pub fn app_handle() -> Option<AppHandle> {
    APP.get().cloned()
}

#[derive(Clone, Serialize)]
pub struct PlaybackTick {
    pub generation: u64,
    pub position: f64,
    pub playing: bool,
    pub finished: bool,
    pub duration: Option<f64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamErrorEvent {
    pub video_id: String,
    pub message: String,
    pub generation: u64,
}

type TickKey = (u64, bool, bool, u64);

fn reported_pos(sink: Option<&TrackSink>, floor: Option<Duration>) -> f64 {
    let pos = sink
        .map(|s| s.sink.get_pos() + s.position_offset)
        .unwrap_or_default();
    pos.max(floor.unwrap_or_default()).as_secs_f64()
}

fn emit_playback_tick(
    generation: u64,
    sink: Option<&TrackSink>,
    seek_floor: Option<Duration>,
    duration_secs: Option<f64>,
    last_key: &mut Option<TickKey>,
) {
    if generation == 0 {
        return;
    }

    let has_sink = sink.is_some();
    let empty = sink.map(|s| s.sink.empty()).unwrap_or(false);
    let paused = PAUSED.load(Ordering::Relaxed);
    let starved = sink
        .as_ref()
        .and_then(|track| track._session.as_ref())
        .is_some_and(|session| session.recently_underrun(750));
    let playing = has_sink && !empty && !paused && !starved;
    let finished = has_sink && empty && !paused;
    let position = reported_pos(sink, seek_floor);

    let key: TickKey = (generation, playing, finished, position.to_bits());
    if !playing && *last_key == Some(key) {
        return;
    }
    *last_key = Some(key);

    let Some(app) = APP.get() else { return };
    if has_sink
        && empty
        && let Some(track) = sink
        && let Some(session) = &track._session
        && let Some(message) = session.error()
    {
        let _ = app.emit(
            "vynl:player:stream-error",
            StreamErrorEvent {
                video_id: session.video_id.clone(),
                message,
                generation,
            },
        );
    }
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

fn emit_stream_error(path: &str, message: &str, generation: u64) {
    let video_id = path.strip_prefix("ytm:").unwrap_or_default().to_string();
    let Some(app) = APP.get() else { return };
    let _ = app.emit(
        "vynl:player:stream-error",
        StreamErrorEvent {
            video_id,
            message: message.to_string(),
            generation,
        },
    );
}

fn scale_volume(raw: u32) -> f32 {
    let v = raw as f32 / 100.0;
    (v * v).min(0.8)
}

fn clamp_speed(speed: f32) -> f32 {
    if speed.is_finite() {
        speed.clamp(MIN_SPEED, MAX_SPEED)
    } else {
        1.0
    }
}

fn playback_speed() -> f32 {
    f32::from_bits(SPEED.load(Ordering::Relaxed))
}

fn replay_gain_linear(db: f32) -> f32 {
    if db.is_finite() {
        10f32
            .powf(db / 20.0)
            .clamp(REPLAY_GAIN_MIN_LINEAR, REPLAY_GAIN_MAX_LINEAR)
    } else {
        1.0
    }
}

fn valid_seek_time(time: f64) -> bool {
    time.is_finite() && time >= 0.0 && time < Duration::MAX.as_secs_f64()
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

    fn ramp_to(&self, to: f32, secs: f64, sample_rate: u32, channels: u16) {
        let samples = if secs.is_finite() && secs > 0.0 && sample_rate > 0 {
            (secs * sample_rate as f64 * channels.max(1) as f64)
                .round()
                .min(u32::MAX as f64) as u32
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

type DecodedStream = (
    Box<dyn Source<Item = f32> + Send>,
    Option<f64>,
    Option<Arc<StreamSession>>,
);

enum TrackInput {
    File(PathBuf),
    CatalogueFile(PathBuf),
    Stream(Arc<StreamSession>),
}

impl TrackInput {
    fn is_stream(&self) -> bool {
        matches!(self, Self::Stream(_))
    }
}

fn track_replay_gain(input: &TrackInput) -> Option<f32> {
    if !REPLAY_GAIN.load(Ordering::Relaxed) {
        return None;
    }
    match input {
        TrackInput::File(path) | TrackInput::CatalogueFile(path) => {
            crate::services::audio::replay_gain_db(path)
        }
        TrackInput::Stream(_) => None,
    }
}

fn apply_pending_pause(
    sink: &mut Option<TrackSink>,
    fading: &mut Option<FadingOut>,
    pending_pause: &mut Option<u64>,
    generation: u64,
) {
    if pending_pause.take() == Some(generation) {
        let mut paused_any = false;
        if let Some(t) = sink {
            t.sink.pause();
            paused_any = true;
        }
        if let Some(out) = fading {
            out.track.sink.pause();
            paused_any = true;
        }
        if paused_any {
            PAUSED.store(true, Ordering::Relaxed);
        }
    }
}

struct TrackSink {
    sink: Sink,
    fade: FadeControl,
    sample_rate: u32,
    channels: u16,
    position_offset: Duration,
    _session: Option<Arc<StreamSession>>,
}

impl TrackSink {
    fn open(
        handle: &OutputStreamHandle,
        input: TrackInput,
        fade_secs: f64,
        position_offset: Duration,
    ) -> Result<(Self, Option<f64>), String> {
        let replay_gain = track_replay_gain(&input);
        let (source, total, session): DecodedStream = match input {
            TrackInput::File(path) | TrackInput::CatalogueFile(path) => {
                let source = crate::services::audio::FileSource::open(&path)?;
                let total = source.total_duration().map(|d| d.as_secs_f64());
                (Box::new(source), total, None)
            }
            TrackInput::Stream(session) => {
                let source = StreamSource::new(&session);
                let total = source.total_duration().map(|d| d.as_secs_f64());
                (Box::new(source), total, Some(session))
            }
        };
        let sample_rate = source.sample_rate().max(1);
        let channels = source.channels().max(1);
        let fade_secs = if fade_secs.is_finite() {
            fade_secs
        } else {
            0.0
        };
        let control = FadeControl::settled(if fade_secs > 0.0 { 0.0 } else { 1.0 });
        if fade_secs > 0.0 {
            control.ramp_to(1.0, fade_secs, sample_rate, channels);
        }
        let sink = Sink::try_new(handle).map_err(|e| format!("sink: {e}"))?;
        sink.append(FadeSource::new(
            EqualizerSource::new(source).amplify(replay_gain_linear(replay_gain.unwrap_or(0.0))),
            control.clone(),
        ));
        sink.set_speed(playback_speed());
        Ok((
            Self {
                sink,
                fade: control,
                sample_rate,
                channels,
                position_offset,
                _session: session,
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
    TrackSink::open(
        handle,
        TrackInput::File(PathBuf::from(path)),
        fade_secs,
        Duration::ZERO,
    )
}

fn open_input(
    handle: Option<&OutputStreamHandle>,
    input: TrackInput,
    fade_secs: f64,
    position_offset: Duration,
) -> Result<(TrackSink, Option<f64>), String> {
    let handle = handle.ok_or_else(|| "No audio output device".to_string())?;
    TrackSink::open(handle, input, fade_secs, position_offset)
}

fn seek_sink(sink: &Sink, time: f64) -> Result<(), String> {
    if !valid_seek_time(time) {
        return Err("invalid seek time".into());
    }
    sink.try_seek(Duration::from_secs_f64(time))
        .map_err(|e| format!("seek: {e}"))
}

struct PlayStreamRequest {
    path: String,
    input: TrackInput,
    start: f64,
    generation: u64,
    preserve_pause: bool,
}

enum PlayerCmd {
    Play(String, Option<f64>, u64),
    PlayStream(PlayStreamRequest),
    FinishDeviceChange {
        token: u64,
        generation: u64,
        path: String,
        position: f64,
        prepared: Option<PreparedAudio>,
    },
    Crossfade(String, u64, f64),
    CrossfadeStream {
        path: String,
        input: TrackInput,
        start: f64,
        generation: u64,
        fade_secs: f64,
    },
    Stop(u64),
    Pause(u64),
    Resume(u64),
    Seek(f64, u64),
    Volume(u32),
    Speed(f32),
    ReplayGain(bool),
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
    let thread_tx = cmd_tx.clone();

    std::thread::spawn(move || player_thread(cmd_rx, thread_tx));

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
    let name = device.name().ok()?;
    let config = device.default_output_config().ok();
    match config {
        Some(config) => Some(format!(
            "{name}:{}/{}",
            config.sample_format(),
            config.channels()
        )),
        None => Some(name),
    }
}

const STREAM_OFFSET_CAP_SECS: f64 = 86_400.0 * 3650.0;
const MAX_CROSSFADE_SECS: f64 = 20.0;

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
    input: TrackInput,
    start: f64,
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
    let fade_start = match sink.as_ref().map(|t| &t.sink) {
        Some(s) if !s.empty() => s.get_pos(),
        _ => return Err("nothing playing".into()),
    };

    let fade_secs = fade_secs.min(MAX_CROSSFADE_SECS);
    let stream = input.is_stream();
    let seek_file = !stream && start > 0.0;
    let offset = if stream {
        Duration::from_secs_f64(start.clamp(0.0, STREAM_OFFSET_CAP_SECS))
    } else {
        Duration::ZERO
    };
    let (incoming, total) = open_input(Some(handle), input, fade_secs, offset)?;
    if seek_file && let Err(e) = seek_sink(&incoming.sink, start) {
        incoming.sink.stop();
        return Err(e);
    }
    let volume = scale_volume(VOLUME.load(Ordering::Relaxed));
    incoming.sink.set_volume(volume);
    let outgoing = sink.replace(incoming).ok_or("nothing playing")?;
    outgoing.sink.set_volume(volume);
    outgoing
        .fade
        .ramp_to(0.0, fade_secs, outgoing.sample_rate, outgoing.channels);
    stop_fading_out(fading);
    *fading = Some(FadingOut {
        track: outgoing,
        until: fade_start + Duration::from_secs_f64(fade_secs),
    });

    Ok(total)
}

fn install_source(
    handle: &Option<OutputStreamHandle>,
    sink: &mut Option<TrackSink>,
    fading: &mut Option<FadingOut>,
    input: TrackInput,
    start: f64,
    preserve_pause: bool,
) -> Result<Option<f64>, String> {
    let handle = handle
        .as_ref()
        .ok_or_else(|| "No audio output device".to_string())?;
    let paused_before = PAUSED.load(Ordering::Relaxed);
    let stream = input.is_stream();
    let seek_file = !stream && start > 0.0;
    let offset = if stream {
        Duration::from_secs_f64(start.clamp(0.0, STREAM_OFFSET_CAP_SECS))
    } else {
        Duration::ZERO
    };
    let (track, total) = TrackSink::open(handle, input, 0.0, offset)?;
    track
        .sink
        .set_volume(scale_volume(VOLUME.load(Ordering::Relaxed)));
    if seek_file && let Err(e) = seek_sink(&track.sink, start) {
        track.sink.stop();
        return Err(e);
    }
    stop_fading_out(fading);
    if let Some(old) = sink.take() {
        old.sink.stop();
    }
    if preserve_pause && paused_before {
        track.sink.pause();
        PAUSED.store(true, Ordering::Relaxed);
    } else {
        PAUSED.store(false, Ordering::Relaxed);
    }
    *sink = Some(track);
    Ok(total)
}

fn stop_playback_for_device_change(
    output: &mut Option<OutputStream>,
    handle: &mut Option<OutputStreamHandle>,
    sink: &mut Option<TrackSink>,
    fading: &mut Option<FadingOut>,
) -> bool {
    if let Some(t) = sink.take() {
        t.sink.stop();
    }
    stop_fading_out(fading);
    drop(handle.take());
    drop(output.take());
    match OutputStream::try_default() {
        Ok((new_output, new_handle)) => {
            *output = Some(new_output);
            *handle = Some(new_handle);
            true
        }
        Err(e) => {
            log::warn!("device change: reopen output failed: {e}");
            false
        }
    }
}

fn finish_device_change(
    handle: &Option<OutputStreamHandle>,
    sink: &mut Option<TrackSink>,
    fading: &mut Option<FadingOut>,
    path: &str,
    position: f64,
    prepared: Option<PreparedAudio>,
) -> Result<Option<f64>, String> {
    let input = match prepared {
        Some(PreparedAudio::Stream(session)) => TrackInput::Stream(session),
        Some(PreparedAudio::File(file)) => TrackInput::CatalogueFile(file),
        None => {
            if path.starts_with("ytm:") {
                return Err("stream could not be re-resolved".into());
            }
            TrackInput::File(PathBuf::from(path))
        }
    };
    install_source(handle, sink, fading, input, position, true)
}

fn player_thread(rx: mpsc::Receiver<PlayerRequest>, cmd_tx: mpsc::Sender<PlayerRequest>) {
    let (out, h) = match OutputStream::try_default() {
        Ok(v) => v,
        Err(e) => {
            log::warn!("audio output unavailable: {e}");
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
    let mut pending_pause: Option<u64> = None;
    let mut seek_floor: Option<Duration> = None;
    let mut duration_secs: Option<f64> = None;
    let mut last_tick_key: Option<TickKey> = None;
    let mut last_device_poll = Instant::now();
    let mut device_handoff_token = 0u64;
    let mut pending_resume: Option<(String, f64)> = None;

    loop {
        match rx.recv_timeout(TICK_INTERVAL) {
            Ok(PlayerRequest { cmd, reply_tx }) => {
                let resp = match cmd {
                    PlayerCmd::Play(path, seek_to, generation) => {
                        if generation < playback_generation {
                            PlayerResp::Ok
                        } else {
                            playback_generation = generation;
                            pending_resume = None;
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
                                    apply_pending_pause(
                                        &mut sink,
                                        &mut fading,
                                        &mut pending_pause,
                                        generation,
                                    );
                                    PlayerResp::Ok
                                }
                                Err(e) => {
                                    current_path = None;
                                    duration_secs = None;
                                    seek_floor = None;
                                    PlayerResp::Err(e)
                                }
                            }
                        }
                    }
                    PlayerCmd::PlayStream(request) => {
                        if request.generation < playback_generation {
                            PlayerResp::Ok
                        } else {
                            pending_resume = None;
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
                            let queued_seek = pending_seek
                                .take()
                                .filter(|(_, queued_generation)| {
                                    *queued_generation == request.generation
                                })
                                .map(|(time, _)| time);
                            let start = queued_seek.unwrap_or(request.start);
                            match install_source(
                                &handle,
                                &mut sink,
                                &mut fading,
                                request.input,
                                start,
                                request.preserve_pause,
                            ) {
                                Ok(total) => {
                                    playback_generation = request.generation;
                                    active_generation = request.generation;
                                    seek_floor = seek_floor_for(start);
                                    current_path = Some(request.path);
                                    duration_secs = total;
                                    apply_pending_pause(
                                        &mut sink,
                                        &mut fading,
                                        &mut pending_pause,
                                        request.generation,
                                    );
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
                            TrackInput::File(PathBuf::from(&path)),
                            0.0,
                            generation,
                            fade_secs,
                        ) {
                            Ok(total) => {
                                playback_generation = generation;
                                active_generation = generation;
                                pending_seek = None;
                                pending_resume = None;
                                seek_floor = None;
                                current_path = Some(path);
                                duration_secs = total;
                                PAUSED.store(false, Ordering::Relaxed);
                                apply_pending_pause(
                                    &mut sink,
                                    &mut fading,
                                    &mut pending_pause,
                                    generation,
                                );
                                PlayerResp::Ok
                            }
                            Err(e) => PlayerResp::Err(e),
                        }
                    }
                    PlayerCmd::CrossfadeStream {
                        path,
                        input,
                        start,
                        generation,
                        fade_secs,
                    } => {
                        if generation <= playback_generation {
                            let _ = reply_tx.send(PlayerResp::Err("stale generation".to_string()));
                            continue;
                        }
                        match begin_crossfade(
                            &handle,
                            &mut sink,
                            &mut fading,
                            input,
                            start,
                            generation,
                            fade_secs,
                        ) {
                            Ok(total) => {
                                playback_generation = generation;
                                active_generation = generation;
                                pending_seek = None;
                                pending_resume = None;
                                seek_floor = seek_floor_for(start);
                                current_path = Some(path);
                                duration_secs = total;
                                PAUSED.store(false, Ordering::Relaxed);
                                apply_pending_pause(
                                    &mut sink,
                                    &mut fading,
                                    &mut pending_pause,
                                    generation,
                                );
                                PlayerResp::Ok
                            }
                            Err(e) => PlayerResp::Err(e),
                        }
                    }
                    PlayerCmd::FinishDeviceChange {
                        token,
                        generation,
                        path,
                        position,
                        prepared,
                    } => {
                        if token != device_handoff_token
                            || generation != playback_generation
                            || pending_resume.is_none()
                        {
                            PlayerResp::Ok
                        } else {
                            if pending_seek.as_ref().is_some_and(|(_, queued_generation)| {
                                *queued_generation != generation
                            }) {
                                pending_seek = None;
                            }
                            let queued_seek = pending_seek
                                .take()
                                .filter(|(_, queued_generation)| *queued_generation == generation)
                                .map(|(time, _)| time);
                            let position = queued_seek.unwrap_or(position);
                            let result = finish_device_change(
                                &handle,
                                &mut sink,
                                &mut fading,
                                &path,
                                position,
                                prepared,
                            );
                            pending_resume = None;
                            match result {
                                Ok(total) => {
                                    duration_secs = total;
                                    seek_floor = seek_floor_for(position);
                                    PlayerResp::Ok
                                }
                                Err(e) => {
                                    log::warn!("device change: could not resume {path}: {e}");
                                    emit_stream_error(&path, &e, generation);
                                    seek_floor = None;
                                    if sink.is_none() {
                                        PAUSED.store(false, Ordering::Relaxed);
                                    }
                                    PlayerResp::Ok
                                }
                            }
                        }
                    }
                    PlayerCmd::Stop(generation) => {
                        if generation < playback_generation {
                            PlayerResp::Ok
                        } else {
                            playback_generation = generation.saturating_add(1);
                            active_generation = 0;
                            pending_seek = None;
                            pending_resume = None;
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
                            if generation == playback_generation && active_generation == 0 {
                                pending_pause = Some(generation);
                            }
                            PlayerResp::Ok
                        } else if generation == playback_generation
                            && sink.is_none()
                            && pending_resume.is_some()
                        {
                            PAUSED.store(true, Ordering::Relaxed);
                            PlayerResp::Ok
                        } else {
                            if let Some(t) = &sink {
                                t.sink.pause();
                            }
                            if let Some(out) = &fading {
                                out.track.sink.pause();
                            }
                            if sink.as_ref().is_some_and(|track| !track.sink.empty())
                                || fading.as_ref().is_some_and(|out| !out.track.sink.empty())
                            {
                                PAUSED.store(true, Ordering::Relaxed);
                            }
                            PlayerResp::Ok
                        }
                    }
                    PlayerCmd::Resume(generation) => {
                        if generation < playback_generation || generation != active_generation {
                            PlayerResp::Bool(false)
                        } else {
                            let resumed_tail = if let Some(out) = &fading {
                                out.track.sink.play();
                                true
                            } else {
                                false
                            };
                            if let Some(t) = sink.as_ref().filter(|t| !t.sink.empty()) {
                                t.sink.play();
                                PAUSED.store(false, Ordering::Relaxed);
                                PlayerResp::Bool(true)
                            } else {
                                if resumed_tail {
                                    PAUSED.store(false, Ordering::Relaxed);
                                }
                                PlayerResp::Bool(resumed_tail)
                            }
                        }
                    }
                    PlayerCmd::Seek(time, generation) => {
                        if !valid_seek_time(time) {
                            PlayerResp::Err("invalid seek time".into())
                        } else if generation < playback_generation {
                            PlayerResp::Ok
                        } else {
                            playback_generation = generation;
                            let active = generation == active_generation && sink.is_some();
                            let ended =
                                active && sink.as_ref().is_some_and(|track| track.sink.empty());
                            if active && !ended {
                                if let Some(track) = sink.as_ref() {
                                    match seek_sink(&track.sink, time) {
                                        Ok(()) => {
                                            seek_floor = seek_floor_for(time);
                                            PlayerResp::Ok
                                        }
                                        Err(e) => PlayerResp::Err(e),
                                    }
                                } else {
                                    PlayerResp::Err("no active sink".into())
                                }
                            } else if ended {
                                match current_path.clone() {
                                    Some(path) if !path.starts_with("ytm:") => {
                                        match install_source(
                                            &handle,
                                            &mut sink,
                                            &mut fading,
                                            TrackInput::File(PathBuf::from(&path)),
                                            time,
                                            true,
                                        ) {
                                            Ok(total) => {
                                                duration_secs = total;
                                                seek_floor = seek_floor_for(time);
                                                PlayerResp::Ok
                                            }
                                            Err(e) => PlayerResp::Err(e),
                                        }
                                    }
                                    Some(path) if path.starts_with("ytm:") => {
                                        pending_seek = None;
                                        PlayerResp::Err(
                                            "seek on an ended catalogue track is not supported — play it again".into(),
                                        )
                                    }
                                    _ => {
                                        pending_seek = Some((time, generation));
                                        PlayerResp::Ok
                                    }
                                }
                            } else {
                                pending_seek = Some((time, generation));
                                PlayerResp::Ok
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
                    PlayerCmd::Speed(speed) => {
                        let speed = clamp_speed(speed);
                        SPEED.store(speed.to_bits(), Ordering::Relaxed);
                        if let Some(t) = &sink {
                            t.sink.set_speed(speed);
                        }
                        if let Some(out) = &fading {
                            out.track.sink.set_speed(speed);
                        }
                        PlayerResp::Ok
                    }
                    PlayerCmd::ReplayGain(enabled) => {
                        REPLAY_GAIN.store(enabled, Ordering::Relaxed);
                        PlayerResp::Ok
                    }
                    PlayerCmd::GetPosition(generation) => {
                        let pos = if generation == active_generation {
                            reported_pos(sink.as_ref(), seek_floor)
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
                    let changed = output_device_name.as_deref() != now.as_deref();
                    if changed {
                        output_device_name = now;
                        device_handoff_token += 1;
                        let token = device_handoff_token;
                        let resume_pos = sink
                            .as_ref()
                            .filter(|t| !t.sink.empty())
                            .map(|t| reported_pos(Some(t), seek_floor))
                            .or_else(|| pending_resume.as_ref().map(|(_, pos)| *pos));
                        let reopened = stop_playback_for_device_change(
                            &mut output,
                            &mut handle,
                            &mut sink,
                            &mut fading,
                        );
                        let generation = playback_generation;
                        let candidate =
                            resume_pos.and_then(|pos| current_path.clone().map(|path| (path, pos)));
                        match candidate {
                            Some((path, pos)) if reopened => {
                                if path.starts_with("ytm:") {
                                    pending_resume = Some((path.clone(), pos));
                                    let handoff_tx = cmd_tx.clone();
                                    std::thread::spawn(move || {
                                        let prepared = path
                                            .strip_prefix("ytm:")
                                            .and_then(|video_id| {
                                                match crate::services::catalogue::prepare_blocking(
                                                    video_id, pos, None,
                                                ) {
                                                    Ok(prepared) => Some(prepared.audio),
                                                    Err(error) => {
                                                        log::error!(
                                                            "device change: could not re-resolve {path}: {error}"
                                                        );
                                                        None
                                                    }
                                                }
                                            });
                                        let (reply_tx, _) = mpsc::channel::<PlayerResp>();
                                        let _ = handoff_tx.send(PlayerRequest {
                                            cmd: PlayerCmd::FinishDeviceChange {
                                                token,
                                                generation,
                                                path,
                                                position: pos,
                                                prepared,
                                            },
                                            reply_tx,
                                        });
                                    });
                                } else {
                                    pending_resume = None;
                                    match finish_device_change(
                                        &handle,
                                        &mut sink,
                                        &mut fading,
                                        &path,
                                        pos,
                                        None,
                                    ) {
                                        Ok(total) => {
                                            duration_secs = total;
                                            seek_floor = seek_floor_for(pos);
                                        }
                                        Err(e) => {
                                            log::warn!(
                                                "device change: could not resume {path}: {e}"
                                            );
                                            emit_stream_error(&path, &e, generation);
                                            seek_floor = None;
                                            if sink.is_none() {
                                                PAUSED.store(false, Ordering::Relaxed);
                                            }
                                        }
                                    }
                                }
                            }
                            Some((path, _)) => {
                                emit_stream_error(
                                    &path,
                                    "the audio output disappeared",
                                    generation,
                                );
                                pending_resume = None;
                                seek_floor = None;
                                PAUSED.store(false, Ordering::Relaxed);
                            }
                            None => {
                                pending_resume = None;
                            }
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if fading
            .as_ref()
            .is_some_and(|out| out.track.sink.empty() || out.track.sink.get_pos() >= out.until)
        {
            stop_fading_out(&mut fading);
        }
        emit_playback_tick(
            active_generation,
            sink.as_ref(),
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

pub fn play_stream(
    path: String,
    session: Arc<StreamSession>,
    start: f64,
    generation: u64,
) -> Result<(), String> {
    to_result(send(PlayerCmd::PlayStream(PlayStreamRequest {
        path,
        input: TrackInput::Stream(session),
        start,
        generation,
        preserve_pause: false,
    })))
}

pub fn play_cached_stream(
    path: String,
    file: String,
    start: Option<f64>,
    generation: u64,
) -> Result<(), String> {
    to_result(send(PlayerCmd::PlayStream(PlayStreamRequest {
        path,
        input: TrackInput::CatalogueFile(PathBuf::from(file)),
        start: start.unwrap_or(0.0),
        generation,
        preserve_pause: false,
    })))
}

pub fn seek_stream_source(
    path: String,
    session: Arc<StreamSession>,
    start: f64,
    generation: u64,
    playing: bool,
) -> Result<(), String> {
    to_result(send(PlayerCmd::PlayStream(PlayStreamRequest {
        path,
        input: TrackInput::Stream(session),
        start,
        generation,
        preserve_pause: !playing,
    })))
}

pub fn seek_cached_stream_source(
    path: String,
    file: String,
    start: f64,
    generation: u64,
    playing: bool,
) -> Result<(), String> {
    to_result(send(PlayerCmd::PlayStream(PlayStreamRequest {
        path,
        input: TrackInput::CatalogueFile(PathBuf::from(file)),
        start,
        generation,
        preserve_pause: !playing,
    })))
}

pub fn crossfade_stream(
    path: String,
    session: Arc<StreamSession>,
    start: f64,
    generation: u64,
    fade_secs: f64,
) -> Result<(), String> {
    to_result(send(PlayerCmd::CrossfadeStream {
        path,
        input: TrackInput::Stream(session),
        start,
        generation,
        fade_secs,
    }))
}

pub fn crossfade_cached_stream(
    path: String,
    file: String,
    start: f64,
    generation: u64,
    fade_secs: f64,
) -> Result<(), String> {
    to_result(send(PlayerCmd::CrossfadeStream {
        path,
        input: TrackInput::CatalogueFile(PathBuf::from(file)),
        start,
        generation,
        fade_secs,
    }))
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

pub fn set_speed(speed: f32) {
    let _ = send(PlayerCmd::Speed(speed));
}

pub fn set_replay_gain(enabled: bool) {
    let _ = send(PlayerCmd::ReplayGain(enabled));
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
