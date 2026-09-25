use std::fs;
use std::path::Path;

use crate::commands::types::NowPlayingState;
use crate::services::util;

crate::declare_file_mutex!();

fn now_playing_file(user_data_dir: &Path) -> std::path::PathBuf {
    user_data_dir.join("now-playing.json")
}

pub fn save_now_playing(user_data_dir: &Path, state: &NowPlayingState) -> Result<(), String> {
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    util::write_json(&now_playing_file(user_data_dir), state)
}

pub fn load_now_playing(user_data_dir: &Path) -> Option<NowPlayingState> {
    let _lock = file_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let path = now_playing_file(user_data_dir);
    let state: NowPlayingState = util::read_json(&path)?;
    let age_ms = util::now_ms().saturating_sub(state.timestamp);
    if age_ms > 7 * 24 * 60 * 60 * 1000 {
        let _ = fs::remove_file(&path);
        return None;
    }
    Some(state)
}

pub fn clear_now_playing(user_data_dir: &Path) -> Result<(), String> {
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let path = now_playing_file(user_data_dir);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
