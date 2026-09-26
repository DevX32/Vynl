pub mod artist;
pub mod plugins;
pub mod types;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

use types::*;

const MAX_COLLECTION_TRACKS: usize = 10_000;

pub struct AppState {
    pub settings: Mutex<Option<Settings>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            settings: Mutex::new(None),
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
    Ok(guard
        .as_ref()
        .map(|s| PathBuf::from(&s.output_dir))
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(dirs::audio_dir)
        .unwrap_or_else(|| PathBuf::from(".")))
}

/// Canonicalises `file`, tolerating a not-yet-existing final component, and
/// rejects it unless it resolves inside `canonical_output`.
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

/// Canonical output directory, used as the containment root for every command
/// that touches user files.
fn canonical_output_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let output = output_dir_from_settings(app)?;
    Ok(fs::canonicalize(&output).unwrap_or(output))
}

fn validate_path_in_output(file: &str, app: &AppHandle) -> Result<PathBuf, String> {
    canonicalize_in_output(file, &canonical_output_dir(app)?)
}

/// [`validate_path_in_output`] plus the "the file must be there" check that
/// every read/modify command performs right after it.
fn require_path_in_output(file: &str, app: &AppHandle) -> Result<PathBuf, String> {
    let path = validate_path_in_output(file, app)?;
    if !path.exists() {
        return Err("File does not exist".into());
    }
    Ok(path)
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
pub async fn resolve(url: String) -> Result<Collection, String> {
    crate::services::downloader::resolve_link(&url).await
}

#[tauri::command]
pub async fn search_spotify(
    query: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<SpotifySearchResult>, String> {
    let l = limit.unwrap_or(20);
    let o = offset.unwrap_or(0);
    crate::services::internal::search_spotify(&query, l, o).await
}

#[tauri::command]
pub async fn downloaded_paths(
    collection: Collection,
    app: AppHandle,
) -> Result<Vec<String>, String> {
    let settings = load_settings(&app)?;
    Ok(collection
        .tracks
        .iter()
        .filter_map(|t| {
            let base = crate::services::downloader::sanitize(
                &crate::services::downloader::render_pattern(&settings.filename_pattern, t),
            );
            let path = std::path::PathBuf::from(&settings.output_dir).join(format!(
                "{}.{}",
                base,
                crate::services::downloader::audio_format_ext(&settings.format)
            ));
            if path.is_file() {
                Some(path.to_string_lossy().to_string())
            } else {
                None
            }
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
    let changed = {
        let state = app.state::<AppState>();
        let mut guard = state.settings.lock().map_err(|e| e.to_string())?;
        let changed = guard.as_ref().map(|s| s.launch_at_startup) != Some(next.launch_at_startup);
        *guard = Some(next.clone());
        changed
    };
    if changed {
        crate::sync_launch_at_startup(next.launch_at_startup);
    }

    crate::services::player::set_eq(next.eq_enabled, &next.eq_bands);
    Ok(next)
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
pub async fn update_tool(name: String, app: AppHandle) -> Result<(), String> {
    let valid_names = ["yt-dlp", "ffmpeg"];
    if !valid_names.contains(&name.as_str()) {
        return Err("Unknown tool".into());
    }
    let user_data = user_data_dir(&app)?;
    let exe = if cfg!(target_os = "windows") {
        format!("{}.exe", name)
    } else {
        name.clone()
    };
    let bin_path = user_data.join("bin").join(&exe);
    fs::remove_file(&bin_path).ok();
    crate::services::tools::install_tool(&user_data, &name, app).await
}

#[tauri::command]
pub async fn delete_library_track(path: String, app: AppHandle) -> Result<(), String> {
    let canonical_path = require_path_in_output(&path, &app)?;
    fs::remove_file(&canonical_path).map_err(|e| format!("Failed to delete file: {e}"))?;
    for sidecar_ext in &["lrc", "txt"] {
        let sidecar =
            crate::services::lyrics::sidecar_path(&canonical_path.to_string_lossy(), sidecar_ext);
        let _ = fs::remove_file(sidecar);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_library(app: AppHandle) -> Result<Vec<LibraryTrack>, String> {
    let user_data = user_data_dir(&app)?;
    let output = output_dir_from_settings(&app)?;

    if let Some(cached) = crate::services::library::load_cache(&user_data) {
        let app_clone = app.clone();
        let baseline = cached.clone();
        tokio::task::spawn_blocking(move || {
            let ud = user_data_dir(&app_clone).ok();
            let od = output_dir_from_settings(&app_clone).ok();
            if let (Some(ud), Some(od)) = (ud, od) {
                let result = crate::services::library::scan_cached_library(&od, &ud);
                if result != baseline {
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
    Ok(crate::services::playlists::list_playlists(&output))
}

#[tauri::command]
pub async fn get_playlist(id: String, app: AppHandle) -> Result<Option<Playlist>, String> {
    let output = output_dir_from_settings(&app)?;
    Ok(crate::services::playlists::get_playlist(&output, &id))
}

#[tauri::command]
pub async fn create_playlist(name: String, app: AppHandle) -> Result<Playlist, String> {
    let output = output_dir_from_settings(&app)?;
    crate::services::playlists::create_playlist(&output, &name)
}

#[tauri::command]
pub async fn rename_playlist(id: String, name: String, app: AppHandle) -> Result<Playlist, String> {
    let output = output_dir_from_settings(&app)?;
    crate::services::playlists::rename_playlist(&output, &id, &name)
}

#[tauri::command]
pub async fn delete_playlist(id: String, app: AppHandle) -> Result<(), String> {
    let output = output_dir_from_settings(&app)?;
    crate::services::playlists::delete_playlist(&output, &id);
    Ok(())
}

#[tauri::command]
pub async fn add_to_playlist(
    id: String,
    paths: Vec<String>,
    app: AppHandle,
) -> Result<Playlist, String> {
    let output = output_dir_from_settings(&app)?;
    let validated_paths: Vec<String> = paths
        .iter()
        .map(|p| require_path_in_output(p, &app).map(|c| c.to_string_lossy().to_string()))
        .collect::<Result<Vec<String>, String>>()?;
    crate::services::playlists::add_to_playlist(&output, &id, &validated_paths)
}

#[tauri::command]
pub async fn remove_from_playlist(
    id: String,
    track_path: String,
    app: AppHandle,
) -> Result<Playlist, String> {
    let output = output_dir_from_settings(&app)?;
    crate::services::playlists::remove_from_playlist(&output, &id, &track_path)
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
    let output = output_dir_from_settings(&app)?;
    let output_clone = output.clone();
    let playlist =
        crate::services::playlists::get_playlist(&output, &id).ok_or("playlist not found")?;
    let len = playlist.paths.len() as i64;
    if from >= len {
        return Err("from index out of bounds".into());
    }
    let to = to.min(len - 1);
    crate::services::playlists::move_in_playlist(&output_clone, &id, from as usize, to as usize)
}

#[tauri::command]
pub async fn set_playlist_cover(
    id: String,
    cover: Option<String>,
    app: AppHandle,
) -> Result<Playlist, String> {
    let output = output_dir_from_settings(&app)?;
    crate::services::playlists::set_playlist_cover(&output, &id, cover)
}

#[tauri::command]
pub async fn save_playlist_cover(
    id: String,
    ext: String,
    data: Vec<u8>,
    app: AppHandle,
) -> Result<String, String> {
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
    let covers_dir = user_data.join("covers");
    fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;
    let filename = format!("pl-{}.{}", id, ext);
    let path = covers_dir.join(&filename);
    fs::write(&path, &data).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn lyrics_local(file: String, app: AppHandle) -> Result<Option<LyricsResult>, String> {
    let validated = require_path_in_output(&file, &app)?;
    Ok(crate::services::lyrics::local_lyrics(
        &validated.to_string_lossy(),
    ))
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
    let user_data = user_data_dir(&app)?;
    let ffmpeg = crate::services::tools::get_tool_path("ffmpeg", &user_data);
    tokio::task::spawn_blocking(move || {
        crate::services::lyrics::embed_lyrics(
            &validated.to_string_lossy(),
            &text,
            ffmpeg.as_deref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
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
pub async fn lyrics_export(
    content: String,
    default_name: String,
    app: AppHandle,
) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Save Lyrics")
        .set_file_name(&default_name)
        .save_file(move |path| {
            let _ = tx.send(path);
        });

    let path = rx.await.map_err(|e| e.to_string())?;

    match path {
        Some(fp) => {
            let p = fp.into_path().map_err(|e| e.to_string())?;
            let path_str = p.to_string_lossy().to_string();
            tokio::fs::write(&p, &content)
                .await
                .map_err(|e| e.to_string())?;
            Ok(Some(path_str))
        }
        None => Ok(None),
    }
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
    let state = match state {
        Some(mut s) => {
            let has_public_cover = s
                .cover
                .as_deref()
                .is_some_and(|c| c.starts_with("http://") || c.starts_with("https://"));
            if !has_public_cover {
                s.cover = crate::services::artwork::lookup(&s.artist, &s.title).await;
            }
            Some(s)
        }
        None => None,
    };

    crate::services::rpc::update_presence(state.as_ref());
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
            update_svc::emit_status(&app, &status).await;
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
            update_svc::emit_status(&app, &status).await;
            Ok(status)
        }
        Err(error) => {
            eprintln!("update check failed: {error}");
            let _ = clear_pending_update(&app);
            Ok(UpdateStatus {
                available: false,
                version: None,
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                downloading: None,
                progress: None,
                ready: None,
                error: None,
            })
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
                update_svc::emit_status(&app, &status).await;
                return Err(error.to_string());
            }
            Err(error) => {
                let status = update_error_status(None, &error);
                update_svc::emit_status(&app, &status).await;
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
    update_svc::emit_status(&app, &downloading_status).await;

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
        update_svc::emit_status(&app, &error_status).await;
        return Err(e);
    }

    Ok(())
}

#[tauri::command]
pub async fn restart_app(app: AppHandle) -> Result<(), String> {
    app.restart();
}

#[tauri::command]
pub async fn player_play(
    path: String,
    seek_to: Option<f64>,
    generation: u64,
) -> Result<(), String> {
    crate::services::player::play(&path, seek_to, generation)
}

#[tauri::command]
pub async fn player_stop(generation: u64) -> Result<(), String> {
    crate::services::player::stop(generation)
}

#[tauri::command]
pub async fn player_pause(generation: u64) -> Result<(), String> {
    crate::services::player::pause(generation)
}

#[tauri::command]
pub async fn player_resume(generation: u64) -> Result<bool, String> {
    crate::services::player::resume(generation)
}

#[tauri::command]
pub async fn player_seek(time: f64, generation: u64) -> Result<(), String> {
    crate::services::player::seek(time, generation)
}

#[tauri::command]
pub async fn player_set_volume(vol: u32) -> Result<(), String> {
    crate::services::player::set_volume(vol);
    Ok(())
}

#[tauri::command]
pub async fn player_get_position(generation: u64) -> Result<f64, String> {
    crate::services::player::get_position(generation)
}

#[tauri::command]
pub async fn player_is_playing(generation: u64) -> Result<bool, String> {
    crate::services::player::is_playing(generation)
}

#[tauri::command]
pub async fn player_check_finished(generation: u64) -> Result<bool, String> {
    crate::services::player::check_finished(generation)
}

#[tauri::command]
pub async fn cache_remote_audio(url: String, key: String) -> Result<String, String> {
    crate::services::audio_cache::cache_remote_audio(&url, &key).await
}

#[tauri::command]
pub async fn clear_audio_cache() -> Result<(), String> {
    crate::services::audio_cache::clear_cache()
}
