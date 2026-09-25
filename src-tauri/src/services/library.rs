use crate::commands::types::LibraryTrack;
use crate::services::process;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

const AUDIO_EXTS: &[&str] = &["mp3", "m4a", "opus", "flac", "wav", "ogg", "aac"];

static DURATION_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"Duration:\s*(\d+):(\d+):(\d+(?:\.\d+)?)").unwrap());

static KV_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"^\s*([A-Za-z_][\w]*)\s*:\s*(.+?)\s*$").unwrap());

static CONT_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"^\s*:\s*(.+?)\s*$").unwrap());

static METADATA_BLOCK_PICTURE_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"METADATA_BLOCK_PICTURE").unwrap());

static PIC_CODEC_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"Video:\s*([a-z0-9]+)").unwrap());

static SCAN_LOCK: once_cell::sync::Lazy<Mutex<()>> = once_cell::sync::Lazy::new(|| Mutex::new(()));

fn codec_ext(codec: &str) -> &str {
    match codec {
        "png" => "png",
        "webp" => "webp",
        _ => "jpg",
    }
}

fn list_audio_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    let mut visited: std::collections::HashSet<std::path::PathBuf> =
        std::collections::HashSet::new();
    while let Some(current) = stack.pop() {
        let canonical = match fs::canonicalize(&current) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if !canonical.starts_with(dir) {
            continue;
        }

        if !visited.insert(canonical) {
            continue;
        }
        let entries = match fs::read_dir(&current) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') {
                continue;
            }
            let full = entry.path();

            if full.is_symlink() {
                if let Ok(resolved) = fs::canonicalize(&full) {
                    if resolved.starts_with(dir) && resolved.is_file() {
                        if let Some(ext) = resolved.extension().and_then(|e| e.to_str()) {
                            if AUDIO_EXTS.contains(&ext.to_lowercase().as_str()) {
                                out.push(full);
                            }
                        }
                    }
                }
                continue;
            }

            if full.is_file() {
                if let Some(ext) = full.extension().and_then(|e| e.to_str()) {
                    if AUDIO_EXTS.contains(&ext.to_lowercase().as_str()) {
                        out.push(full);
                    }
                }
            } else if full.is_dir() {
                stack.push(full);
            }
        }
    }
    out
}

#[derive(Default, Clone)]
struct ProbeResult {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    date: Option<String>,
    track: Option<String>,
    duration: Option<String>,
    pic_codec: Option<String>,
    lyrics: Option<String>,
}

fn probe_via_lofty(file: &Path) -> ProbeResult {
    use lofty::prelude::*;
    use lofty::probe::Probe;
    use lofty::tag::ItemKey;
    let tagged_file = match Probe::open(file).and_then(|p| p.read()) {
        Ok(tf) => tf,
        Err(_) => return ProbeResult::default(),
    };
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());
    let mut result = ProbeResult::default();
    if let Some(t) = tag {
        result.title = t.title().map(|s| s.to_string());
        result.artist = t.artist().map(|s| s.to_string());
        if result.artist.is_none() {
            result.artist = t.get_string(ItemKey::AlbumArtist).map(|s| s.to_string());
        }
        result.album = t.album().map(|s| s.to_string());
        result.date = t
            .get_string(ItemKey::RecordingDate)
            .or_else(|| t.get_string(ItemKey::Year))
            .map(|s| s.to_string());
        result.track = t.track().map(|n| n.to_string());
        if result.track.is_none() {
            result.track = t.get_string(ItemKey::TrackNumber).map(|s| s.to_string());
        }
        let lyrics = t
            .get_string(ItemKey::Lyrics)
            .or_else(|| t.get_string(ItemKey::UnsyncLyrics))
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty());
        result.lyrics = lyrics;
        if let Some(pic) = t.pictures().first() {
            let mime = pic
                .mime_type()
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| "image/jpeg".to_string());
            let codec = if mime.contains("png") {
                "png"
            } else if mime.contains("webp") {
                "webp"
            } else {
                "jpeg"
            };
            result.pic_codec = Some(codec.to_string());
        }
    }
    let dur = tagged_file.properties().duration();
    if dur.as_secs_f64() > 0.0 {
        result.duration = Some(format!("{}", dur.as_secs_f64()));
    }
    result
}

