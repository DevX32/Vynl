use mp3lame_encoder::{
    Bitrate, Builder as Mp3EncoderBuilder, EncodeError, Encoder as Mp3Encoder, FlushNoGap,
    InterleavedPcm, MonoPcm, Quality, VbrMode,
};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::codecs::registry::CodecRegistry;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;

use std::time::Duration;

pub(crate) fn codec_registry() -> &'static CodecRegistry {
    static REGISTRY: std::sync::OnceLock<CodecRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = CodecRegistry::new();
        symphonia::default::register_enabled_codecs(&mut registry);
        registry.register_audio_decoder::<symphonia_adapter_libopus::OpusDecoder>();
        registry
    })
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<i64>,
    pub track_number: Option<i64>,
    pub genre: Option<String>,
    pub comment: Option<String>,
}
pub struct FileSource {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    channels: u16,
    sample_rate: u32,
    duration: Option<Duration>,
    samples: Vec<f32>,
    cursor: usize,
}

impl FileSource {
    pub fn open(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("open: {e}"))?;
        let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
        let source = MediaSourceStream::new(Box::new(file), Default::default());
        let format_reader = symphonia::default::get_probe()
            .probe(
                Hint::new().with_extension(ext),
                source,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|e| format!("decode: {e}"))?;
        let track = format_reader
            .default_track(TrackType::Audio)
            .cloned()
            .ok_or_else(|| "this track has no audio".to_string())?;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .cloned()
            .ok_or_else(|| "this track has no audio parameters".to_string())?;
        let decoder = codec_registry()
            .make_audio_decoder(&params, &AudioDecoderOptions::default())
            .map_err(|e| format!("decode: {e}"))?;
        let duration = match (track.num_frames, params.sample_rate) {
            (Some(frames), Some(rate)) if rate > 0 => {
                Some(Duration::from_secs_f64(frames as f64 / rate as f64))
            }
            _ => None,
        };
        let sample_rate = params.sample_rate.unwrap_or(44100).max(1);
        let channels = params
            .channels
            .as_ref()
            .map(|channels| channels.count() as u16)
            .unwrap_or(2)
            .max(1);
        Ok(Self {
            format_reader,
            decoder,
            track_id: track.id,
            channels,
            sample_rate,
            duration,
            samples: Vec::new(),
            cursor: 0,
        })
    }
}

impl Iterator for FileSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        loop {
            if self.cursor < self.samples.len() {
                let sample = self.samples[self.cursor];
                self.cursor += 1;
                return Some(sample);
            }
            self.samples.clear();
            self.cursor = 0;
            let packet = match self.format_reader.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => return None,
                Err(error) => {
                    eprintln!("[Vynl] audio read error: {error}");
                    return None;
                }
            };
            if packet.track_id != self.track_id {
                continue;
            }
            match self.decoder.decode(&packet) {
                Ok(audio) => {
                    let samples = audio.samples_interleaved();
                    let mut buffer = vec![0.0f32; samples];
                    audio.copy_to_slice_interleaved(&mut buffer);
                    self.samples.extend_from_slice(&buffer);
                }
                Err(error) => {
                    eprintln!("[Vynl] audio decode error: {error}");
                }
            }
        }
    }
}

impl rodio::Source for FileSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.duration
    }

    fn try_seek(&mut self, to: Duration) -> Result<(), rodio::source::SeekError> {
        let to = match self.duration {
            Some(total) if total > Duration::ZERO => {
                let margin = (total / 4).min(Duration::from_millis(250));
                to.min(total.saturating_sub(margin))
            }
            _ => to,
        };
        let time = Time::try_from_secs_f64(to.as_secs_f64()).ok_or(
            rodio::source::SeekError::NotSupported {
                underlying_source: "Vynl file",
            },
        )?;
        self.format_reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time,
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|_| rodio::source::SeekError::NotSupported {
                underlying_source: "Vynl file",
            })?;
        self.decoder.reset();
        self.samples.clear();
        self.cursor = 0;
        Ok(())
    }
}

