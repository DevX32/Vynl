use tauri::{AppHandle, Manager};

use super::types::*;
use super::{
    AppState, ensure_collection_size, load_settings, output_dir_from_settings, user_data_dir,
};

#[tauri::command]
pub async fn sync_done(collection: Collection, app: AppHandle) -> Result<Vec<String>, String> {
    ensure_collection_size(&collection)?;
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
        Err(e) if e.contains(crate::services::sync::NO_REMOTE_STATE) => {
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
            Err(e) if e.contains(crate::services::sync::NO_REMOTE_STATE) => {}
            Err(e) => {
                log::warn!("[Vynl sync] auto pull after sign-in failed: {e}");
                crate::services::sync::record_sync_error(&user_data, &e);
                can_push = false;
            }
        }
        if can_push
            && let Err(e) =
                crate::services::sync::push_sync_state(&user_data, &output, &settings).await
        {
            log::warn!("[Vynl sync] auto push after sign-in failed: {e}");
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
