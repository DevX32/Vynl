use std::fs;
use std::path::Path;

use tauri::AppHandle;

use super::types::*;
use super::{load_settings, output_dir_from_settings, require_deletable_path, user_data_dir};

#[tauri::command]
pub async fn list_playlists(app: AppHandle) -> Result<Vec<PlaylistMeta>, String> {
    let output = output_dir_from_settings(&app)?;
    let playlists = crate::services::playlists::list_playlists(&output)
        .into_iter()
        .filter(|meta| meta.id != crate::services::sync::LIKED_PLAYLIST_ID)
        .collect();
    Ok(playlists)
}

#[tauri::command]
pub async fn get_playlist(id: String, app: AppHandle) -> Result<Option<Playlist>, String> {
    if id == crate::services::sync::LIKED_PLAYLIST_ID {
        return Ok(None);
    }
    let output = output_dir_from_settings(&app)?;
    Ok(crate::services::playlists::get_playlist(&output, &id))
}

#[tauri::command]
pub async fn create_playlist(name: String, app: AppHandle) -> Result<Playlist, String> {
    let output = output_dir_from_settings(&app)?;
    let playlist = crate::services::playlists::create_playlist(&output, &name)?;
    sync_playlist(&app, &playlist);
    Ok(playlist)
}

#[tauri::command]
pub async fn reorder_playlists(ids: Vec<String>, app: AppHandle) -> Result<(), String> {
    let output = output_dir_from_settings(&app)?;
    crate::services::playlists::reorder_playlists(&output, &ids)
}

pub(crate) fn ensure_not_hidden_playlist(id: &str) -> Result<(), String> {
    if id == crate::services::sync::LIKED_PLAYLIST_ID {
        return Err("the Liked Songs playlist is managed automatically".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn rename_playlist(id: String, name: String, app: AppHandle) -> Result<Playlist, String> {
    ensure_not_hidden_playlist(&id)?;
    let output = output_dir_from_settings(&app)?;
    let playlist = crate::services::playlists::rename_playlist(&output, &id, &name)?;
    sync_playlist(&app, &playlist);
    Ok(playlist)
}

pub(crate) fn delete_track_file(user_data: &Path, path: &str, canonical: &Path) -> bool {
    let canonical_str = canonical.to_string_lossy().to_string();
    crate::services::library::remove_cached_cover(user_data, None, path);
    if fs::remove_file(canonical).is_err() {
        return false;
    }
    for ext in ["ttml", "lrc", "txt"] {
        let sidecar = crate::services::lyrics::sidecar_path(&canonical_str, ext);
        let _ = fs::remove_file(sidecar);
        let legacy = crate::services::lyrics::legacy_sidecar_path(&canonical_str, ext);
        let _ = fs::remove_file(legacy);
    }
    true
}

pub(crate) fn prune_empty_dirs(root: &Path) {
    let mut dirs = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path.clone());
                dirs.push(path);
            }
        }
    }
    let protected = root.join("playlists");
    dirs.sort_by_key(|dir| dir.as_os_str().len());
    dirs.reverse();
    for dir in dirs {
        if dir == protected || dir.starts_with(&protected) {
            continue;
        }
        let _ = fs::remove_dir(dir);
    }
}

