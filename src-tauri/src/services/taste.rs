use std::cmp::Reverse;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::commands::types::CatalogueTrack;

pub const TASTE_FILE: &str = "vynl.taste.v1";

const MAX_ENTRIES: usize = 160;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TasteEntry {
    pub track: CatalogueTrack,
    pub plays: u32,
    pub last_played: u64,
    pub liked: bool,
    pub liked_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TasteSummary {
    pub liked: usize,
    pub played: usize,
    pub entries: usize,
}

pub fn taste_path(user_data: &Path) -> PathBuf {
    user_data.join(TASTE_FILE)
}

fn is_entry(value: &serde_json::Value) -> bool {
    let Some(entry) = value.as_object() else {
        return false;
    };
    let Some(track) = entry.get("track").and_then(|t| t.as_object()) else {
        return false;
    };
    let has_id = track
        .get("id")
        .and_then(|id| id.as_str())
        .is_some_and(|id| !id.is_empty());
    let has_liked = entry.get("liked").is_some_and(|v| v.is_boolean());
    let has_plays = entry.get("plays").is_some_and(|v| v.is_u64());
    has_id && has_liked && has_plays
}

pub fn taste_mutex() -> &'static std::sync::Mutex<()> {
    static TASTE_MUTEX: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    TASTE_MUTEX.get_or_init(|| std::sync::Mutex::new(()))
}

pub fn locked<R>(_user_data: &Path, f: impl FnOnce() -> R) -> R {
    let _guard = taste_mutex()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f()
}

pub fn load_unlocked(user_data: &Path) -> Vec<TasteEntry> {
    let path = taste_path(user_data);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    let Some(items) = parsed.as_array() else {
        return Vec::new();
    };
    let entries: Vec<TasteEntry> = items
        .iter()
        .filter(|item| is_entry(item))
        .filter_map(|item| serde_json::from_value(item.clone()).ok())
        .collect();
    if entries.len() > MAX_ENTRIES {
        return entries[entries.len() - MAX_ENTRIES..].to_vec();
    }
    entries
}

fn save_unlocked(user_data: &Path, entries: &[TasteEntry]) -> Result<(), String> {
    let mut trimmed: Vec<&TasteEntry> = entries.iter().collect();
    if trimmed.len() > MAX_ENTRIES {
        trimmed = trimmed[trimmed.len() - MAX_ENTRIES..].to_vec();
    }
    let owned: Vec<TasteEntry> = trimmed.into_iter().cloned().collect();
    crate::services::util::write_json(&taste_path(user_data), &owned)
}

pub fn load(user_data: &Path) -> Vec<TasteEntry> {
    let _guard = taste_mutex()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    load_unlocked(user_data)
}

pub fn save(user_data: &Path, entries: &[TasteEntry]) -> Result<(), String> {
    let _guard = taste_mutex()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    save_unlocked(user_data, entries)
}

pub fn locked_update(
    user_data: &Path,
    f: impl FnOnce(&mut Vec<TasteEntry>) -> Result<bool, String>,
) -> Result<bool, String> {
    let _guard = taste_mutex()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut entries = load_unlocked(user_data);
    let changed = f(&mut entries)?;
    if changed {
        save_unlocked(user_data, &entries)?;
    }
    Ok(changed)
}

pub fn summary(entries: &[TasteEntry]) -> TasteSummary {
    TasteSummary {
        liked: entries.iter().filter(|entry| entry.liked).count(),
        played: entries.iter().filter(|entry| entry.plays > 0).count(),
        entries: entries.len(),
    }
}

pub fn liked(entries: &[TasteEntry]) -> Vec<&TasteEntry> {
    let mut liked: Vec<&TasteEntry> = entries.iter().filter(|entry| entry.liked).collect();
    liked.sort_by_key(|entry| Reverse(entry.liked_at.unwrap_or(0)));
    liked
}
