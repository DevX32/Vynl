use std::fs::{File, OpenOptions};

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;
use ytmusic::AudioFormat as YtAudioFormat;

use crate::services::audio;

use super::buffer::{StreamBuffer, DEFAULT_CHANNELS, DEFAULT_RATE};
use super::cache::{abandon_cache, StreamCache};
use super::resolver::{ext_for, UrlResolver};
use super::store::{fetch_loop, RangeReader, RangeStore, SessionControl};

pub struct StreamSession {
    pub video_id: String,
    pub duration: Option<f64>,
    pub(super) buffer: Arc<StreamBuffer>,
    pub(super) control: Arc<SessionControl>,
    pub(super) store: Arc<RangeStore>,
    pub(super) cache: Arc<StreamCache>,
    pub(super) file_name: String,
    pub(super) underrun_ms: Arc<std::sync::atomic::AtomicU64>,
}

impl StreamSession {
    pub fn open(
        cache: Arc<StreamCache>,
        resolver: Arc<UrlResolver>,
        video_id: String,
        format: YtAudioFormat,
        start: f64,
        duration_hint: Option<f64>,
    ) -> Result<Arc<Self>, String> {
        let total = format
            .content_length
            .ok_or_else(|| "this stream has no known length".to_string())?;
        if total == 0 {
            return Err("this stream is empty".into());
        }
        let duration = format
            .duration
            .map(|d| d.as_secs_f64())
            .or(duration_hint)
            .filter(|d| d.is_finite() && *d > 0.0);
        let ext = ext_for(&format);
        let begun = cache.begin(&video_id, ext, total, format.itag, true)?;
        let (path, file_name, coverage) = (begun.path, begun.file_name, begun.coverage);

        let write_file = match OpenOptions::new().read(true).write(true).open(&path) {
            Ok(file) => file,
            Err(error) => {
                let _ = std::fs::remove_file(&path);
                cache.finish(&video_id, &file_name, false);
                return Err(format!("could not open the stream cache: {error}"));
            }
        };
        let read_file = match File::open(&path) {
            Ok(file) => file,
            Err(error) => {
                let _ = std::fs::remove_file(&path);
                cache.finish(&video_id, &file_name, false);
                return Err(format!("could not open the stream cache: {error}"));
            }
        };

        let control = Arc::new(SessionControl { cancelled: AtomicBool::new(false), complete: AtomicBool::new(false), fetched: AtomicU64::new(0) });
        let buffer = Arc::new(StreamBuffer::new());
        let store = Arc::new(RangeStore::new(
            total,
            control.clone(),
            coverage,
            Some((cache.clone(), video_id.clone(), file_name.clone())),
        ));

        let refresh_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let fetch_store = store.clone();
        let fetch_resolver = resolver.clone();
        let fetch_video = video_id.clone();
        let fetch_refresh_count = refresh_count.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("vynl-stream-fetch".into())
            .spawn(move || {
                let failure = fetch_store.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                    fetch_loop(
                        fetch_store,
                        fetch_resolver,
                        fetch_video,
                        format,
                        write_file,
                        fetch_refresh_count,
                    );
                }));
                if result.is_err() {
                    log::error!("[Vynl] stream fetch thread panicked");
                    failure.fail("the stream downloader crashed".into());
                }
            })
        {
            control.cancel();
            buffer.close();
            let _ = std::fs::remove_file(&path);
            cache.finish(&video_id, &file_name, false);
            return Err(format!("could not start the stream downloader: {error}"));
        }

        let decode_buffer = buffer.clone();
        let decode_control = control.clone();
        let decode_store = store.clone();
        let decode_video = video_id.clone();
        let decode_cache = cache.clone();
        let decode_name = file_name.clone();
        let decode_refresh_count = refresh_count.clone();
        let decode_panic_buffer = decode_buffer.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("vynl-stream-decode".into())
            .spawn(move || {
                let decode_control = decode_control.clone();
                let panic_control = decode_control.clone();
                let panic_buffer = decode_panic_buffer.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                    decode_loop(
                        decode_buffer,
                        decode_control,
                        decode_store,
                        read_file,
                        decode_video,
                        start,
                        ext,
                        decode_cache,
                        decode_name,
                        total,
                        decode_refresh_count,
                    );
                }));
                if result.is_err() {
                    log::error!("[Vynl] stream decode thread panicked");
                    panic_control.cancel();
                    panic_buffer.fail("the stream decoder crashed".into());
                }
            })
        {
            control.cancel();
            buffer.fail(format!("could not start the stream decoder: {error}"));
            cache.finish(&video_id, &file_name, false);
            return Err(format!("could not start the stream decoder: {error}"));
        }

        Ok(Arc::new(Self {
            video_id,
            duration,
            buffer,
            control,
            store,
            cache,
            file_name,
            underrun_ms: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }))
    }

    pub fn cancel(&self) {
        self.control.cancel();
        self.buffer.close();
    }

    pub fn buffered_secs(&self) -> f64 {
        self.buffer.buffered_secs()
    }

    pub fn is_ready(&self) -> bool {
        self.buffer.format().is_some()
    }

    pub fn is_finished(&self) -> bool {
        self.buffer.is_closed()
    }

    pub fn error(&self) -> Option<String> {
        self.buffer.error()
    }

    pub fn is_complete(&self) -> bool {
        self.store.is_complete()
    }

    pub fn recently_underrun(&self, budget_ms: u64) -> bool {
        let last = self.underrun_ms.load(std::sync::atomic::Ordering::Relaxed);
        last > 0 && crate::services::util::now_ms().saturating_sub(last as i64) < budget_ms as i64
    }
}

