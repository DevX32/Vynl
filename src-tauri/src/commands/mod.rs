pub mod types;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

use types::*;

pub(crate) const MAX_COLLECTION_TRACKS: usize = 10_000;

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

fn user_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
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

    let canonical = fs::canonicalize(path)
        .or_else(|_| {
            path.parent()
                .and_then(|parent| fs::canonicalize(parent).ok())
                .map(|parent| parent.join(path.file_name().unwrap_or_default()))
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

fn canonical_output_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let output = output_dir_from_settings(app)?;
    Ok(fs::canonicalize(&output).unwrap_or(output))
}

fn validate_path_in_output(file: &str, app: &AppHandle) -> Result<PathBuf, String> {
    canonicalize_in_output(file, &canonical_output_dir(app)?)
}

fn require_path_in_output(file: &str, app: &AppHandle) -> Result<PathBuf, String> {
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

fn require_deletable_path(file: &str, app: &AppHandle) -> Result<PathBuf, String> {
    let path = require_path_in_output(file, app)?;
    let canonical = fs::canonicalize(&path).unwrap_or(path);
    if let Ok(output) = output_dir_from_settings(app)
        && let Ok(canonical_output) = fs::canonicalize(&output)
        && canonical.starts_with(canonical_output.join("playlists"))
    {
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

fn load_settings(app: &AppHandle) -> Result<Settings, String> {
    let state = app.state::<AppState>();
    let guard = state.settings.lock().map_err(|e| e.to_string())?;
    guard.clone().ok_or("Settings not initialized".into())
}

fn get_user_data_and_settings(app: &AppHandle) -> Result<(std::path::PathBuf, Settings), String> {
    let user_data = user_data_dir(app)?;
    let settings = load_settings(app)?;
    Ok((user_data, settings))
}

#[tauri::command]
pub async fn resolve(url: String, app: AppHandle) -> Result<Collection, String> {
    let generation = crate::services::progress::set_emitter(Some(app.clone()));
    let result = crate::services::downloader::resolve_link(&url).await;
    crate::services::progress::clear_emitter(generation);
    result
}

#[tauri::command]
pub async fn search_tracks(
    query: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<SearchResult>, String> {
    let l = limit.unwrap_or(20);
    let o = offset.unwrap_or(0);
    crate::services::internal::search_tracks(&query, l, o).await
}

#[tauri::command]
pub async fn downloaded_paths(
    collection: Collection,
    app: AppHandle,
) -> Result<Vec<String>, String> {
    let settings = load_settings(&app)?;
    let ext = crate::services::downloader::audio_format_ext(&settings.format);
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    Ok(collection
        .tracks
        .iter()
        .filter_map(|t| {
            let base = crate::services::downloader::sanitize(
                &crate::services::downloader::render_pattern(&settings.filename_pattern, t),
            );
            let path =
                std::path::PathBuf::from(&settings.output_dir).join(format!("{}.{}", base, ext));
            if !path.is_file() {
                return None;
            }
            let path = path.to_string_lossy().to_string();
            seen.insert(path.clone()).then_some(path)
        })
        .collect())
}

#[tauri::command]
pub async fn start_download(
    collection: Collection,
    opts: Option<DownloadOpts>,
    app: AppHandle,
) -> Result<(), String> {
    if collection.tracks.len() > MAX_COLLECTION_TRACKS {
        return Err(format!(
            "Collection too large (max {} tracks)",
            MAX_COLLECTION_TRACKS
        ));
    }
    let (user_data, settings) = get_user_data_and_settings(&app)?;
    let download_opts = opts.unwrap_or(DownloadOpts {
        matches: None,
        picks: None,
        skip_ids: None,
    });
    crate::services::downloader::download_collection(
        collection,
        settings,
        &user_data,
        app,
        download_opts,
    )
    .await
}

#[tauri::command]
pub async fn cancel_download() -> Result<(), String> {
    crate::services::downloader::cancel_download().await;
    Ok(())
}

#[tauri::command]
pub async fn export_backup(
    app: AppHandle,
) -> Result<Option<crate::services::export::ExportSummary>, String> {
    use tauri_plugin_dialog::DialogExt;

    let user_data = user_data_dir(&app)?;
    let settings = load_settings(&app)?;

    let (default_name, content, track_count, playlist_count) =
        tokio::task::spawn_blocking(move || {
            let backup = crate::services::export::build_backup(&settings, &user_data);
            let filename = crate::services::export::suggested_filename(&backup);
            let content = crate::services::export::render(&backup)?;
            Ok::<_, String>((filename, content, backup.track_count, backup.playlist_count))
        })
        .await
        .map_err(|e| format!("Export task failed: {e}"))??;

    let dialog_app = app.clone();
    let ext = crate::services::export::EXPORT_EXTENSION;
    let label = crate::services::export::EXPORT_LABEL;
    let picked = tokio::task::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .set_title("Export library backup")
            .set_file_name(&default_name)
            .add_filter(label, &[ext])
            .blocking_save_file()
    })
    .await
    .map_err(|e| format!("Export task failed: {e}"))?;

    let Some(picked) = picked else {
        return Ok(None);
    };

    let path = match picked {
        tauri_plugin_dialog::FilePath::Path(p) => p,
        tauri_plugin_dialog::FilePath::Url(u) => u
            .to_file_path()
            .map_err(|_| "Could not resolve the chosen location.".to_string())?,
    };
    let path_string = path.to_string_lossy().to_string();

    let bytes =
        tokio::task::spawn_blocking(move || crate::services::export::write_backup(&content, &path))
            .await
            .map_err(|e| format!("Export task failed: {e}"))??;

    Ok(Some(crate::services::export::ExportSummary {
        path: path_string,
        format: crate::services::export::EXPORT_EXTENSION.to_string(),
        track_count,
        playlist_count,
        bytes,
    }))
}

#[tauri::command]
pub async fn import_backup(
    app: AppHandle,
) -> Result<Option<crate::services::export::RestoredBackup>, String> {
    use tauri_plugin_dialog::DialogExt;

    let dialog_app = app.clone();
    let picked = tokio::task::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .set_title("Open Vynl backup")
            .add_filter("Vynl backup", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| format!("Import task failed: {e}"))?;

    let Some(picked) = picked else {
        return Ok(None);
    };

    let path = match picked {
        tauri_plugin_dialog::FilePath::Path(p) => p,
        tauri_plugin_dialog::FilePath::Url(u) => u
            .to_file_path()
            .map_err(|_| "Could not resolve the chosen file.".to_string())?,
    };

    let restored = tokio::task::spawn_blocking(move || {
        let backup = crate::services::export::read_backup_file(&path)?;
        Ok::<_, String>(crate::services::export::restore(&backup))
    })
    .await
    .map_err(|e| format!("Import task failed: {e}"))??;

    if restored.total == 0 {
        return Err("That backup doesn't contain any tracks.".into());
    }

    Ok(Some(restored))
}

#[tauri::command]
pub async fn matches(
    collection: Collection,
    app: AppHandle,
) -> Result<HashMap<String, Vec<SearchCandidate>>, String> {
    if collection.tracks.len() > MAX_COLLECTION_TRACKS {
        return Err(format!(
            "Collection too large (max {} tracks)",
            MAX_COLLECTION_TRACKS
        ));
    }
    let user_data = user_data_dir(&app)?;
    crate::services::downloader::find_matches(&collection, &user_data).await
}

#[tauri::command]
pub async fn cancel_stream_import() {
    crate::services::downloader::cancel_find();
}

#[tauri::command]
pub async fn sync_done(collection: Collection, app: AppHandle) -> Result<Vec<String>, String> {
    if collection.tracks.len() > MAX_COLLECTION_TRACKS {
        return Err(format!(
            "Collection too large (max {} tracks)",
            MAX_COLLECTION_TRACKS
        ));
    }
    let user_data = user_data_dir(&app)?;
    let settings = {
        let state = app.state::<AppState>();
        let guard = state.settings.lock().map_err(|e| e.to_string())?;
        guard
            .clone()
            .unwrap_or_else(|| crate::services::settings::get_settings(&user_data))
    };
    Ok(crate::services::downloader::get_done_track_ids(
        &collection,
        &user_data,
        Some(&settings),
    ))
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> Result<Settings, String> {
    let state = app.state::<AppState>();
    let mut guard = state.settings.lock().map_err(|e| e.to_string())?;
    if let Some(ref s) = *guard {
        return Ok(s.clone());
    }
    let user_data = user_data_dir(&app)?;
    let s = crate::services::settings::get_settings(&user_data);
    *guard = Some(s.clone());
    Ok(s)
}

#[tauri::command]
pub async fn set_settings(patch: Settings, app: AppHandle) -> Result<Settings, String> {
    let user_data = user_data_dir(&app)?;
    let next = crate::services::settings::set_settings(&user_data, patch)?;
    let (launch_changed, media_keys_changed) = {
        let state = app.state::<AppState>();
        let mut guard = state.settings.lock().map_err(|e| e.to_string())?;
        let launch_changed =
            guard.as_ref().map(|s| s.launch_at_startup) != Some(next.launch_at_startup);
        let prev_media_keys = guard.as_ref().map(|s| s.global_media_keys);
        let media_keys_changed = prev_media_keys != Some(next.global_media_keys);
        *guard = Some(next.clone());
        (launch_changed, media_keys_changed)
    };
    if launch_changed {
        crate::sync_launch_at_startup(&app, next.launch_at_startup);
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    if media_keys_changed {
        if next.global_media_keys {
            crate::register_global_media_keys(app.clone());
        } else {
            crate::unregister_global_media_keys(app.clone());
        }
    }

    crate::services::player::set_eq(next.eq_enabled, &next.eq_bands);
    Ok(next)
}

#[tauri::command]
pub async fn get_eq_design() -> Result<crate::services::player::EqDesign, String> {
    Ok(crate::services::player::eq_design())
}

#[tauri::command]
pub async fn get_mobile_sync_status(
    app: AppHandle,
) -> Result<crate::services::mobile_sync::MobileSyncStatus, String> {
    let user_data = user_data_dir(&app)?;
    Ok(crate::services::mobile_sync::get_status(&user_data).await)
}

#[tauri::command]
pub async fn set_mobile_sync_enabled(
    enabled: bool,
    app: AppHandle,
) -> Result<crate::services::mobile_sync::MobileSyncStatus, String> {
    let user_data = user_data_dir(&app)?;
    crate::services::mobile_sync::set_enabled(&user_data, enabled).await
}

#[tauri::command]
pub async fn rotate_mobile_sync_pin(
    app: AppHandle,
) -> Result<crate::services::mobile_sync::MobileSyncStatus, String> {
    let user_data = user_data_dir(&app)?;
    crate::services::mobile_sync::rotate_pin(&user_data).await
}

#[tauri::command]
pub async fn taste_load(app: AppHandle) -> Result<Vec<crate::services::taste::TasteEntry>, String> {
    let user_data = user_data_dir(&app)?;
    Ok(crate::services::taste::load(&user_data))
}

#[tauri::command]
pub async fn taste_save(
    entries: Vec<crate::services::taste::TasteEntry>,
    app: AppHandle,
) -> Result<(), String> {
    let user_data = user_data_dir(&app)?;
    let output = output_dir_from_settings(&app)?;
    mirror_taste_to_liked(&output, &user_data, &entries, &app)?;
    crate::services::taste::save(&user_data, &entries)
}

fn mirror_taste_to_liked(
    output_dir: &std::path::Path,
    user_data_dir: &std::path::Path,
    entries: &[crate::services::taste::TasteEntry],
    app: &AppHandle,
) -> Result<(), String> {
    let before: std::collections::HashSet<String> =
        crate::services::taste::locked(user_data_dir, || {
            crate::services::taste::load_unlocked(user_data_dir)
                .iter()
                .filter(|entry| entry.liked)
                .map(|entry| format!("ytm:{}", entry.track.id))
                .collect()
        });
    let after: std::collections::HashSet<String> = entries
        .iter()
        .filter(|entry| entry.liked)
        .map(|entry| format!("ytm:{}", entry.track.id))
        .collect();
    let adds: Vec<String> = after.difference(&before).cloned().collect();
    let removals: Vec<String> = before.difference(&after).cloned().collect();
    if adds.is_empty() && removals.is_empty() {
        return Ok(());
    }
    let now = crate::services::util::now_ms();
    let id = crate::services::sync::LIKED_PLAYLIST_ID;
    let mut playlist = match crate::services::playlists::get_playlist(output_dir, id) {
        Some(found) => found,
        None => {
            let streams: Vec<crate::commands::types::CatalogueTrack> = entries
                .iter()
                .filter(|entry| entry.liked)
                .map(|entry| entry.track.clone())
                .collect();
            let paths: Vec<String> = streams
                .iter()
                .map(|track| format!("ytm:{}", track.id))
                .collect();
            let created_at = entries
                .iter()
                .filter_map(|entry| entry.liked_at)
                .min()
                .map(|at| at.min(i64::MAX as u64) as i64)
                .unwrap_or(now);
            crate::commands::types::Playlist {
                id: id.to_string(),
                name: "Liked Songs".to_string(),
                paths,
                streams,
                adds: Vec::new(),
                removals: Vec::new(),
                cover: None,
                created_at,
                updated_at: now,
            }
        }
    };
    for key in &adds {
        let Some(id) = key.strip_prefix("ytm:") else {
            continue;
        };
        let Some(entry) = entries.iter().find(|entry| entry.track.id == id) else {
            continue;
        };
        let path = format!("ytm:{id}");
        if !playlist.paths.contains(&path) {
            playlist.paths.push(path);
        }
        match playlist.streams.iter_mut().find(|track| track.id == id) {
            Some(existing) => *existing = entry.track.clone(),
            None => playlist.streams.push(entry.track.clone()),
        }
    }
    for key in &removals {
        playlist.paths.retain(|p| p != key);
        if let Some(id) = key.strip_prefix("ytm:") {
            playlist.streams.retain(|track| track.id != id);
        }
    }
    crate::services::playlists::record_marks(&mut playlist, &adds, &removals);
    playlist.updated_at = now;
    crate::services::playlists::upsert_playlist(output_dir, &playlist)?;
    sync_playlist(app, &playlist);
    Ok(())
}

#[tauri::command]
pub async fn get_tools(app: AppHandle) -> Result<Vec<ToolStatus>, String> {
    let user_data = user_data_dir(&app)?;
    Ok(crate::services::tools::check_tools(&user_data, &app).await)
}

#[tauri::command]
pub async fn check_tool_updates(app: AppHandle) -> Result<Vec<ToolStatus>, String> {
    let user_data = user_data_dir(&app)?;
    Ok(crate::services::tools::check_for_updates(&user_data, &app).await)
}

#[tauri::command]
pub async fn install_tool(name: String, app: AppHandle) -> Result<(), String> {
    let user_data = user_data_dir(&app)?;
    crate::services::tools::install_tool(&user_data, &name, app).await
}

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

fn ensure_not_hidden_playlist(id: &str) -> Result<(), String> {
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

fn delete_track_file(user_data: &Path, path: &str, canonical: &Path) -> bool {
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

fn prune_empty_dirs(root: &Path) {
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
    dirs.sort_by_key(|dir| dir.as_os_str().len());
    dirs.reverse();
    for dir in dirs {
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

    let canonical = require_deletable_path(&track_path, &app).ok();
    let exists = canonical.as_ref().is_some_and(|path| path.exists());
    if exists {
        let canonical = canonical.as_ref().expect("canonical checked");
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
    if let Some(canonical) = canonical {
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
    tauri::async_runtime::spawn(async move {
        if let Err(e) = crate::services::sync::push_sync_state(&user_data, &output, &settings).await
        {
            eprintln!("[Vynl sync] background push failed: {e}");
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
            eprintln!("[Vynl sync] background delete push failed: {e}");
        }
    });
}

#[tauri::command]
pub async fn sync_pull(app: AppHandle) -> Result<crate::services::sync::SyncState, String> {
    let user_data = user_data_dir(&app)?;
    let output = output_dir_from_settings(&app)?;
    let result = crate::services::sync::pull_sync_state(&user_data, &output).await;
    if let Err(error) = &result {
        crate::services::sync::record_sync_error(&user_data, error);
    }
    result
}

#[tauri::command]
pub async fn sync_push(app: AppHandle) -> Result<(), String> {
    let user_data = user_data_dir(&app)?;
    let output = output_dir_from_settings(&app)?;
    let settings = load_settings(&app)?;
    crate::services::sync::push_sync_state(&user_data, &output, &settings).await
}

#[tauri::command]
pub fn sync_mark_dirty(app: AppHandle) -> Result<(), String> {
    let user_data = user_data_dir(&app)?;
    crate::services::sync::mark_sync_dirty(&user_data);
    Ok(())
}

#[tauri::command]
pub async fn sync_now(app: AppHandle) -> Result<crate::services::sync::SyncState, String> {
    let user_data = user_data_dir(&app)?;
    let output = output_dir_from_settings(&app)?;
    let settings = load_settings(&app)?;
    let state = match crate::services::sync::pull_sync_state(&user_data, &output).await {
        Ok(state) => state,
        Err(e) if e.contains("No cloud sync state found") => {
            crate::services::sync::build_sync_state(&user_data, &output, &settings)
        }
        Err(e) => {
            crate::services::sync::record_sync_error(&user_data, &e);
            return Err(e);
        }
    };
    crate::services::sync::push_sync_state(&user_data, &output, &settings).await?;
    Ok(state)
}

#[tauri::command]
pub async fn sync_resolve_missing_source(
    app: AppHandle,
    path: String,
) -> Result<Option<String>, String> {
    let user_data = user_data_dir(&app)?;
    let state = crate::services::sync::read_sync_state_cache(&user_data);
    let Some(state) = state else {
        return Ok(None);
    };
    if let Some(track) = state.tracks.iter().find(|t| t.path == path)
        && let Some(url) = &track.source_url
    {
        return Ok(Some(url.clone()));
    }
    for pl in &state.playlists {
        if let Some(track) = pl.tracks.iter().find(|t| t.path == path)
            && let Some(url) = &track.source_url
        {
            return Ok(Some(url.clone()));
        }
    }
    Ok(None)
}

#[tauri::command]
pub async fn sync_status(app: AppHandle) -> Result<crate::services::sync::SyncStatus, String> {
    let user_data = user_data_dir(&app)?;
    Ok(crate::services::sync::status(&user_data))
}

async fn sync_auth(
    app: &AppHandle,
    email: String,
    password: String,
    create: bool,
) -> Result<crate::services::sync::SyncAuthResult, String> {
    let user_data = user_data_dir(app)?;
    let output = output_dir_from_settings(app)?;
    let result = if create {
        crate::services::sync::sign_up(&user_data, &email, &password).await?
    } else {
        crate::services::sync::sign_in(&user_data, &email, &password).await?
    };
    if result.status.signed_in {
        let settings = load_settings(app)?;
        let mut can_push = true;
        match crate::services::sync::pull_sync_state(&user_data, &output).await {
            Ok(_) => {}
            Err(e) if e.contains("No cloud sync state found") => {}
            Err(e) => {
                eprintln!("[Vynl sync] auto pull after sign-in failed: {e}");
                crate::services::sync::record_sync_error(&user_data, &e);
                can_push = false;
            }
        }
        if can_push
            && let Err(e) =
                crate::services::sync::push_sync_state(&user_data, &output, &settings).await
        {
            eprintln!("[Vynl sync] auto push after sign-in failed: {e}");
        }
    }
    Ok(result)
}

#[tauri::command]
pub async fn sync_sign_in(
    email: String,
    password: String,
    app: AppHandle,
) -> Result<crate::services::sync::SyncAuthResult, String> {
    sync_auth(&app, email, password, false).await
}

#[tauri::command]
pub async fn sync_sign_up(
    email: String,
    password: String,
    app: AppHandle,
) -> Result<crate::services::sync::SyncAuthResult, String> {
    sync_auth(&app, email, password, true).await
}

#[tauri::command]
pub async fn sync_sign_out(app: AppHandle) -> Result<(), String> {
    let user_data = user_data_dir(&app)?;
    crate::services::sync::sign_out(&user_data).await
}

#[tauri::command]
pub async fn sync_confirm_email(token_hash: String, email_type: String) -> Result<bool, String> {
    crate::services::sync::confirm_email(&token_hash, &email_type)
        .await
        .map(|_| true)
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

#[tauri::command]
pub async fn library_read_tags(file: String) -> Result<crate::services::audio::TrackTags, String> {
    Ok(crate::services::audio::read_tags(std::path::Path::new(
        &file,
    )))
}

#[tauri::command]
pub async fn library_write_tags(
    file: String,
    tags: crate::services::audio::TrackTags,
) -> Result<(), String> {
    crate::services::audio::write_tags(std::path::Path::new(&file), &tags)
}

#[tauri::command]
pub async fn lyrics_local(file: String, app: AppHandle) -> Result<Option<LyricsResult>, String> {
    match require_path_in_output(&file, &app) {
        Ok(validated) => Ok(crate::services::lyrics::local_lyrics(
            &validated.to_string_lossy(),
        )),
        Err(_) => {
            let user_data = user_data_dir(&app)?;
            Ok(crate::services::lyrics::path_lyrics(&user_data, &file))
        }
    }
}

#[tauri::command]
pub async fn lyrics_fetch(
    lookup: LyricsLookup,
    app: AppHandle,
) -> Result<Option<LyricsResult>, String> {
    let user_data = user_data_dir(&app)?;
    Ok(crate::services::lyrics::fetch_lyrics_with_cache(&lookup, &user_data).await)
}

#[tauri::command]
pub async fn lyrics_embed(file: String, text: String, app: AppHandle) -> Result<(), String> {
    let validated = require_path_in_output(&file, &app)?;
    tokio::task::spawn_blocking(move || {
        crate::services::lyrics::embed_lyrics(&validated.to_string_lossy(), &text)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn lyrics_save(
    file: String,
    kind: LyricsKind,
    text: String,
    app: AppHandle,
) -> Result<(), String> {
    match require_path_in_output(&file, &app) {
        Ok(validated) => tokio::task::spawn_blocking(move || {
            crate::services::lyrics::save_manual_lyrics(&validated.to_string_lossy(), kind, &text)
        })
        .await
        .map_err(|e| e.to_string())?,
        Err(_) => {
            let user_data = user_data_dir(&app)?;
            crate::services::lyrics::save_path_lyrics(&user_data, &file, &text, kind);
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn lyrics_search(
    query: String,
    track_name: Option<String>,
    artist_name: Option<String>,
) -> Result<Vec<LrcSearchResult>, String> {
    Ok(crate::services::lyrics::search_lyrics(
        &query,
        track_name.as_deref(),
        artist_name.as_deref(),
    )
    .await)
}

#[tauri::command]
pub async fn rpc_update(state: Option<RpcPresence>, app: AppHandle) -> Result<(), String> {
    let enabled = {
        let s = app.state::<AppState>();
        let guard = s.settings.lock().map_err(|e| e.to_string())?;
        guard.as_ref().map(|s| s.discord_rpc).unwrap_or(true)
    };
    if !enabled {
        crate::services::rpc::update_presence(None);
        return Ok(());
    }
    let Some(mut s) = state else {
        crate::services::rpc::update_presence(None);
        return Ok(());
    };

    let has_public_cover = s
        .cover
        .as_deref()
        .is_some_and(crate::services::rpc::is_public_cover);

    if has_public_cover {
        crate::services::rpc::update_presence(Some(&s));
        return Ok(());
    }

    if let Some(cover) = crate::services::artwork::cached(&s.artist, &s.album, &s.title).await {
        s.cover = cover;
        crate::services::rpc::update_presence(Some(&s));
        return Ok(());
    }

    let artist = s.artist.clone();
    let album = s.album.clone();
    let title = s.title.clone();
    let duration = s.duration;
    s.cover = None;
    crate::services::rpc::update_presence(Some(&s));

    tauri::async_runtime::spawn(async move {
        if let Some(cover) =
            crate::services::artwork::lookup(&artist, &album, &title, duration).await
        {
            crate::services::rpc::patch_cover(&artist, &album, &title, cover);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn save_now_playing(state: NowPlayingState, app: AppHandle) -> Result<(), String> {
    let user_data = user_data_dir(&app)?;
    crate::services::queue::save_now_playing(&user_data, &state)
}

#[tauri::command]
pub async fn load_now_playing(app: AppHandle) -> Result<Option<NowPlayingState>, String> {
    let user_data = user_data_dir(&app)?;
    Ok(crate::services::queue::load_now_playing(&user_data))
}

#[tauri::command]
pub async fn clear_now_playing(app: AppHandle) -> Result<(), String> {
    let user_data = user_data_dir(&app)?;
    crate::services::queue::clear_now_playing(&user_data)
}

fn clear_pending_update(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<crate::services::update::UpdateState>();
    let mut guard = state.pending.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}

fn update_error_status(version: Option<String>, error: &str) -> UpdateStatus {
    UpdateStatus {
        available: version.is_some(),
        version,
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        downloading: Some(false),
        progress: None,
        ready: Some(false),
        error: Some(error.to_string()),
    }
}

#[tauri::command]
pub async fn check_app_update(app: AppHandle) -> Result<UpdateStatus, String> {
    use crate::services::update as update_svc;
    match update_svc::check_for_update(&app).await {
        Ok(Some(update)) => {
            if update.version == env!("CARGO_PKG_VERSION") {
                clear_pending_update(&app)?;
                let status = UpdateStatus {
                    available: false,
                    version: None,
                    current_version: env!("CARGO_PKG_VERSION").to_string(),
                    downloading: None,
                    progress: None,
                    ready: None,
                    error: None,
                };
                update_svc::emit_status(&app, &status);
                return Ok(status);
            }
            let status = UpdateStatus {
                available: true,
                version: Some(update.version.clone()),
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                downloading: None,
                progress: None,
                ready: None,
                error: None,
            };
            {
                let state = app.state::<crate::services::update::UpdateState>();
                let mut guard = state.pending.lock().map_err(|e| e.to_string())?;
                *guard = Some(update);
            }
            update_svc::emit_status(&app, &status);
            Ok(status)
        }
        Ok(None) => {
            clear_pending_update(&app)?;
            let status = UpdateStatus {
                available: false,
                version: None,
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                downloading: None,
                progress: None,
                ready: None,
                error: None,
            };
            update_svc::emit_status(&app, &status);
            Ok(status)
        }
        Err(error) => {
            eprintln!("update check failed: {error}");
            let _ = clear_pending_update(&app);
            let status = UpdateStatus {
                available: false,
                version: None,
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                downloading: None,
                progress: None,
                ready: None,
                error: Some(error),
            };
            update_svc::emit_status(&app, &status);
            Ok(status)
        }
    }
}

#[tauri::command]
pub async fn install_app_update(app: AppHandle) -> Result<(), String> {
    use crate::services::update as update_svc;

    let update = {
        let state = app.state::<crate::services::update::UpdateState>();
        let mut guard = state.pending.lock().map_err(|e| e.to_string())?;
        guard.take()
    };

    let update = match update {
        Some(u) => u,
        None => match update_svc::check_for_update(&app).await {
            Ok(Some(u)) => u,
            Ok(None) => {
                let error = "No update available";
                let status = update_error_status(None, error);
                update_svc::emit_status(&app, &status);
                return Err(error.to_string());
            }
            Err(error) => {
                let status = update_error_status(None, &error);
                update_svc::emit_status(&app, &status);
                return Err(error);
            }
        },
    };

    let version = update.version.clone();

    let downloading_status = UpdateStatus {
        available: true,
        version: Some(version.clone()),
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        downloading: Some(true),
        progress: Some(0.0),
        ready: None,
        error: None,
    };
    update_svc::emit_status(&app, &downloading_status);

    if let Err(e) = update_svc::download_and_install(&app, update).await {
        let error_status = UpdateStatus {
            available: true,
            version: Some(version),
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            downloading: Some(false),
            progress: None,
            ready: None,
            error: Some(e.clone()),
        };
        update_svc::emit_status(&app, &error_status);
        return Err(e);
    }

    Ok(())
}

#[tauri::command]
pub async fn restart_app(app: AppHandle) -> Result<(), String> {
    app.restart();
}

#[tauri::command]
pub fn media_update(
    state: Option<MediaUpdate>,
    controller: tauri::State<'_, crate::services::media_controls::MediaController>,
) {
    controller.update(state);
}

#[tauri::command]
pub async fn player_play(
    path: String,
    seek_to: Option<f64>,
    generation: u64,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || crate::services::player::play(&path, seek_to, generation))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn player_crossfade(path: String, generation: u64, fade_secs: f64) -> Result<(), String> {
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

#[tauri::command]
pub async fn clear_cache(app: AppHandle) -> Result<u64, String> {
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