pub fn duration_secs(path: &Path) -> Option<f64> {
    use lofty::prelude::*;
    use lofty::probe::Probe;

    let tagged = Probe::open(path).ok()?.read().ok()?;
    let secs = tagged.properties().duration().as_secs_f64();
    (secs.is_finite() && secs > 0.0).then(|| secs.round())
}

pub fn read_tags(path: &Path) -> TrackTags {
    use lofty::prelude::*;
    use lofty::probe::Probe;
    use lofty::tag::ItemKey;

    let Ok(tagged) = Probe::open(path).and_then(|p| p.read()) else {
        return TrackTags::default();
    };
    let tag = tagged.primary_tag().or_else(|| tagged.tags().first());

    let get = |key: ItemKey| -> Option<String> {
        tag.and_then(|t| t.get_string(key)).map(|s| s.to_string())
    };
    let parse =
        |value: Option<String>| -> Option<i64> { value.and_then(|v| v.trim().parse::<i64>().ok()) };

    TrackTags {
        title: get(ItemKey::TrackTitle),
        artist: get(ItemKey::TrackArtist),
        album: get(ItemKey::AlbumTitle),
        year: parse(get(ItemKey::Year)),
        track_number: parse(get(ItemKey::TrackNumber)),
        genre: get(ItemKey::Genre),
        comment: get(ItemKey::Comment),
    }
}

