use std::fs;

use tauri::{AppHandle, Emitter};

use super::playlists::{delete_track_file, prune_empty_dirs};
use super::types::*;
use super::{
    output_dir_from_settings, require_deletable_path, require_path_in_output, user_data_dir,
    validate_path_in_output,
};

#[tauri::command]
pub async fn delete_library_track(
    path: String,
    id: Option<String>,
    app: AppHandle,
) -> Result<(), String> {
    let canonical_path = require_deletable_path(&path, &app)?;
    let canonical_str = canonical_path.to_string_lossy().to_string();
    let user_data = user_data_dir(&app)?;
    let output = output_dir_from_settings(&app)?;

    crate::services::library::remove_cached_cover(&user_data, id.as_deref(), &path);

    fs::remove_file(&canonical_path).map_err(|e| format!("Failed to delete file: {e}"))?;

    for sidecar_ext in &["ttml", "lrc", "txt"] {
        let sidecar = crate::services::lyrics::sidecar_path(&canonical_str, sidecar_ext);
        let _ = fs::remove_file(sidecar);
        let legacy = crate::services::lyrics::legacy_sidecar_path(&canonical_str, sidecar_ext);
        let _ = fs::remove_file(legacy);
    }

    let mut stale = vec![path];
    if !stale.contains(&canonical_str) {
        stale.push(canonical_str);
    }
    crate::services::playlists::remove_paths_from_all(&output, &stale);

    Ok(())
}

#[tauri::command]
pub async fn get_library(app: AppHandle) -> Result<Vec<LibraryTrack>, String> {
    let user_data = user_data_dir(&app)?;
    let output = output_dir_from_settings(&app)?;

    if let Some(cached) = crate::services::library::load_cache(&user_data) {
        let app_clone = app.clone();
        tokio::task::spawn_blocking(move || {
            let ud = user_data_dir(&app_clone).ok();
            let od = output_dir_from_settings(&app_clone).ok();
            if let (Some(ud), Some(od)) = (ud, od) {
                let (result, changed) = crate::services::library::scan_cached_library(&od, &ud);
                if changed {
                    let _ = app_clone.emit("library-updated", &result);
                }
            }
        });
        return Ok(cached);
    }

    tokio::task::spawn_blocking(move || crate::services::library::scan_library(&output, &user_data))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_all_library_tracks(app: AppHandle) -> Result<DeleteTracksResult, String> {
    let output = output_dir_from_settings(&app)?;
    let user_data = user_data_dir(&app)?;
    let files = crate::services::library::list_audio_files(&output);
    let playlists_dir = output.join("playlists");
    let mut deleted = 0_u64;
    let mut failed = 0_u64;
    let mut stale = Vec::with_capacity(files.len());
    for file in &files {
        if file.starts_with(&playlists_dir) {
            continue;
        }
        let path = file.to_string_lossy().to_string();
        if delete_track_file(&user_data, &path, file) {
            deleted += 1;
        } else {
            failed += 1;
        }
        stale.push(path.clone());

        if let Ok(canonical) = validate_path_in_output(&path, &app) {
            let canonical_str = canonical.to_string_lossy().to_string();
            if canonical_str != path {
                stale.push(canonical_str);
            }
        }
    }
    if !stale.is_empty() {
        crate::services::playlists::remove_paths_from_all(&output, &stale);
    }
    prune_empty_dirs(&output);
    Ok(DeleteTracksResult { deleted, failed })
}

#[tauri::command]
pub async fn library_read_tags(
    file: String,
    app: AppHandle,
) -> Result<crate::services::audio::TrackTags, String> {
    let validated = require_path_in_output(&file, &app)?;
    Ok(crate::services::audio::read_tags(&validated))
}

#[tauri::command]
pub async fn library_write_tags(
    file: String,
    tags: crate::services::audio::TrackTags,
    app: AppHandle,
) -> Result<(), String> {
    let validated = require_path_in_output(&file, &app)?;
    crate::services::audio::write_tags(&validated, &tags)
}

#[tauri::command]
pub async fn clear_storage_cache(app: AppHandle) -> Result<u64, String> {
    let user_data = user_data_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        Ok(crate::services::storage::clear_caches(&user_data))
    })
    .await
    .map_err(|e| format!("Could not clear the cache: {e}"))?
}

#[tauri::command]
pub async fn downloads_stats(
    app: AppHandle,
) -> Result<crate::services::storage::DownloadsStats, String> {
    let output = output_dir_from_settings(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        Ok(crate::services::storage::downloads_stats(&output))
    })
    .await
    .map_err(|e| format!("Could not read the downloads folder: {e}"))?
}
