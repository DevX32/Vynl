use tauri::AppHandle;

use super::types::*;
use super::{require_path_in_output, user_data_dir};

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
    force: Option<bool>,
    app: AppHandle,
) -> Result<Option<LyricsResult>, String> {
    let user_data = user_data_dir(&app)?;
    crate::services::lyrics::fetch_lyrics_with_cache(&lookup, &user_data, force.unwrap_or(false))
        .await
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