pub fn write_tags(path: &Path, tags: &TrackTags) -> Result<(), String> {
    use lofty::prelude::*;
    use lofty::probe::Probe;
    use lofty::tag::{ItemKey, Tag, TagType};

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut tagged = Probe::open(path)
        .and_then(|p| p.read())
        .map_err(|e| format!("lofty probe failed: {e}"))?;

    let tag_type = match tagged.primary_tag().map(|t| t.tag_type()) {
        Some(t) => t,
        None => match ext.as_str() {
            "mp3" => TagType::Id3v2,
            "flac" | "ogg" | "oga" | "opus" => TagType::VorbisComments,
            "m4a" | "mp4" | "m4b" | "aac" => TagType::Mp4Ilst,
            _ => return Err("this format cannot store tags.".into()),
        },
    };
    if tagged.tag(tag_type).is_none() {
        tagged.insert_tag(Tag::new(tag_type));
    }
    let tag = tagged
        .tag_mut(tag_type)
        .ok_or_else(|| "could not access tag.".to_string())?;

    let set_or_remove = |tag: &mut Tag, key: ItemKey, value: Option<&str>| {
        if let Some(v) = value
            && !v.trim().is_empty()
        {
            tag.insert_text(key, v.trim().to_string());
            return;
        }
        tag.remove_key(key);
    };

    set_or_remove(tag, ItemKey::TrackTitle, tags.title.as_deref());
    set_or_remove(tag, ItemKey::TrackArtist, tags.artist.as_deref());
    set_or_remove(tag, ItemKey::AlbumTitle, tags.album.as_deref());
    set_or_remove(tag, ItemKey::Genre, tags.genre.as_deref());
    set_or_remove(tag, ItemKey::Comment, tags.comment.as_deref());

    if let Some(year) = tags.year {
        tag.insert_text(ItemKey::Year, year.to_string());
    } else {
        tag.remove_key(ItemKey::Year);
    }
    if let Some(number) = tags.track_number {
        tag.insert_text(ItemKey::TrackNumber, number.to_string());
    } else {
        tag.remove_key(ItemKey::TrackNumber);
    }

    let staged = staged_path(path);
    tagged
        .save_to_path(&staged, lofty::config::WriteOptions::new().use_id3v23(true))
        .map_err(|e| format!("lofty save failed: {e}"))?;
    replace_file(&staged, path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn replay_gain_db(path: &Path) -> Option<f32> {
    use lofty::prelude::*;
    use lofty::probe::Probe;
    use lofty::tag::ItemKey;

    let tagged = Probe::open(path).ok()?.read().ok()?;
    tagged.tags().iter().find_map(|tag| {
        tag.get_string(ItemKey::ReplayGainTrackGain)
            .and_then(parse_replay_gain)
    })
}

fn parse_replay_gain(value: &str) -> Option<f32> {
    let number = value.split_whitespace().next()?;
    let db = number.parse::<f32>().ok()?;
    db.is_finite().then_some(db)
}

pub fn extract_cover(file: &Path, dest: &Path) -> bool {
    use lofty::prelude::*;
    use lofty::probe::Probe;

    let tagged = match Probe::open(file).and_then(|p| p.read()) {
        Ok(tf) => tf,
        Err(_) => return false,
    };
    let pic = tagged
        .tags()
        .iter()
        .find_map(|t| t.pictures().first().cloned());
    let Some(pic) = pic else { return false };

    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(dest, pic.data()).is_ok()
        && dest.metadata().map(|m| m.len() > 0).unwrap_or(false)
}

pub fn embed_lyrics(file: &Path, text: &str) -> Result<(), String> {
    use lofty::prelude::*;
    use lofty::probe::Probe;
    use lofty::tag::{ItemKey, Tag, TagType};

    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "wav" {
        return Err("wav cannot store lyrics tags.".into());
    }

    let mut tagged = Probe::open(file)
        .and_then(|p| p.read())
        .map_err(|e| format!("lofty probe failed: {e}"))?;

    let tag_type = match tagged.primary_tag().map(|t| t.tag_type()) {
        Some(t) => t,
        None => match ext.as_str() {
            "mp3" => TagType::Id3v2,
            "flac" | "ogg" | "oga" | "opus" => TagType::VorbisComments,
            "m4a" | "mp4" | "m4b" | "aac" => TagType::Mp4Ilst,
            _ => return Err("this format cannot store lyrics tags.".into()),
        },
    };
    if tagged.tag(tag_type).is_none() {
        tagged.insert_tag(Tag::new(tag_type));
    }
    let tag = tagged
        .tag_mut(tag_type)
        .ok_or_else(|| "could not create the lyrics tag.".to_string())?;

    tag.insert_text(ItemKey::Lyrics, text.to_string());
    tag.insert_text(ItemKey::UnsyncLyrics, text.to_string());
    static LYRICS_SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = LYRICS_SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = file.with_file_name(format!(
        ".vynl-lyrics-{}-{}-{}.tmp",
        std::process::id(),
        serial,
        crate::services::util::now_ms()
    ));
    let result = tagged
        .save_to_path(&tmp, lofty::config::WriteOptions::new().use_id3v23(true))
        .map_err(|e| format!("lofty save failed: {e}"));
    match result {
        Ok(()) => std::fs::rename(&tmp, file).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("could not replace the audio file: {e}")
        }),
        Err(error) => {
            let _ = std::fs::remove_file(&tmp);
            Err(error)
        }
    }
}

