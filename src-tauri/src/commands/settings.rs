use tauri::{AppHandle, Manager};

use super::playlists::sync_playlist;
use super::types::*;
use super::{AppState, output_dir_from_settings, user_data_dir};

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
    crate::services::mobile_sync::refresh_output_root(&user_data).await;
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

pub fn mirror_taste_to_liked(
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
