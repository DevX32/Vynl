use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use rodio::{OutputStreamHandle, Sink, Source};

use crate::services::catalogue::stream::{StreamSession, StreamSource};

use super::eq::EqualizerSource;
use super::fade::{FadeControl, FadeSource};
use super::{PAUSED, REPLAY_GAIN, playback_speed, replay_gain_linear, valid_seek_time};

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

#[derive(Clone)]
pub(super) struct ReplayGainControl {
    gain: Arc<AtomicU32>,
    db: Option<f32>,
}

impl ReplayGainControl {
    fn new(enabled: bool, db: Option<f32>) -> Self {
        let control = Self {
            gain: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            db,
        };
        control.apply(enabled);
        control
    }

    pub(super) fn apply(&self, enabled: bool) {
        let linear = if enabled {
            replay_gain_linear(self.db.unwrap_or(0.0))
        } else {
            1.0
        };
        self.gain.store(linear.to_bits(), Ordering::Relaxed);
    }

    #[inline]
    fn value(&self) -> f32 {
        f32::from_bits(self.gain.load(Ordering::Relaxed))
    }
}

struct ReplayGainSource<S> {
    inner: S,
    control: ReplayGainControl,
}

impl<S> Iterator for ReplayGainSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        Some(sample * self.control.value())
    }
}

impl<S> Source for ReplayGainSource<S>
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

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.inner.try_seek(pos)
    }
}

pub(crate) struct TrackSink {
    pub(super) sink: Sink,
    pub(super) fade: FadeControl,
    pub(super) replay_gain: ReplayGainControl,
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
        let replay_db = track_replay_gain(&input);
        let replay_gain = ReplayGainControl::new(REPLAY_GAIN.load(Ordering::Relaxed), replay_db);
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
            ReplayGainSource {
                inner: EqualizerSource::new(source),
                control: replay_gain.clone(),
            },
            control.clone(),
        ));
        sink.set_speed(playback_speed());
        Ok((
            Self {
                sink,
                fade: control,
                replay_gain,
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