#[tauri::command]
pub async fn delete_playlist(id: String, app: AppHandle) -> Result<DeleteTracksResult, String> {
    ensure_not_hidden_playlist(&id)?;
    let output = output_dir_from_settings(&app)?;
    let user_data = user_data_dir(&app)?;
    let mut deleted = 0_u64;
    let mut failed = 0_u64;
    if let Some(playlist) = crate::services::playlists::get_playlist(&output, &id) {
        let signature: Vec<String> = playlist
            .paths
            .iter()
            .filter_map(|path| path.strip_prefix("ytm:").map(str::to_string))
            .collect();
        let tomb_name = playlist.name.clone();
        let tomb_created = playlist.created_at;
        let mut stale: Vec<String> = Vec::new();
        for path in playlist
            .paths
            .iter()
            .filter(|path| !path.starts_with("ytm:"))
        {
            match require_deletable_path(path, &app) {
                Ok(canonical) if canonical.exists() => {
                    if delete_track_file(&user_data, path, &canonical) {
                        deleted += 1;
                    } else {
                        failed += 1;
                    }
                    stale.push(path.clone());
                    let canonical_str = canonical.to_string_lossy().to_string();
                    if canonical_str != *path {
                        stale.push(canonical_str);
                    }
                }
                Ok(_) | Err(_) => {
                    stale.push(path.clone());
                }
            }
        }
        if !stale.is_empty() {
            crate::services::playlists::remove_paths_from_all(&output, &stale);
        }
        crate::services::playlists::delete_playlist(&output, &user_data, &id)?;
        sync_playlist_delete(&app, &id, Some(&tomb_name), Some(tomb_created), &signature);
    } else {
        crate::services::playlists::delete_playlist(&output, &user_data, &id)?;
        sync_playlist_delete(&app, &id, None, None, &[]);
    }
    Ok(DeleteTracksResult { deleted, failed })
}

#[tauri::command]
pub async fn add_to_playlist(
    id: String,
    paths: Vec<String>,
    app: AppHandle,
) -> Result<Playlist, String> {
    ensure_not_hidden_playlist(&id)?;
    let output = output_dir_from_settings(&app)?;
    let validated_paths: Vec<String> = paths
        .iter()
        .map(|p| require_deletable_path(p, &app).map(|c| c.to_string_lossy().to_string()))
        .collect::<Result<Vec<String>, String>>()?;
    let playlist = crate::services::playlists::add_to_playlist(&output, &id, &validated_paths)?;
    sync_playlist(&app, &playlist);
    Ok(playlist)
}

#[tauri::command]
pub async fn remove_from_playlist(
    id: String,
    track_path: String,
    app: AppHandle,
) -> Result<Playlist, String> {
    ensure_not_hidden_playlist(&id)?;
    let output = output_dir_from_settings(&app)?;
    let user_data = user_data_dir(&app)?;
    if track_path.starts_with("ytm:") {
        let playlist = crate::services::playlists::remove_from_playlist(&output, &id, &track_path)?;
        sync_playlist(&app, &playlist);
        return Ok(playlist);
    }

    let canonical_path = require_deletable_path(&track_path, &app).ok();
    if let Some(canonical) = canonical_path.as_ref().filter(|path| path.exists()) {
        if !delete_track_file(&user_data, &track_path, canonical) {
            return Err(
                "Could not delete the file, it may be in use. Stop playback and try again.".into(),
            );
        }
    } else {
        let playlist = crate::services::playlists::remove_from_playlist(&output, &id, &track_path)?;
        sync_playlist(&app, &playlist);
        return Ok(playlist);
    }
    let mut stale = vec![track_path.clone()];
    if let Some(canonical) = canonical_path {
        let canonical_str = canonical.to_string_lossy().to_string();
        if canonical_str != track_path {
            stale.push(canonical_str);
        }
    }
    crate::services::playlists::remove_paths_from_all(&output, &stale);
    let playlist = crate::services::playlists::get_playlist(&output, &id)
        .ok_or_else(|| "playlist not found".to_string())?;
    sync_playlist(&app, &playlist);
    Ok(playlist)
}

#[tauri::command]
pub async fn move_in_playlist(
    id: String,
    from: i64,
    to: i64,
    app: AppHandle,
) -> Result<Playlist, String> {
    if from < 0 || to < 0 {
        return Err("from and to must be non-negative".into());
    }
    ensure_not_hidden_playlist(&id)?;
    let output = output_dir_from_settings(&app)?;
    let output_clone = output.clone();
    let playlist =
        crate::services::playlists::get_playlist(&output, &id).ok_or("playlist not found")?;
    let len = playlist.paths.len() as i64;
    if from >= len {
        return Err("from index out of bounds".into());
    }
    let to = to.min(len - 1);
    let playlist = crate::services::playlists::move_in_playlist(
        &output_clone,
        &id,
        from as usize,
        to as usize,
    )?;
    sync_playlist(&app, &playlist);
    Ok(playlist)
}

