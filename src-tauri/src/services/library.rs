use crate::commands::types::LibraryTrack;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const AUDIO_EXTS: &[&str] = &["mp3", "m4a", "opus", "flac", "wav", "ogg", "aac"];

static SCAN_LOCK: once_cell::sync::Lazy<Mutex<()>> = once_cell::sync::Lazy::new(|| Mutex::new(()));

fn track_hash(path: &str, mtime_ms: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{path}:{mtime_ms}").as_bytes());
    hex::encode(hasher.finalize())
}

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
                if let Ok(resolved) = fs::canonicalize(&full)
                    && resolved.starts_with(dir)
                    && resolved.is_file()
                    && let Some(ext) = resolved.extension().and_then(|e| e.to_str())
                    && AUDIO_EXTS.contains(&ext.to_lowercase().as_str())
                {
                    out.push(full);
                }
                continue;
            }

            if full.is_file() {
                if let Some(ext) = full.extension().and_then(|e| e.to_str())
                    && AUDIO_EXTS.contains(&ext.to_lowercase().as_str())
                {
                    out.push(full);
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

fn extract_cover_to_cache(file: &Path, dest: &Path) -> bool {
    super::audio::extract_cover(file, dest)
}

fn covers_search_dirs(user_data_dir: &Path) -> Vec<PathBuf> {
    super::tools::user_data_search_roots(user_data_dir)
        .into_iter()
        .map(|dir| dir.join("covers"))
        .collect()
}

fn extract_or_cache_cover(
    file: &Path,
    hash: &str,
    pic_codec: Option<&str>,
    user_data_dir: &Path,
) -> Option<String> {
    let covers_dir = user_data_dir.join("covers");
    let cover_ext = pic_codec.map(codec_ext).unwrap_or("jpg");
    let filename = format!("{}.{}", hash, cover_ext);

    let cached = covers_search_dirs(user_data_dir)
        .into_iter()
        .map(|dir| dir.join(&filename))
        .find(|dest| dest.exists() && dest.metadata().map(|m| m.len() > 0).unwrap_or(false));

    if let Some(path) = cached {
        return Some(path.to_string_lossy().to_string());
    }

    pic_codec?;

    let dest = covers_dir.join(&filename);
    if extract_cover_to_cache(file, &dest) {
        let has_data = dest.metadata().map(|m| m.len() > 0).unwrap_or(false);
        if has_data {
            return Some(dest.to_string_lossy().to_string());
        }
        let _ = fs::remove_file(&dest);
    }

    None
}

pub fn remove_cached_cover(user_data_dir: &Path, track_id: Option<&str>, scan_path: &str) {
    let mtime_ms = file_times(Path::new(scan_path)).0;
    let candidates = [
        track_id.map(str::to_string),
        (mtime_ms > 0).then(|| track_hash(scan_path, mtime_ms)),
    ];

    let mut ids: Vec<String> = Vec::new();
    for candidate in candidates {
        if let Some(id) = candidate
            && id.len() == 64
            && id.chars().all(|c| c.is_ascii_hexdigit())
            && !ids.contains(&id)
        {
            ids.push(id);
        }
    }
    if ids.is_empty() {
        return;
    }

    for dir in covers_search_dirs(user_data_dir) {
        for id in &ids {
            for ext in ["jpg", "png", "webp"] {
                let path = dir.join(format!("{id}.{ext}"));
                if path.is_file() {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }
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

fn probe_file(file: &Path, user_data_dir: &Path, added_at_hint: Option<u64>) -> LibraryTrack {
    let (mtime_ms, created_ms) = file_times(file);
    let added_at_ms = resolve_added_at(added_at_hint, created_ms, mtime_ms);

    let hash = track_hash(&file.to_string_lossy(), mtime_ms);

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

    let p = probe_via_lofty(file);

    if p.duration.is_none() && p.title.is_none() && p.artist.is_none() {
        return fallback;
    }

    let cover = extract_or_cache_cover(file, &hash, p.pic_codec.as_deref(), user_data_dir);

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
    scan_library_incremental(output_dir, user_data_dir, None).0
}

pub fn scan_cached_library(output_dir: &Path, user_data_dir: &Path) -> (Vec<LibraryTrack>, bool) {
    let _scan_guard = SCAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let existing = load_cache(user_data_dir);
    scan_library_incremental_unlocked(output_dir, user_data_dir, existing.as_deref())
}

fn scan_library_incremental(
    output_dir: &Path,
    user_data_dir: &Path,
    existing: Option<&[LibraryTrack]>,
) -> (Vec<LibraryTrack>, bool) {
    let _scan_guard = SCAN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    scan_library_incremental_unlocked(output_dir, user_data_dir, existing)
}

fn scan_library_incremental_unlocked(
    output_dir: &Path,
    user_data_dir: &Path,
    existing: Option<&[LibraryTrack]>,
) -> (Vec<LibraryTrack>, bool) {
    let covers_dir = user_data_dir.join("covers");
    fs::create_dir_all(&covers_dir).ok();

    if !output_dir.exists() {
        return (vec![], existing.is_none_or(|e| !e.is_empty()));
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
                && !(cached.artist.is_empty() || cached.title.is_empty())
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
                let ud = user_data_dir.to_path_buf();
                let f = file.clone();
                let hint = *added_at_hint;
                handles.push(s.spawn(move || probe_file(&f, &ud, hint)));
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

    out.sort_by_cached_key(|t| format!("{}{}", t.artist, t.title));

    let changed = existing.is_none_or(|e| e != out.as_slice());
    save_cache(user_data_dir, &out);

    (out, changed)
}