fn extract_cover_via_lofty(file: &Path, dest: &Path) -> bool {
    use lofty::prelude::*;
    use lofty::probe::Probe;
    let tagged_file = match Probe::open(file).and_then(|p| p.read()) {
        Ok(tf) => tf,
        Err(_) => return false,
    };
    let tag = match tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
    {
        Some(t) => t,
        None => return false,
    };
    let pic = match tag.pictures().first() {
        Some(p) => p,
        None => return false,
    };
    std::fs::write(dest, pic.data()).is_ok()
        && dest.metadata().map(|m| m.len() > 0).unwrap_or(false)
}

fn probe(ffprobe: &str, file: &Path) -> ProbeResult {
    let mut command = process::hidden_std(Command::new(ffprobe));
    command
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(file)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());

    let output = match command.output() {
        Ok(o) => o,
        Err(_) => return ProbeResult::default(),
    };
    if !output.status.success() {
        return ProbeResult::default();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let j: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(_) => return ProbeResult::default(),
    };

    let tags = j["format"]["tags"].as_object().cloned().unwrap_or_default();

    let mut pic_codec = None;
    if let Some(streams) = j["streams"].as_array() {
        for s in streams {
            if s["disposition"]["attached_pic"].as_i64() == Some(1) {
                pic_codec = s["codec_name"].as_str().map(|s| s.to_string());
                break;
            }
        }
    }

    let lyrics_val = tags
        .get("lyrics")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let lyrics = lyrics_val.filter(|s| !s.trim().is_empty());

    ProbeResult {
        title: tags
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        artist: tags
            .get("artist")
            .or_else(|| tags.get("album_artist"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        album: tags
            .get("album")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        date: tags
            .get("date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        track: tags
            .get("track")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        duration: j["format"]["duration"].as_str().map(|s| s.to_string()),
        pic_codec,
        lyrics,
    }
}

fn apply_metadata_field(result: &mut ProbeResult, key: &str, val: &str) {
    match key {
        "lyrics" => {
            result.lyrics = Some(match &result.lyrics {
                Some(existing) => format!("{}\n{}", existing, val),
                None => val.to_string(),
            });
        }
        "artist" | "album_artist" => {
            if result.artist.is_none() {
                result.artist = Some(val.to_string());
            }
        }
        "title" => {
            if result.title.is_none() {
                result.title = Some(val.to_string());
            }
        }
        "album" => {
            if result.album.is_none() {
                result.album = Some(val.to_string());
            }
        }
        "date" => {
            if result.date.is_none() {
                result.date = Some(val.to_string());
            }
        }
        "track" if result.track.is_none() => {
            result.track = Some(val.to_string());
        }
        _ => {}
    }
}

fn extract_attached_pic_codec(text: &str) -> Option<String> {
    let pic_line = text
        .lines()
        .find(|l| l.contains("(attached pic)") || METADATA_BLOCK_PICTURE_RE.is_match(l))?;
    Some(
        PIC_CODEC_RE
            .captures(pic_line)
            .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
            .unwrap_or_else(|| "jpeg".to_string()),
    )
}

fn parse_ffmpeg_metadata_block(text: &str, result: &mut ProbeResult) {
    let mut in_metadata = false;

    for line in text.lines() {
        if line.contains("Metadata:") {
            in_metadata = true;
            continue;
        }
        if in_metadata {
            if let Some(caps) = CONT_RE.captures(line) {
                if let Some(ref lyrics) = result.lyrics {
                    result.lyrics = Some(format!("{}\n{}", lyrics, &caps[1]));
                }
                continue;
            }
            if let Some(caps) = KV_RE.captures(line) {
                let key = caps[1].to_lowercase();
                let val = caps[2].to_string();
                apply_metadata_field(result, &key, &val);
            }
        }
    }
}

fn probe_via_ffmpeg(ffmpeg: &str, file: &Path) -> ProbeResult {
    let mut command = process::hidden_std(Command::new(ffmpeg));
    command
        .args(["-hide_banner", "-loglevel", "info", "-i"])
        .arg(file)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    let output = match command.output() {
        Ok(o) => o,
        Err(_) => return ProbeResult::default(),
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = stderr.as_ref();
    if text.is_empty() {
        return ProbeResult::default();
    }

    let mut result = ProbeResult::default();

    if let Some(caps) = DURATION_RE.captures(text) {
        let h: f64 = caps[1].parse().unwrap_or(0.0);
        let m: f64 = caps[2].parse().unwrap_or(0.0);
        let s: f64 = caps[3].parse().unwrap_or(0.0);
        let total = h * 3600.0 + m * 60.0 + s;
        if total > 0.0 {
            result.duration = Some(format!("{total}"));
        }
    }

    parse_ffmpeg_metadata_block(text, &mut result);

    if let Some(ref lyrics) = result.lyrics {
        if lyrics.trim().is_empty() {
            result.lyrics = None;
        }
    }

    result.pic_codec = extract_attached_pic_codec(text);

    result
}

fn extract_cover(ffmpeg: &str, file: &Path, dest: &Path) -> bool {
    let mut command = process::hidden_std(Command::new(ffmpeg));
    command
        .args(["-hide_banner", "-v", "error", "-i"])
        .arg(file)
        .args(["-an", "-c:v", "copy", "-y"])
        .arg(dest)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    command
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn extract_or_cache_cover(
    ffmpeg: &str,
    file: &Path,
    hash: &str,
    pic_codec: Option<&str>,
    user_data_dir: &Path,
) -> Option<String> {
    let covers_dir = user_data_dir.join("covers");
    let cover_ext = pic_codec.map(codec_ext).unwrap_or("jpg");
    let filename = format!("{}.{}", hash, cover_ext);

    let cached = super::tools::covers_search_dirs(user_data_dir)
        .into_iter()
        .map(|dir| dir.join(&filename))
        .find(|dest| dest.exists() && dest.metadata().map(|m| m.len() > 0).unwrap_or(false));

    if let Some(path) = cached {
        return Some(path.to_string_lossy().to_string());
    }

    let dest = covers_dir.join(&filename);
    if extract_cover_via_lofty(file, &dest) {
        let has_data = dest.metadata().map(|m| m.len() > 0).unwrap_or(false);
        if has_data {
            return Some(dest.to_string_lossy().to_string());
        }
        let _ = fs::remove_file(&dest);
    }
    if extract_cover(ffmpeg, file, &dest) {
        let has_data = dest.metadata().map(|m| m.len() > 0).unwrap_or(false);
        if has_data {
            return Some(dest.to_string_lossy().to_string());
        }
        let _ = fs::remove_file(&dest);
    }

    None
}

fn epoch_ms(t: std::time::SystemTime) -> u64 {
    t.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn file_times(file: &Path) -> (u64, u64) {
    match fs::metadata(file) {
        Ok(m) => (
            m.modified().map(epoch_ms).unwrap_or(0),
            m.created().map(epoch_ms).unwrap_or(0),
        ),
        Err(_) => (0, 0),
    }
}

fn resolve_added_at(hint: Option<u64>, created_ms: u64, mtime_ms: u64) -> u64 {
    hint.filter(|v| *v > 0)
        .or_else(|| (created_ms > 0).then_some(created_ms))
        .unwrap_or(mtime_ms)
}

fn probe_file(
    file: &Path,
    ffprobe_path: Option<&str>,
    ffmpeg_path: Option<&str>,
    user_data_dir: &Path,
    added_at_hint: Option<u64>,
) -> LibraryTrack {
    let (mtime_ms, created_ms) = file_times(file);
    let added_at_ms = resolve_added_at(added_at_hint, created_ms, mtime_ms);

    let hash_input = format!("{}:{}", file.to_string_lossy(), mtime_ms);
    let hash = {
        let mut hasher = Sha256::new();
        hasher.update(hash_input.as_bytes());
        hex::encode(hasher.finalize())
    };

    let fallback_title = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let ext = file
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let fallback = LibraryTrack {
        id: hash.clone(),
        path: file.to_string_lossy().to_string(),
        title: fallback_title.clone(),
        artist: String::new(),
        album: String::new(),
        year: None,
        duration: 0.0,
        cover: None,
        track_number: None,
        lyrics: None,
        ext,
        mtime: Some(mtime_ms),
        added_at: Some(added_at_ms),
    };

    let p_lofty = probe_via_lofty(file);
    let p = if p_lofty.title.is_some()
        || p_lofty.artist.is_some()
        || p_lofty.duration.is_some()
        || p_lofty.album.is_some()
    {
        p_lofty.clone()
    } else if let Some(ffprobe) = ffprobe_path {
        let p2 = probe(ffprobe, file);
        if p2.title.is_none() && p2.duration.is_none() && p2.artist.is_none() {
            p_lofty.clone()
        } else {
            p2
        }
    } else if let Some(ffmpeg) = ffmpeg_path {
        let p2 = probe_via_ffmpeg(ffmpeg, file);
        if p2.title.is_none() && p2.duration.is_none() && p2.artist.is_none() {
            p_lofty.clone()
        } else {
            p2
        }
    } else {
        p_lofty.clone()
    };

    if p.duration.is_none() && p.title.is_none() && p.artist.is_none() {
        return fallback;
    }

    let mut cover: Option<String> = None;
    if p.pic_codec.is_some() || p_lofty.pic_codec.is_some() {
        let pic_codec = p.pic_codec.as_deref().or(p_lofty.pic_codec.as_deref());
        if let Some(ffmpeg) = ffmpeg_path {
            cover = extract_or_cache_cover(ffmpeg, file, &hash, pic_codec, user_data_dir);
        } else {
            let covers_dir = user_data_dir.join("covers");
            let cover_ext = pic_codec.map(codec_ext).unwrap_or("jpg");
            let filename = format!("{}.{}", hash, cover_ext);
            let cached = super::tools::covers_search_dirs(user_data_dir)
                .into_iter()
                .map(|dir| dir.join(&filename))
                .find(|dest| {
                    dest.exists() && dest.metadata().map(|m| m.len() > 0).unwrap_or(false)
                });
            if let Some(path) = cached {
                cover = Some(path.to_string_lossy().to_string());
            } else {
                let dest = covers_dir.join(&filename);
                if extract_cover_via_lofty(file, &dest) {
                    if dest.metadata().map(|m| m.len() > 0).unwrap_or(false) {
                        cover = Some(dest.to_string_lossy().to_string());
                    } else {
                        let _ = fs::remove_file(&dest);
                    }
                }
            }
        }
    } else if let Some(ffmpeg) = ffmpeg_path {
        cover = extract_or_cache_cover(ffmpeg, file, &hash, p.pic_codec.as_deref(), user_data_dir);
    }

    let duration = p
        .duration
        .as_deref()
        .and_then(|s| s.parse::<f64>().ok())
        .map(|d| d.round().max(0.0))
        .unwrap_or(0.0);

    let year = p
        .date
        .as_deref()
        .and_then(|d| d.get(..4).and_then(|y| y.parse::<i64>().ok()));

    let track_number = p
        .track
        .as_deref()
        .and_then(|t| t.split('/').next().and_then(|n| n.parse::<i64>().ok()));

    LibraryTrack {
        id: hash,
        path: file.to_string_lossy().to_string(),
        title: p.title.unwrap_or(fallback_title),
        artist: p.artist.unwrap_or_default(),
        album: p.album.unwrap_or_default(),
        year,
        duration,
        cover,
        track_number,
        lyrics: p.lyrics,
        ext: fallback.ext,
        mtime: Some(mtime_ms),
        added_at: Some(added_at_ms),
    }
}

fn cache_path(user_data_dir: &Path) -> PathBuf {
    user_data_dir.join("library-cache.json")
}

pub fn load_cache(user_data_dir: &Path) -> Option<Vec<LibraryTrack>> {
    let data = fs::read_to_string(cache_path(user_data_dir)).ok()?;
    let tracks: Vec<LibraryTrack> = serde_json::from_str(&data).ok()?;
    Some(tracks)
}

fn save_cache(user_data_dir: &Path, tracks: &[LibraryTrack]) {
    if let Ok(json) = serde_json::to_string(tracks) {
        let _ = fs::write(cache_path(user_data_dir), json);
    }
}

pub fn scan_library(output_dir: &Path, user_data_dir: &Path) -> Vec<LibraryTrack> {
    scan_library_incremental(output_dir, user_data_dir, None)
}

pub fn scan_cached_library(output_dir: &Path, user_data_dir: &Path) -> Vec<LibraryTrack> {
    let _scan_guard = SCAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let existing = load_cache(user_data_dir);
    scan_library_incremental_unlocked(output_dir, user_data_dir, existing.as_deref())
}

pub fn scan_library_incremental(
    output_dir: &Path,
    user_data_dir: &Path,
    existing: Option<&[LibraryTrack]>,
) -> Vec<LibraryTrack> {
    let _scan_guard = SCAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    scan_library_incremental_unlocked(output_dir, user_data_dir, existing)
}

fn scan_library_incremental_unlocked(
    output_dir: &Path,
    user_data_dir: &Path,
    existing: Option<&[LibraryTrack]>,
) -> Vec<LibraryTrack> {
    let ffmpeg_path = super::tools::get_tool_path("ffmpeg", user_data_dir);
    let ffprobe_path = super::tools::get_ffprobe_path(user_data_dir);

    let covers_dir = user_data_dir.join("covers");
    fs::create_dir_all(&covers_dir).ok();

    if !output_dir.exists() {
        return vec![];
    }

    let files = list_audio_files(output_dir);

    let existing_map: HashMap<&str, &LibraryTrack> = existing
        .unwrap_or_default()
        .iter()
        .map(|t| (t.path.as_str(), t))
        .collect();

    let mut cached_tracks: Vec<LibraryTrack> = Vec::new();
    let mut files_to_probe: Vec<(PathBuf, Option<u64>)> = Vec::new();

    for file in &files {
        let path_str = file.to_string_lossy().to_string();
        let cached = existing_map.get(path_str.as_str());

        if let Some(cached) = cached {
            let (current_mtime, current_created) = file_times(file);

            if cached.mtime == Some(current_mtime)
                && current_mtime > 0
                && !(cached.artist.is_empty() || cached.title.is_empty() || cached.cover.is_none())
            {
                let mut track = (*cached).clone();
                if track.added_at.unwrap_or(0) == 0 {
                    track.added_at = Some(resolve_added_at(None, current_created, current_mtime));
                }
                cached_tracks.push(track);
                continue;
            }
        }

        let added_at_hint = cached.and_then(|c| c.added_at);
        files_to_probe.push((file.clone(), added_at_hint));
    }

    let mut probed: Vec<LibraryTrack> = Vec::with_capacity(files_to_probe.len());
    let max_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let batch_size = max_threads.max(1);

    for chunk in files_to_probe.chunks(batch_size) {
        std::thread::scope(|s| {
            let mut handles = Vec::with_capacity(chunk.len());
            for (file, added_at_hint) in chunk {
                let fp = ffprobe_path.clone();
                let ff = ffmpeg_path.clone();
                let ud = user_data_dir.to_path_buf();
                let f = file.clone();
                let hint = *added_at_hint;
                handles
                    .push(s.spawn(move || probe_file(&f, fp.as_deref(), ff.as_deref(), &ud, hint)));
            }
            for handle in handles {
                if let Ok(track) = handle.join() {
                    probed.push(track);
                }
            }
        });
    }

    let mut out = cached_tracks;
    out.extend(probed);

    let output_dir_str = output_dir.to_string_lossy().to_string();
    for track in &mut out {
        if track.album.is_empty() {
            let parent = Path::new(&track.path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            if parent != output_dir_str {
                track.album = Path::new(&track.path)
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
            }
        }
    }

    out.sort_by(|a, b| format!("{}{}", a.artist, a.title).cmp(&format!("{}{}", b.artist, b.title)));

    save_cache(user_data_dir, &out);

    out
}