impl Drop for StreamSession {
    fn drop(&mut self) {
        self.cancel();
        let cache = self.cache.clone();
        let video_id = self.video_id.clone();
        let file_name = self.file_name.clone();
        let complete = self.store.is_complete();
        std::thread::spawn(move || cache.finish(&video_id, &file_name, complete));
    }
}


#[allow(clippy::too_many_arguments)]
pub(super) fn decode_loop(
    buffer: Arc<StreamBuffer>,
    control: Arc<SessionControl>,
    store: Arc<RangeStore>,
    file: File,
    video_id: String,
    start: f64,
    ext: &'static str,
    cache: Arc<StreamCache>,
    file_name: String,
    total: u64,
    refresh_count: Arc<std::sync::atomic::AtomicU32>,
) {
    let reader = RangeReader {
        store: store.clone(),
        file,
        pos: 0,
        total,
    };
    let source = MediaSourceStream::new(Box::new(reader), Default::default());
    let mut format_reader = match symphonia::default::get_probe().probe(
        Hint::new().with_extension(ext),
        source,
        FormatOptions::default(),
        MetadataOptions::default(),
    ) {
        Ok(reader) => reader,
        Err(error) => {
            buffer.fail(format!("could not read this track: {error}"));
            abandon_cache(&store, &cache, &video_id, &file_name);
            return;
        }
    };

    let track = match format_reader.default_track(TrackType::Audio).cloned() {
        Some(track) => track,
        None => {
            buffer.fail("this track has no audio".into());
            abandon_cache(&store, &cache, &video_id, &file_name);
            return;
        }
    };
    let params = match track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .cloned()
    {
        Some(params) => params,
        None => {
            buffer.fail("this track has no audio parameters".into());
            abandon_cache(&store, &cache, &video_id, &file_name);
            return;
        }
    };
    let sample_rate = params.sample_rate.unwrap_or(DEFAULT_RATE).max(1);
    let channels = params
        .channels
        .as_ref()
        .map(|channels| channels.count() as u16)
        .unwrap_or(DEFAULT_CHANNELS)
        .max(1);
    buffer.set_format(sample_rate, channels);

    let mut decoder = match audio::codec_registry()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
    {
        Ok(decoder) => decoder,
        Err(error) => {
            buffer.fail(format!("this audio format cannot be decoded: {error}"));
            abandon_cache(&store, &cache, &video_id, &file_name);
            return;
        }
    };

    let mut discard = 0usize;
    if start > 0.5
        && let Some(time) = Time::try_from_secs_f64(start)
    {
        match format_reader.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time,
                track_id: Some(track.id),
            },
        ) {
            Ok(seeked) => {
                let actual = track
                    .time_base
                    .map(|base| {
                        seeked.actual_ts.get() as f64 * base.numer.get() as f64
                            / base.denom.get() as f64
                    })
                    .unwrap_or(start);
                let skip = (start - actual).max(0.0);
                discard = (skip * sample_rate as f64 * channels as f64) as usize;
                decoder = match audio::codec_registry()
                    .make_audio_decoder(&params, &AudioDecoderOptions::default())
                {
                    Ok(decoder) => decoder,
                    Err(error) => {
                        buffer.fail(format!("this audio format cannot be decoded: {error}"));
                        abandon_cache(&store, &cache, &video_id, &file_name);
                        return;
                    }
                };
            }
            Err(error) => {
                log::error!("catalogue: seek in {video_id} failed: {error}");
                buffer.fail(format!("this stream could not be seeked: {error}"));
                abandon_cache(&store, &cache, &video_id, &file_name);
                return;
            }
        }
    }

    let mut decode_errors = 0usize;
    let mut scratch: Vec<f32> = Vec::new();
    loop {
        if control.is_cancelled() {
            break;
        }
        let packet = match format_reader.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(error) => {
                buffer.fail(format!("stream read failed: {error}"));
                abandon_cache(&store, &cache, &video_id, &file_name);
                return;
            }
        };
        if packet.track_id != track.id {
            continue;
        }
        let audio = match decoder.decode(&packet) {
            Ok(audio) => audio,
            Err(error) => {
                decode_errors += 1;
                log::error!("catalogue: skipped undecodable packet for {video_id}: {error}");
                continue;
            }
        };
        let samples = audio.samples_interleaved();
        scratch.resize(samples, 0.0);
        audio.copy_to_slice_interleaved(&mut scratch[..samples]);
        let mut slice = &scratch[..samples];
        if discard > 0 {
            let frames_needed = discard.div_ceil(channels as usize);
            let trim = (frames_needed * channels as usize).min(slice.len());
            slice = &slice[trim..];
            discard = discard.saturating_sub(trim);
        }
        if !buffer.push(slice, &control) {
            break;
        }
    }

    if decode_errors >= 10 && refresh_count.load(Ordering::Relaxed) > 0 {
        log::error!(
            "catalogue: {video_id} had {decode_errors} decode errors after a URL refresh; dropping the poisoned cache"
        );
        buffer.fail("the stream could not be decoded after a URL refresh".into());
        abandon_cache(&store, &cache, &video_id, &file_name);
    } else {
        if store.is_complete() {
            cache.mark_complete(&video_id, &file_name, total);
        }
        buffer.close();
    }
}

