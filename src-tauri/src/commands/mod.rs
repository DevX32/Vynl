pub mod types;

mod backup;
mod catalogue;
mod library;
mod lyrics;
mod playback;
mod playlists;
mod settings;
mod sync;
mod updates;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

use types::*;

pub use backup::*;
pub use catalogue::*;
pub use library::*;
pub use lyrics::*;
pub use playback::*;
pub use playlists::*;
pub use settings::*;
pub use sync::*;
pub use updates::*;

pub(crate) const MAX_COLLECTION_TRACKS: usize = 10_000;

pub(crate) fn ensure_collection_size(collection: &Collection) -> Result<(), String> {
    ensure_collection_size_named(collection, "Collection too large")
}

pub(crate) fn ensure_collection_size_named(
    collection: &Collection,
    label: &str,
) -> Result<(), String> {
    if collection.tracks.len() > MAX_COLLECTION_TRACKS {
        return Err(format!("{label} (max {} tracks)", MAX_COLLECTION_TRACKS));
    }
    Ok(())
}

pub struct AppState {
    pub settings: Mutex<Option<Settings>>,
    pub pending_confirm: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            settings: Mutex::new(None),
            pending_confirm: Mutex::new(None),
        }
    }
}

pub(crate) fn user_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))
}

pub(crate) fn output_dir_from_settings(app: &AppHandle) -> Result<PathBuf, String> {
    let state = app.state::<AppState>();
    let guard = state.settings.lock().map_err(|e| e.to_string())?;
    guard
        .as_ref()
        .map(|s| PathBuf::from(&s.output_dir))
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(dirs::audio_dir)
        .ok_or_else(|| "No output directory is configured".to_string())
}

fn canonicalize_in_output(file: &str, canonical_output: &Path) -> Result<PathBuf, String> {
    let path = Path::new(file);
    if !path.is_absolute() {
        return Err("Path must be absolute".into());
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| "invalid path: missing file name".to_string())?;
    let canonical = fs::canonicalize(path)
        .or_else(|_| {
            path.parent()
                .and_then(|parent| fs::canonicalize(parent).ok())
                .map(|parent| parent.join(file_name))
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "file or parent directory does not exist",
                    )
                })
        })
        .map_err(|e| format!("invalid path: {e}"))?;

    if !canonical.starts_with(canonical_output) {
        return Err("Path must be within the output directory".into());
    }
    Ok(canonical)
}

pub(crate) fn canonical_output_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let output = output_dir_from_settings(app)?;
    fs::create_dir_all(&output).map_err(|e| format!("invalid output directory: {e}"))?;
    fs::canonicalize(&output).map_err(|e| format!("invalid output directory: {e}"))
}

pub(crate) fn validate_path_in_output(file: &str, app: &AppHandle) -> Result<PathBuf, String> {
    canonicalize_in_output(file, &canonical_output_dir(app)?)
}

pub(crate) fn require_path_in_output(file: &str, app: &AppHandle) -> Result<PathBuf, String> {
    let path = validate_path_in_output(file, app)?;
    if !path.exists() {
        return Err("File does not exist".into());
    }
    Ok(path)
}

const DELETABLE_EXTS: &[&str] = &[
    "mp3", "m4a", "mka", "m4b", "opus", "flac", "wav", "ogg", "aac", "webm", "mp4", "wma", "aiff",
    "ape", "wv",
];

pub(crate) fn require_deletable_path(file: &str, app: &AppHandle) -> Result<PathBuf, String> {
    let canonical = require_path_in_output(file, app)?;
    let canonical_output = canonical_output_dir(app)?;
    if canonical.starts_with(canonical_output.join("playlists")) {
        return Err("Path is inside the playlists directory".into());
    }
    let ext = canonical
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();
    if !DELETABLE_EXTS.contains(&ext.as_str()) {
        return Err("Path must be an audio file".into());
    }
    Ok(canonical)
}

pub(crate) fn load_settings(app: &AppHandle) -> Result<Settings, String> {
    let state = app.state::<AppState>();
    let guard = state.settings.lock().map_err(|e| e.to_string())?;
    guard.clone().ok_or("Settings not initialized".into())
}

pub(crate) fn get_user_data_and_settings(
    app: &AppHandle,
) -> Result<(std::path::PathBuf, Settings), String> {
    let user_data = user_data_dir(app)?;
    let settings = load_settings(app)?;
    Ok((user_data, settings))
}

#[tauri::command]
pub fn take_pending_confirm_url(app: AppHandle) -> Result<Option<String>, String> {
    let state = app.state::<AppState>();
    let mut pending = state
        .pending_confirm
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    Ok(pending.take())
}
