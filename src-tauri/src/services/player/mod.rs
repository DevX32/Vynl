mod engine;
pub(crate) mod eq;
mod fade;
mod track;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::services::catalogue::stream::StreamSession;

pub use eq::{
    EQ_BAND_COUNT, EQ_FREQS, EQ_MAX_DB, EQ_MIN_DB, EqDesign, eq_design, normalize_eq_bands, set_eq,
};

pub(super) static VOLUME: AtomicU32 = AtomicU32::new(75);
pub(super) static PAUSED: AtomicBool = AtomicBool::new(false);
pub(super) static SPEED: AtomicU32 = AtomicU32::new(1.0f32.to_bits());
pub(super) static REPLAY_GAIN: AtomicBool = AtomicBool::new(false);

pub(super) const MIN_SPEED: f32 = 0.5;
pub(super) const MAX_SPEED: f32 = 2.0;
pub(super) const REPLAY_GAIN_MIN_LINEAR: f32 = 0.25;
pub(super) const REPLAY_GAIN_MAX_LINEAR: f32 = 4.0;

pub const PLAYBACK_TICK_EVENT: &str = "vynl:player:tick";
pub(super) const TICK_INTERVAL: Duration = Duration::from_millis(100);
pub(super) const DEVICE_POLL_INTERVAL: Duration = Duration::from_secs(2);
pub(super) static APP: OnceLock<AppHandle> = OnceLock::new();

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

use track::TrackSink;

pub(super) type TickKey = (u64, bool, bool, u64);

pub(super) fn reported_pos(sink: Option<&TrackSink>, floor: Option<Duration>) -> f64 {
    let pos = sink
        .map(|s| s.sink.get_pos() + s.position_offset)
        .unwrap_or_default();
    pos.max(floor.unwrap_or_default()).as_secs_f64()
}

pub(super) fn emit_playback_tick(
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

pub(super) fn emit_stream_error(path: &str, message: &str, generation: u64) {
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

pub(super) fn scale_volume(raw: u32) -> f32 {
    let v = raw as f32 / 100.0;
    (v * v).min(0.8)
}

pub(super) fn clamp_speed(speed: f32) -> f32 {
    if speed.is_finite() {
        speed.clamp(MIN_SPEED, MAX_SPEED)
    } else {
        1.0
    }
}

pub(super) fn playback_speed() -> f32 {
    f32::from_bits(SPEED.load(Ordering::Relaxed))
}

pub(super) fn replay_gain_linear(db: f32) -> f32 {
    if db.is_finite() {
        10f32
            .powf(db / 20.0)
            .clamp(REPLAY_GAIN_MIN_LINEAR, REPLAY_GAIN_MAX_LINEAR)
    } else {
        1.0
    }
}

pub(super) fn valid_seek_time(time: f64) -> bool {
    time.is_finite() && time >= 0.0 && time < Duration::MAX.as_secs_f64()
}

pub(super) const STREAM_OFFSET_CAP_SECS: f64 = 86_400.0 * 3650.0;
pub(super) const MAX_CROSSFADE_SECS: f64 = 20.0;

pub(crate) use engine::{PlayStreamRequest, PlayerCmd, PlayerResp, send, to_result};
pub(crate) use track::TrackInput;

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