#[derive(Clone)]
pub enum PreparedAudio {
    Stream(Arc<StreamSession>),
    File(PathBuf),
}

#[derive(Clone)]
pub struct PreparedStream {
    pub video_id: String,
    pub duration: Option<f64>,
    pub start: f64,
    pub audio: PreparedAudio,
    pub cached: bool,
}

impl PreparedStream {
    pub fn session(&self) -> Option<&Arc<StreamSession>> {
        match &self.audio {
            PreparedAudio::Stream(session) => Some(session),
            PreparedAudio::File(_) => None,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match &self.audio {
            PreparedAudio::File(path) => Some(path),
            PreparedAudio::Stream(_) => None,
        }
    }
}

pub async fn prepare(
    cache: &Arc<StreamCache>,
    resolver: &Arc<UrlResolver>,
    video_id: &str,
    start: f64,
    duration_hint: Option<f64>,
) -> Result<PreparedStream, String> {
    let start = if start.is_finite() && start > 0.0 {
        start
    } else {
        0.0
    };

    if let Some(path) = cache.lookup_complete(video_id) {
        return Ok(PreparedStream {
            video_id: video_id.to_string(),
            duration: duration_hint,
            start,
            audio: PreparedAudio::File(path),
            cached: true,
        });
    }

    let format = resolver.resolve(video_id, None).await?;
    let session = StreamSession::open(
        cache.clone(),
        resolver.clone(),
        video_id.to_string(),
        format,
        start,
        duration_hint,
    )?;

    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(error) = session.error() {
            return Err(error);
        }
        if session.is_ready() {
            if session.buffered_secs() >= 0.25 || session.is_finished() {
                break;
            }
            let duration = session.duration.unwrap_or(0.0);
            if duration > 0.0 && start + session.buffered_secs() >= duration - 1.0 {
                break;
            }
        }
        if std::time::Instant::now() >= deadline {
            if session.is_ready() && session.buffered_secs() > 0.0 {
                break;
            }
            session.cancel();
            return Err("the track took too long to start".into());
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    Ok(PreparedStream {
        video_id: video_id.to_string(),
        duration: session.duration.or(duration_hint),
        start,
        audio: PreparedAudio::Stream(session),
        cached: false,
    })
}

pub fn prepare_blocking(
    cache: &Arc<StreamCache>,
    resolver: &Arc<UrlResolver>,
    video_id: &str,
    start: f64,
    duration_hint: Option<f64>,
) -> Result<PreparedStream, String> {
    resolver
        .runtime
        .block_on(prepare(cache, resolver, video_id, start, duration_hint))
}

