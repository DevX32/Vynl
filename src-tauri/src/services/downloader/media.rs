use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::Emitter;
use tokio::sync::Semaphore;

use crate::commands::types::*;
use crate::services::{audio, util as svcutil};

use super::{CANCELLED_DOWNLOAD, COVER_PREFETCH_WORKERS, MAX_COVER_BYTES, UA, sanitize_key_component};

pub(super) type CoverMap = Arc<std::sync::Mutex<HashMap<String, Option<PathBuf>>>>;

pub(super) async fn probe_duration(file: &Path) -> Option<f64> {
    let path = file.to_path_buf();
    tokio::task::spawn_blocking(move || audio::duration_secs(&path))
        .await
        .ok()
        .flatten()
}


pub(super) async fn fetch_cover(url: &str, dest: &str) -> bool {
    let url = crate::services::cover_cache::normalize_cover_url(url).into_owned();
    if !url.starts_with("https://") {
        return false;
    }

    static HTTP: once_cell::sync::Lazy<reqwest::Client> = once_cell::sync::Lazy::new(|| {
        reqwest::Client::builder()
            .user_agent(UA)
            .timeout(std::time::Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_default()
    });

    let resp = match svcutil::follow_public_https(&HTTP, &url, 5).await {
        Ok(r) => r,
        Err(_) => return false,
    };

    if !resp.status().is_success() {
        return false;
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    if !content_type.starts_with("image/") {
        return false;
    }

    let mut resp = resp;
    let mut buf: Vec<u8> = Vec::new();
    loop {
        match tokio::time::timeout(std::time::Duration::from_secs(30), resp.chunk()).await {
            Ok(Ok(Some(chunk))) => {
                buf.extend_from_slice(&chunk);
                if buf.len() > MAX_COVER_BYTES {
                    return false;
                }
            }
            Ok(Ok(None)) => break,
            Ok(Err(_)) => return false,
            Err(_) => return false,
        }
    }

    tokio::fs::write(dest, &buf).await.is_ok()
}

pub(super) fn emit_track(
    app: &tauri::AppHandle,
    t: &TrackMeta,
    status: TrackStatus,
    percent: f64,
    message: Option<&str>,
) {
    let payload = TrackProgress {
        track_id: t.id.clone(),
        status,
        percent,
        message: message.map(|m| m.to_string()),
    };
    let event = DownloadEvent::Track { payload };
    let _ = app.emit("vynl:dl", &event);
}


pub(super) fn start_cover_prefetch(
    collection: &Collection,
    tmp: &Path,
) -> (CoverMap, tokio::task::JoinHandle<()>) {
    let covers: CoverMap = Arc::new(std::sync::Mutex::new(HashMap::new()));
    let semaphore = Arc::new(Semaphore::new(COVER_PREFETCH_WORKERS));
    let mut handles = Vec::new();

    for track in &collection.tracks {
        let cover_url = track.cover.as_ref().or(collection.cover.as_ref()).cloned();
        let track_id = track.id.clone();
        let tmp = tmp.to_path_buf();
        let covers = Arc::clone(&covers);
        let semaphore = Arc::clone(&semaphore);

        if let Some(url) = cover_url {
            handles.push(tokio::spawn(async move {
                if CANCELLED_DOWNLOAD.load(Ordering::Relaxed) {
                    return;
                }
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(_) => return,
                };
                if CANCELLED_DOWNLOAD.load(Ordering::Relaxed) {
                    return;
                }
                let cp = tmp.join(format!("cover-{}.jpg", sanitize_key_component(&track_id)));
                let _ = std::fs::remove_file(&cp);
                let success = fetch_cover(&url, &cp.to_string_lossy()).await;
                let mut map = covers.lock().unwrap_or_else(|e| e.into_inner());
                if success {
                    map.insert(track_id, Some(cp));
                } else {
                    let _ = std::fs::remove_file(&cp);
                    map.insert(track_id, None);
                }
            }));
        } else {
            let mut map = covers.lock().unwrap_or_else(|e| e.into_inner());
            map.insert(track_id, None);
        }
    }

    let joiner = tokio::spawn(async move {
        for handle in handles {
            let _ = handle.await;
        }
    });

    (covers, joiner)
}


pub(super) async fn fetch_cover_now(
    track: &TrackMeta,
    collection: &Collection,
    tmp: &Path,
) -> Option<PathBuf> {
    let url = track.cover.as_ref().or(collection.cover.as_ref())?;
    let cp = tmp.join(format!(
        "cover-late-{}.jpg",
        sanitize_key_component(&track.id)
    ));
    if fetch_cover(url, &cp.to_string_lossy()).await {
        Some(cp)
    } else {
        let _ = std::fs::remove_file(&cp);
        None
    }
}

