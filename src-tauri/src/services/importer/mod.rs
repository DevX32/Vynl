use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::commands::types::*;
use crate::services::{cover_cache, downloader, playlists};

static IMPORT_BUSY: AtomicBool = AtomicBool::new(false);

const IMPORT_BUSY_MESSAGE: &str = "An import is already running.";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnmatchedImport {
    pub title: String,
    pub artist: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamImportResult {
    pub playlist: Playlist,
    pub matched: u64,
    pub total: u64,
    pub unmatched: Vec<UnmatchedImport>,
}

fn valid_video_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 20
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn extract_video_id(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }

    for segment in trimmed.split(['?', '&', '#']) {
        if let Some(id) = segment.strip_prefix("v=")
            && valid_video_id(id)
        {
            return Some(id.to_string());
        }
    }

    let path = trimmed.split(['?', '#']).next().unwrap_or("");
    let lower = trimmed.to_lowercase();
    let is_youtube = lower.starts_with("https://www.youtube.com/")
        || lower.starts_with("http://www.youtube.com/")
        || lower.starts_with("https://youtube.com/")
        || lower.starts_with("http://youtube.com/")
        || lower.starts_with("https://m.youtube.com/")
        || lower.starts_with("https://youtu.be/")
        || lower.starts_with("http://youtu.be/");
    if is_youtube
        && (lower.contains("/shorts/")
            || lower.contains("/embed/")
            || lower.contains("/live/")
            || lower.contains("/v/"))
    {
        let last = path.rsplit('/').next().unwrap_or("");
        if valid_video_id(last) {
            return Some(last.to_string());
        }
    }

    if valid_video_id(trimmed) {
        return Some(trimmed.to_string());
    }

    None
}

fn stream_track_for(track: &TrackMeta, video_id: String) -> CatalogueTrack {
    let album = track.album.trim();
    CatalogueTrack {
        id: video_id,
        title: track.title.clone(),
        artist: track.artist.clone(),
        artists: vec![CatalogueArtistRef {
            name: track.artist.clone(),
            id: None,
        }],
        album: if album.is_empty() {
            None
        } else {
            Some(CatalogueAlbumRef {
                name: album.to_string(),
                id: None,
            })
        },
        duration: track.duration,
        cover: track.cover.clone(),
        explicit: false,
    }
}

pub async fn import_stream_playlist(
    collection: Collection,
    playlist_id: Option<String>,
    playlist_name: Option<String>,
    app: AppHandle,
) -> Result<StreamImportResult, String> {
    if collection.tracks.len() > crate::commands::MAX_COLLECTION_TRACKS {
        return Err(format!(
            "this collection is too large to import (max {} tracks)",
            crate::commands::MAX_COLLECTION_TRACKS
        ));
    }
    if IMPORT_BUSY
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(IMPORT_BUSY_MESSAGE.into());
    }
    struct BusyGuard;
    impl Drop for BusyGuard {
        fn drop(&mut self) {
            IMPORT_BUSY.store(false, Ordering::SeqCst);
        }
    }
    let _busy_guard = BusyGuard;
    let generation = crate::services::progress::set_emitter(Some(app.clone()));
    let result = do_import(collection, playlist_id, playlist_name, app).await;
    crate::services::progress::clear_emitter(generation);
    result
}

async fn do_import(
    collection: Collection,
    playlist_id: Option<String>,
    playlist_name: Option<String>,
    app: AppHandle,
) -> Result<StreamImportResult, String> {
    if collection.kind != CollectionKind::Playlist {
        return Err("That link is not a supported playlist.".into());
    }
    if collection.tracks.is_empty() {
        return Err("That playlist has no tracks.".into());
    }

    let output = crate::commands::output_dir_from_settings(&app)?;
    let user_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;

    let direct_picks = collection
        .tracks
        .iter()
        .all(|track| extract_video_id(&track.url).is_some());

    let mut picked: Vec<CatalogueTrack> = Vec::with_capacity(collection.tracks.len());
    let mut unmatched: Vec<UnmatchedImport> = Vec::new();
    let mut seen_video_ids: HashSet<String> = HashSet::with_capacity(collection.tracks.len());

    if direct_picks {
        for (index, track) in collection.tracks.iter().enumerate() {
            let video_id = extract_video_id(&track.url);
            match video_id {
                Some(id) => {
                    if seen_video_ids.insert(id.clone()) {
                        picked.push(stream_track_for(track, id));
                    }
                }
                _ => unmatched.push(UnmatchedImport {
                    title: track.title.clone(),
                    artist: track.artist.clone(),
                }),
            }
            crate::services::progress::emit("match", index + 1, collection.tracks.len());
        }
    } else {
        let matches = downloader::find_matches(&collection, &user_data).await?;
        if downloader::find_cancelled() {
            return Err("Import cancelled.".into());
        }
        for track in &collection.tracks {
            let candidates = matches.get(&track.id).cloned().unwrap_or_default();
            let ranked = downloader::filter_and_rank_candidates(candidates, track);
            let video_id = ranked.first().and_then(|c| extract_video_id(&c.url));
            match video_id {
                Some(id) => {
                    if seen_video_ids.insert(id.clone()) {
                        picked.push(stream_track_for(track, id));
                    }
                }
                _ => unmatched.push(UnmatchedImport {
                    title: track.title.clone(),
                    artist: track.artist.clone(),
                }),
            }
        }
    }

    if picked.is_empty() {
        return Err("None of the tracks could be matched to a streamable source.".into());
    }

    let matched = picked.len() as u64;
    let total = matched + unmatched.len() as u64;

    crate::services::progress::emit("save", 1, 3);

    let requested_id = playlist_id
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty());
    if requested_id.as_deref() == Some(crate::services::sync::LIKED_PLAYLIST_ID) {
        return Err("the Liked Songs playlist is managed automatically".into());
    }
    let mut created_new = false;
    let playlist = match requested_id {
        Some(id) => {
            if !playlists::validate_id(&id) {
                return Err("invalid playlist id".into());
            }
            playlists::get_playlist(&output, &id)
                .ok_or_else(|| "Playlist not found.".to_string())?
        }
        None => {
            let title = playlist_name
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty())
                .or_else(|| {
                    let title = collection.title.trim().to_string();
                    (!title.is_empty()).then_some(title)
                })
                .unwrap_or_else(|| "Playlist".to_string());
            created_new = true;
            playlists::create_playlist(&output, &title)?
        }
    };

    let mut playlist = match playlists::add_streams_to_playlist(&output, &playlist.id, &picked) {
        Ok(updated) => updated,
        Err(error) => {
            if created_new {
                let _ = playlists::delete_playlist(&output, &user_data, &playlist.id);
            }
            return Err(error);
        }
    };

    crate::services::progress::emit("save", 2, 3);

    if created_new
        && let Some(cover_url) = collection
            .cover
            .as_deref()
            .map(str::trim)
            .filter(|u| !u.is_empty())
        && let Ok(local) = cover_cache::cache_cover(&user_data, cover_url).await
    {
        match playlists::save_imported_cover(
            &output,
            &user_data,
            &playlist.id,
            std::path::Path::new(&local),
        ) {
            Ok(updated) => playlist = updated,
            Err(error) => {
                log::error!("[Vynl] could not set the imported playlist cover: {error}")
            }
        }
    }

    crate::services::progress::emit("save", 3, 3);

    crate::commands::sync_playlist(&app, &playlist);

    Ok(StreamImportResult {
        playlist,
        matched,
        total,
        unmatched,
    })
}
