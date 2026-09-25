use std::collections::HashMap;
use std::path::Path;

use crate::commands::types::HistoryEntry;
use crate::services::util;

const MAX_ENTRIES: usize = 500;

crate::declare_file_mutex!();

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct HistoryFile {
    entries: Option<Vec<HistoryEntry>>,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
struct SyncEntry {
    track_ids: Vec<String>,
    at: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct SyncFile {
    collections: Option<HashMap<String, SyncEntry>>,
}

fn history_file(user_data_dir: &Path) -> std::path::PathBuf {
    user_data_dir.join("history.json")
}

fn sync_file(user_data_dir: &Path) -> std::path::PathBuf {
    user_data_dir.join("sync.json")
}

fn read_history(user_data_dir: &Path) -> HistoryFile {
    util::read_json(&history_file(user_data_dir)).unwrap_or_default()
}

fn write_history(user_data_dir: &Path, data: &HistoryFile) -> Result<(), String> {
    util::write_json(&history_file(user_data_dir), data)
}

fn read_sync(user_data_dir: &Path) -> SyncFile {
    util::read_json(&sync_file(user_data_dir)).unwrap_or_default()
}

fn write_sync(user_data_dir: &Path, data: &SyncFile) -> Result<(), String> {
    util::write_json(&sync_file(user_data_dir), data)
}

pub fn record_download(user_data_dir: &Path, entry: &HistoryEntry) -> Result<(), String> {
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let mut data = read_history(user_data_dir);
    let mut entries = data.entries.unwrap_or_default();
    entries.insert(0, entry.clone());
    entries.truncate(MAX_ENTRIES);
    data.entries = Some(entries);
    write_history(user_data_dir, &data)
}

pub fn collection_key(kind: &crate::commands::types::CollectionKind, id: &str) -> String {
    let kind_str = match kind {
        crate::commands::types::CollectionKind::Track => "track",
        crate::commands::types::CollectionKind::Album => "album",
        crate::commands::types::CollectionKind::Playlist => "playlist",
    };
    format!("{}:{}", kind_str, id)
}

pub fn mark_collection_done(user_data_dir: &Path, key: &str, track_id: &str) -> Result<(), String> {
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let mut data = read_sync(user_data_dir);
    let collections = data.collections.get_or_insert_with(HashMap::new);
    let entry = collections
        .entry(key.to_string())
        .or_insert_with(|| SyncEntry {
            track_ids: Vec::new(),
            at: util::now_ms(),
        });
    if !entry.track_ids.contains(&track_id.to_string()) {
        entry.track_ids.push(track_id.to_string());
    }
    entry.at = util::now_ms();
    write_sync(user_data_dir, &data)
}

pub fn collection_done(user_data_dir: &Path, key: &str) -> Vec<String> {
    let _lock = file_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let data = read_sync(user_data_dir);
    data.collections
        .and_then(|c| c.get(key).cloned())
        .map(|e| e.track_ids)
        .unwrap_or_default()
}