pub fn transcode_to_mp3(
    src: &Path,
    dest: &Path,
    kbps: Option<u32>,
    cancelled: Option<&AtomicBool>,
) -> Result<(), String> {
    let file = File::open(src).map_err(|e| format!("Failed to open {}: {e}", src.display()))?;
    let mut reader = symphonia::default::get_probe()
        .probe(
            &Hint::new(),
            MediaSourceStream::new(Box::new(file), Default::default()),
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| format!("Could not read {}: {e}", src.display()))?;

    let track = reader
        .default_track(TrackType::Audio)
        .cloned()
        .ok_or_else(|| "Downloaded file has no audio track.".to_string())?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|c| c.audio())
        .ok_or_else(|| "Downloaded file has no audio track.".to_string())?
        .clone();

    let mut decoder = codec_registry()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(|_| format!("Vynl cannot decode this audio format ({:?}).", params.codec))?;

    let track_id = track.id;
    let mut first_packet = None;
    while let Ok(Some(packet)) = reader.next_packet() {
        if packet.track_id == track_id {
            first_packet = Some(packet);
            break;
        }
    }

    let pending: Option<Vec<f32>>;
    let (channels, sample_rate) = match first_packet {
        Some(packet) => {
            let audio = decoder
                .decode(&packet)
                .map_err(|e| format!("Failed to decode audio: {e}"))?;
            let spec = audio.spec();
            let channels = spec.channels().count() as u16;
            let mut samples = vec![0f32; audio.samples_interleaved()];
            audio.copy_to_slice_interleaved(&mut samples);
            pending = Some(samples);
            (channels, spec.rate())
        }
        None => return Err("Downloaded file contains no audio data.".to_string()),
    };

    if channels == 0 || channels > 2 {
        return Err(format!("Unsupported channel count: {channels}"));
    }

    let mut builder = Mp3EncoderBuilder::new().ok_or("Could not start the mp3 encoder.")?;
    builder
        .set_num_channels(channels as u8)
        .map_err(|e| format!("Encoder setup failed: {e}"))?;
    builder
        .set_sample_rate(sample_rate)
        .map_err(|e| format!("Encoder setup failed: {e}"))?;
    match kbps {
        Some(kbps) => builder
            .set_brate(closest_bitrate(kbps))
            .map_err(|e| format!("Encoder setup failed: {e}"))?,
        None => {
            builder
                .set_vbr_quality(Quality::Best)
                .map_err(|e| format!("Encoder setup failed: {e}"))?;
            builder
                .set_vbr_mode(VbrMode::Mt)
                .map_err(|e| format!("Encoder setup failed: {e}"))?;
        }
    }
    let mut encoder = builder
        .build()
        .map_err(|e| format!("Encoder setup failed: {e}"))?;

    let staged = staged_path(dest);
    if let Some(parent) = staged.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut out = BufWriter::new(
        File::create(&staged).map_err(|e| format!("Failed to create {}: {e}", staged.display()))?,
    );

    let mut scratch: Vec<f32> = Vec::with_capacity(64 * 1024);
    let mut encoded: Vec<u8> = Vec::with_capacity(ENCODE_BUFFER);
    let mut decoded_packets = u64::from(pending.is_some());
    let mut skipped = 0u64;

    let mut result = match pending.as_deref() {
        Some(samples) => feed(&mut encoder, &mut out, &mut encoded, samples, channels),
        None => Ok(()),
    };

    while result.is_ok() {
        if let Some(flag) = cancelled
            && flag.load(Ordering::Relaxed)
        {
            result = Err("cancelled".to_string());
            break;
        }

        let packet = match reader.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(e) => {
                result = Err(format!("Failed to read audio: {e}"));
                break;
            }
        };
        if packet.track_id != track_id {
            continue;
        }
        let audio = match decoder.decode(&packet) {
            Ok(audio) => audio,
            Err(e) => {
                skipped += 1;
                eprintln!("skipped undecodable audio packet: {e}");
                continue;
            }
        };
        decoded_packets += 1;
        let samples = audio.samples_interleaved();
        scratch.resize(samples, 0.0);
        audio.copy_to_slice_interleaved(&mut scratch);

        result = feed(
            &mut encoder,
            &mut out,
            &mut encoded,
            &scratch[..samples],
            channels,
        );
    }

    if skipped > 0 {
        eprintln!("{skipped} audio packet(s) could not be decoded");
    }

    if result.is_ok() {
        let written = out
            .write_all(&encoded)
            .map_err(|e| format!("Failed to write audio: {e}"))
            .and_then(|_| {
                encoded.clear();
                encoder
                    .flush_to_vec::<FlushNoGap>(&mut encoded)
                    .map_err(|e| format!("Encoding failed: {e}"))
            })
            .and_then(|_| {
                out.write_all(&encoded)
                    .map_err(|e| format!("Failed to write audio: {e}"))
            });
        if let Err(e) = written {
            let _ = out.flush();
            drop(out);
            let _ = std::fs::remove_file(&staged);
            return Err(e);
        }
    }

    let flushed = out.flush();
    drop(out);

    if result.is_ok() {
        write_lame_tag(&encoder, &staged);
    }

    match result {
        Ok(()) => {
            flushed.map_err(|e| format!("Failed to write audio: {e}"))?;
            if decoded_packets == 0 {
                let _ = std::fs::remove_file(&staged);
                return Err("Downloaded file had no audio data.".into());
            }
            replace_file(&staged, dest)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&staged);
            Err(e)
        }
    }
}

