use tauri::AppHandle;

use super::require_path_in_output;
use super::types::*;

#[tauri::command]
pub fn media_update(
    state: Option<MediaUpdate>,
    controller: tauri::State<'_, crate::services::media_controls::MediaController>,
) {
    controller.update(state);
}

#[tauri::command]
pub fn media_controls_ready(app: AppHandle) {
    crate::services::media_controls::flush_early_actions(&app);
}

#[tauri::command]
pub async fn player_play(
    path: String,
    seek_to: Option<f64>,
    generation: u64,
    app: AppHandle,
) -> Result<(), String> {
    require_path_in_output(&path, &app)?;
    tokio::task::spawn_blocking(move || crate::services::player::play(&path, seek_to, generation))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn player_crossfade(
    path: String,
    generation: u64,
    fade_secs: f64,
    app: AppHandle,
) -> Result<(), String> {
    require_path_in_output(&path, &app)?;
    tokio::task::spawn_blocking(move || {
        crate::services::player::crossfade(&path, generation, fade_secs)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn player_stop(generation: u64) -> Result<(), String> {
    tokio::task::spawn_blocking(move || crate::services::player::stop(generation))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn player_pause(generation: u64) -> Result<(), String> {
    tokio::task::spawn_blocking(move || crate::services::player::pause(generation))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn player_resume(generation: u64) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || crate::services::player::resume(generation))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn player_seek(time: f64, generation: u64) -> Result<(), String> {
    tokio::task::spawn_blocking(move || crate::services::player::seek(time, generation))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn player_set_volume(vol: u32) -> Result<(), String> {
    tokio::task::spawn_blocking(move || crate::services::player::set_volume(vol))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn player_set_speed(speed: f32) -> Result<(), String> {
    tokio::task::spawn_blocking(move || crate::services::player::set_speed(speed))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn player_set_replay_gain(enabled: bool) -> Result<(), String> {
    tokio::task::spawn_blocking(move || crate::services::player::set_replay_gain(enabled))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn player_get_position(generation: u64) -> Result<f64, String> {
    tokio::task::spawn_blocking(move || crate::services::player::get_position(generation))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn player_is_playing(generation: u64) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || crate::services::player::is_playing(generation))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn cache_remote_audio(url: String, key: String) -> Result<String, String> {
    crate::services::audio_cache::cache_remote_audio(&url, &key).await
}

#[tauri::command]
pub async fn clear_audio_cache() -> Result<(), String> {
    crate::services::audio_cache::clear_cache()
}