#[tauri::command]
pub async fn set_playlist_cover(
    id: String,
    cover: Option<String>,
    app: AppHandle,
) -> Result<Playlist, String> {
    ensure_not_hidden_playlist(&id)?;
    let output = output_dir_from_settings(&app)?;
    let user_data = user_data_dir(&app)?;
    let playlist = crate::services::playlists::set_playlist_cover(&output, &user_data, &id, cover)?;
    sync_playlist(&app, &playlist);
    Ok(playlist)
}

static SYNC_PUSH_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub(crate) fn sync_playlist(app: &AppHandle, _playlist: &Playlist) {
    let Ok(user_data) = user_data_dir(app) else {
        return;
    };
    if !crate::services::sync::status(&user_data).signed_in {
        return;
    }
    let Ok(output) = output_dir_from_settings(app) else {
        return;
    };
    let Ok(settings) = load_settings(app) else {
        return;
    };
    let generation = SYNC_PUSH_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        if generation != SYNC_PUSH_GENERATION.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        if let Err(e) = crate::services::sync::push_sync_state(&user_data, &output, &settings).await
        {
            log::warn!("[Vynl sync] background push failed: {e}");
        }
    });
}

pub(crate) fn sync_playlist_delete(
    app: &AppHandle,
    id: &str,
    _name: Option<&str>,
    _created_at: Option<i64>,
    _signature: &[String],
) {
    let Ok(user_data) = user_data_dir(app) else {
        return;
    };
    crate::services::sync::record_deleted_playlist(&user_data, id);
    if !crate::services::sync::status(&user_data).signed_in {
        return;
    }
    let Ok(output) = output_dir_from_settings(app) else {
        return;
    };
    let Ok(settings) = load_settings(app) else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        if let Err(e) = crate::services::sync::push_sync_state(&user_data, &output, &settings).await
        {
            log::warn!("[Vynl sync] background delete push failed: {e}");
        }
    });
}

#[tauri::command]
pub async fn save_playlist_cover(
    id: String,
    ext: String,
    data: Vec<u8>,
    app: AppHandle,
) -> Result<String, String> {
    ensure_not_hidden_playlist(&id)?;
    if !crate::services::playlists::validate_id(&id) {
        return Err("invalid playlist id".into());
    }
    if !crate::services::playlists::validate_cover_ext(&ext) {
        return Err("invalid image extension".into());
    }
    if data.len() > 10 * 1024 * 1024 {
        return Err("cover image too large (max 10MB)".into());
    }
    if data.len() < 8 {
        return Err("cover image data too small".into());
    }
    let is_valid_image = match ext.as_str() {
        "png" => data.len() >= 8 && data[0..8] == [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
        "jpg" | "jpeg" => data[0..2] == [0xff, 0xd8],
        "webp" => data.len() >= 12 && data[0..4] == *b"RIFF" && data[8..12] == *b"WEBP",
        _ => false,
    };
    if !is_valid_image {
        return Err("data does not match image extension".into());
    }
    let user_data = user_data_dir(&app)?;
    let covers_dir = crate::services::playlists::covers_dir(&user_data);
    fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    let filename = format!("pl-{}.{}.{}", id, stamp, ext);
    let path = covers_dir.join(&filename);
    fs::write(&path, &data).map_err(|e| e.to_string())?;
    let prefix = format!("pl-{}.", id);
    if let Ok(entries) = fs::read_dir(&covers_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(&prefix) && name != filename {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    Ok(path.to_string_lossy().into_owned())
}