const ENCODE_BUFFER: usize = 256 * 1024;

fn write_lame_tag(encoder: &Mp3Encoder, staged: &Path) {
    let expected = encoder.lame_tag_size();
    let mut tag: Vec<u8> = Vec::with_capacity(expected);
    if encoder.lame_tag_encode_to_vec(&mut tag).is_none() || tag.len() != expected {
        return;
    }

    if let Ok(mut file) = OpenOptions::new().write(true).open(staged)
        && file.seek(SeekFrom::Start(0)).is_ok()
    {
        let _ = file.write_all(&tag);
    }
}

fn feed(
    encoder: &mut Mp3Encoder,
    out: &mut BufWriter<File>,
    encoded: &mut Vec<u8>,
    samples: &[f32],
    channels: u16,
) -> Result<(), String> {
    let needed =
        mp3lame_encoder::max_required_buffer_size(samples.len() / channels as usize) + 1024;
    if encoded.capacity() - encoded.len() < needed {
        out.write_all(encoded)
            .map_err(|e| format!("Failed to write audio: {e}"))?;
        encoded.clear();
    }
    encode_pcm(encoder, samples, channels, encoded).map_err(|e| format!("Encoding failed: {e}"))
}

fn encode_pcm(
    encoder: &mut Mp3Encoder,
    samples: &[f32],
    channels: u16,
    out: &mut Vec<u8>,
) -> Result<(), EncodeError> {
    if channels == 1 {
        encoder.encode_to_vec(MonoPcm(samples), out).map(|_| ())
    } else {
        let even = samples.len() - samples.len() % 2;
        encoder
            .encode_to_vec(InterleavedPcm(&samples[..even]), out)
            .map(|_| ())
    }
}

fn closest_bitrate(kbps: u32) -> Bitrate {
    const RATES: &[(u32, Bitrate)] = &[
        (8, Bitrate::Kbps8),
        (16, Bitrate::Kbps16),
        (24, Bitrate::Kbps24),
        (32, Bitrate::Kbps32),
        (40, Bitrate::Kbps40),
        (48, Bitrate::Kbps48),
        (64, Bitrate::Kbps64),
        (80, Bitrate::Kbps80),
        (96, Bitrate::Kbps96),
        (112, Bitrate::Kbps112),
        (128, Bitrate::Kbps128),
        (160, Bitrate::Kbps160),
        (192, Bitrate::Kbps192),
        (224, Bitrate::Kbps224),
        (256, Bitrate::Kbps256),
        (320, Bitrate::Kbps320),
    ];

    RATES
        .iter()
        .min_by_key(|(k, _)| k.abs_diff(kbps))
        .map(|(_, b)| *b)
        .unwrap_or(Bitrate::Kbps128)
}

fn staged_path(dest: &Path) -> std::path::PathBuf {
    let mut name = dest
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "out.mp3".into());
    name.push_str(".part");
    dest.with_file_name(name)
}

fn replace_file(from: &Path, to: &Path) -> Result<(), String> {
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => std::fs::copy(from, to)
            .map(|_| {
                let _ = std::fs::remove_file(from);
            })
            .map_err(|e| format!("Failed to finish writing {}: {e}", to.display())),
    }
}
