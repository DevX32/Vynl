use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use rodio::{OutputStreamHandle, Sink, Source};

use crate::services::catalogue::stream::{StreamSession, StreamSource};

use super::eq::EqualizerSource;
use super::fade::{FadeControl, FadeSource};
use super::{playback_speed, replay_gain_linear, valid_seek_time, PAUSED, REPLAY_GAIN};

pub(crate) type DecodedStream = (
    Box<dyn Source<Item = f32> + Send>,
    Option<f64>,
    Option<Arc<StreamSession>>,
);

pub(crate) enum TrackInput {
    File(PathBuf),
    CatalogueFile(PathBuf),
    Stream(Arc<StreamSession>),
}

impl TrackInput {
    pub(super) fn is_stream(&self) -> bool {
        matches!(self, Self::Stream(_))
    }
}

pub(super) fn track_replay_gain(input: &TrackInput) -> Option<f32> {
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

pub(super) fn apply_pending_pause(
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

pub(crate) struct TrackSink {
    pub(super) sink: Sink,
    pub(super) fade: FadeControl,
    pub(super) sample_rate: u32,
    pub(super) channels: u16,
    pub(super) position_offset: Duration,
    pub(super) _session: Option<Arc<StreamSession>>,
}

impl TrackSink {
    pub(super) fn open(
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

pub(super) fn open_track(
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

pub(super) fn open_input(
    handle: Option<&OutputStreamHandle>,
    input: TrackInput,
    fade_secs: f64,
    position_offset: Duration,
) -> Result<(TrackSink, Option<f64>), String> {
    let handle = handle.ok_or_else(|| "No audio output device".to_string())?;
    TrackSink::open(handle, input, fade_secs, position_offset)
}

pub(super) fn seek_sink(sink: &Sink, time: f64) -> Result<(), String> {
    if !valid_seek_time(time) {
        return Err("invalid seek time".into());
    }
    sink.try_seek(Duration::from_secs_f64(time))
        .map_err(|e| format!("seek: {e}"))
}

pub(super) struct FadingOut {
    pub(super) track: TrackSink,
    pub(super) until: Duration,
}

