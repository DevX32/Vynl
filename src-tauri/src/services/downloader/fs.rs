use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::{
    audio_format_ext, emit_track, normalize, render_pattern, same_song, sanitize, song_key,
};
use crate::commands::types::{Collection, Settings, TrackMeta, TrackStatus};
use crate::services::history;

pub(super) fn move_file(from: &Path, to: &Path) -> Result<(), String> {
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            static MOVE_SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let serial = MOVE_SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let tmp = to.with_extension(format!(
                "{}.{}-{}.tmp",
                to.extension().and_then(|e| e.to_str()).unwrap_or(""),
                std::process::id(),
                serial
            ));
            if let Err(error) = std::fs::copy(from, &tmp) {
                let _ = std::fs::remove_file(&tmp);
                return Err(error.to_string());
            }
            match std::fs::rename(&tmp, to) {
                Ok(()) => {
                    let _ = std::fs::remove_file(from);
                    Ok(())
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&tmp);
                    Err(e.to_string())
                }
            }
        }
    }
}

pub(super) fn cleanup_tmp_dir(tmp: &Path) {
    if !tmp.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(tmp) {
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                let _ = std::fs::remove_file(&path);
            } else if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let _ = std::fs::remove_dir_all(&path);
            }
        }
    }
}

pub fn find_song_on_disk(
    output_dir: &Path,
    user_data_dir: &Path,
    track: &TrackMeta,
) -> Option<PathBuf> {
    let key = song_key(&track.title, &track.artist);
    for found in crate::services::library::scan_cached_library(output_dir, user_data_dir).0 {
        if song_key(&found.title, &found.artist) != key {
            continue;
        }
        if same_song(found.duration, track.duration) {
            return Some(PathBuf::from(found.path));
        }
    }
    None
}

pub(super) fn existing_songs(output_dir: &Path, user_data_dir: &Path) -> HashMap<String, Vec<f64>> {
    let mut map: HashMap<String, Vec<f64>> = HashMap::new();
    for track in crate::services::library::scan_cached_library(output_dir, user_data_dir).0 {
        if track.title.trim().is_empty() {
            continue;
        }
        map.entry(song_key(&track.title, &track.artist))
            .or_default()
            .push(track.duration);
    }
    map
}

pub(super) fn is_already_on_disk(existing: &HashMap<String, Vec<f64>>, track: &TrackMeta) -> bool {
    let Some(durations) = existing.get(&song_key(&track.title, &track.artist)) else {
        return false;
    };
    durations.iter().any(|d| same_song(*d, track.duration))
}

pub(super) fn deduplicate_tracks(
    tracks: &[TrackMeta],
    app_handle: &tauri::AppHandle,
    existing: &HashMap<String, Vec<f64>>,
) -> (Vec<TrackMeta>, usize) {
    let mut seen_keys: HashSet<String> = HashSet::new();
    let mut deduped: Vec<TrackMeta> = Vec::new();
    let mut skipped = 0;
    for track in tracks {
        let key = format!(
            "{}||{}||{}",
            normalize(&track.title),
            normalize(&track.artist),
            track.track_number.map_or("".to_string(), |n| n.to_string())
        );
        if !seen_keys.insert(key) {
            emit_track(
                app_handle,
                track,
                TrackStatus::Skipped,
                0.0,
                Some("duplicate"),
            );
            skipped += 1;
            continue;
        }
        if is_already_on_disk(existing, track) {
            emit_track(
                app_handle,
                track,
                TrackStatus::Skipped,
                0.0,
                Some("already in library"),
            );
            skipped += 1;
            continue;
        }
        deduped.push(track.clone());
    }
    (deduped, skipped)
}

pub fn final_output_path(track: &TrackMeta, settings: &Settings) -> PathBuf {
    final_output_path_in(track, settings, Path::new(&settings.output_dir))
}

pub fn final_output_path_in(track: &TrackMeta, settings: &Settings, output_dir: &Path) -> PathBuf {
    let base_name = sanitize(&render_pattern(&settings.filename_pattern, track));
    output_dir.join(format!(
        "{}.{}",
        base_name,
        audio_format_ext(&settings.format)
    ))
}

pub fn get_done_track_ids(
    collection: &Collection,
    user_data_dir: &Path,
    settings: Option<&Settings>,
) -> Vec<String> {
    let key = history::collection_key(&collection.kind, &collection.id);
    let history_ids: Vec<String> = history::collection_done(user_data_dir, &key);
    let history_set: HashSet<&str> = history_ids.iter().map(String::as_str).collect();
    let mut done: HashSet<String> = history_ids.iter().cloned().collect();

    if let Some(s) = settings
        && !s.output_dir.is_empty()
    {
        let output = Path::new(&s.output_dir);
        if output.exists() {
            let lookup = history::HistoryLookup::load(user_data_dir);
            for track in &collection.tracks {
                let path = final_output_path(track, s);
                if path.exists() || lookup.has_downloaded_same_song(&track.title, &track.artist) {
                    done.insert(track.id.clone());
                }
            }
        }
        done.retain(|id| {
            if history_set.contains(id.as_str())
                && let Some(track) = collection.tracks.iter().find(|t| &t.id == id)
            {
                let path = final_output_path(track, s);
                return path.exists();
            }
            true
        });
    }

    let mut out: Vec<String> = done.into_iter().collect();
    out.sort();
    out
}
