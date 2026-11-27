use std::collections::HashMap;

use tauri::AppHandle;

use super::types::*;
use super::{ensure_collection_size, get_user_data_and_settings, load_settings, user_data_dir};

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
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    Ok(collection
        .tracks
        .iter()
        .filter_map(|t| {
            let path = crate::services::downloader::final_output_path(t, &settings);
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
    ensure_collection_size(&collection)?;
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
    ensure_collection_size(&collection)?;
    let user_data = user_data_dir(&app)?;
    crate::services::downloader::find_matches(&collection, &user_data).await
}

#[tauri::command]
pub async fn cancel_stream_import() {
    crate::services::downloader::cancel_find();
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
pub async fn catalogue_search(
    query: String,
) -> Result<crate::catalogue::CatalogueSearchResults, String> {
    crate::services::catalogue::catalogue_search(query).await
}

#[tauri::command]
pub async fn catalogue_cover(app: AppHandle, url: String) -> Result<String, String> {
    crate::services::catalogue::catalogue_cover(app, url).await
}

#[tauri::command]
pub async fn catalogue_album(id: String) -> Result<crate::catalogue::CatalogueAlbumDetail, String> {
    crate::services::catalogue::catalogue_album(id).await
}

#[tauri::command]
pub async fn catalogue_artist(id: String) -> Result<crate::catalogue::CatalogueArtist, String> {
    crate::services::catalogue::catalogue_artist(id).await
}

#[tauri::command]
pub async fn catalogue_playlist(
    id: String,
) -> Result<crate::catalogue::CataloguePlaylistDetail, String> {
    crate::services::catalogue::catalogue_playlist(id).await
}

#[tauri::command]
pub async fn catalogue_playlist_page(
    token: String,
) -> Result<crate::catalogue::CataloguePlaylistPage, String> {
    crate::services::catalogue::catalogue_playlist_page(token).await
}

#[tauri::command]
pub async fn catalogue_prebuffer(
    video_id: String,
    duration: Option<f64>,
) -> Result<CatalogueStreamInfo, String> {
    crate::services::catalogue::catalogue_prebuffer(video_id, duration).await
}

#[tauri::command]
pub async fn catalogue_discard_prebuffer(video_id: String) -> Result<(), String> {
    crate::services::catalogue::catalogue_discard_prebuffer(video_id).await
}

#[tauri::command]
pub async fn catalogue_stream(
    video_id: String,
    start: Option<f64>,
    duration: Option<f64>,
    generation: u64,
) -> Result<CatalogueStreamInfo, String> {
    crate::services::catalogue::catalogue_stream(video_id, start, duration, generation).await
}

#[tauri::command]
pub async fn catalogue_crossfade_stream(
    video_id: String,
    _duration: Option<f64>,
    generation: u64,
    fade_secs: f64,
) -> Result<(), String> {
    crate::services::catalogue::catalogue_crossfade_stream(
        video_id, _duration, generation, fade_secs,
    )
    .await
}

#[tauri::command]
pub async fn catalogue_seek_stream(
    video_id: String,
    time: f64,
    duration: Option<f64>,
    generation: u64,
    playing: bool,
    request_id: u64,
) -> Result<(), String> {
    crate::services::catalogue::catalogue_seek_stream(
        video_id, time, duration, generation, playing, request_id,
    )
    .await
}

#[tauri::command]
pub async fn catalogue_taste(
    seeds: Vec<String>,
) -> Result<Vec<crate::catalogue::CatalogueTrack>, String> {
    crate::services::catalogue::catalogue_taste(seeds).await
}

#[tauri::command]
pub async fn catalogue_save_to_playlist(
    track: crate::catalogue::CatalogueTrack,
    playlist_id: Option<String>,
    playlist_name: Option<String>,
    app: AppHandle,
) -> Result<Playlist, String> {
    crate::services::catalogue::catalogue_save_to_playlist(track, playlist_id, playlist_name, app)
        .await
}

#[tauri::command]
pub async fn catalogue_add_to_playlist(
    track: crate::catalogue::CatalogueTrack,
    playlist_id: Option<String>,
    playlist_name: Option<String>,
    app: AppHandle,
) -> Result<CatalogueAddResult, String> {
    crate::services::catalogue::catalogue_add_to_playlist(track, playlist_id, playlist_name, app)
        .await
}

#[tauri::command]
pub async fn import_stream_playlist(
    collection: Collection,
    playlist_id: Option<String>,
    playlist_name: Option<String>,
    app: AppHandle,
) -> Result<crate::services::importer::StreamImportResult, String> {
    crate::services::importer::import_stream_playlist(collection, playlist_id, playlist_name, app)
        .await
}
