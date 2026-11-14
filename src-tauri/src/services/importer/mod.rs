use std::collections::HashSet;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::commands::types::*;
use crate::services::{cover_cache, downloader, playlists};

const MAX_IMPORT_TRACKS: usize = 1000;

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
    if lower.contains("youtu.be/")
        || lower.contains("/shorts/")
        || lower.contains("/embed/")
        || lower.contains("/live/")
        || lower.contains("/v/")
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

#[tauri::command]
pub async fn import_stream_playlist(
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
    if collection.tracks.len() > MAX_IMPORT_TRACKS {
        return Err(format!(
            "That playlist is too large to import (max {MAX_IMPORT_TRACKS} tracks)."
        ));
    }

    let output = crate::commands::output_dir_from_settings(&app)?;
    let user_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;

    let matches = downloader::find_matches(&collection, &user_data).await?;

    let mut picked: Vec<CatalogueTrack> = Vec::with_capacity(collection.tracks.len());
    let mut unmatched: Vec<UnmatchedImport> = Vec::new();
    let mut seen_video_ids: HashSet<String> = HashSet::with_capacity(collection.tracks.len());

    for track in &collection.tracks {
        let candidates = matches.get(&track.id).cloned().unwrap_or_default();
        let ranked = downloader::filter_and_rank_candidates(candidates, track);
        let video_id = ranked.first().and_then(|c| extract_video_id(&c.url));
        match video_id {
            Some(id) if seen_video_ids.insert(id.clone()) => {
                picked.push(stream_track_for(track, id));
            }
            _ => unmatched.push(UnmatchedImport {
                title: track.title.clone(),
                artist: track.artist.clone(),
            }),
        }
    }

    if picked.is_empty() {
        return Err("None of the tracks could be matched to a streamable source.".into());
    }

    let matched = picked.len() as u64;
    let total = collection.tracks.len() as u64;

    let requested_id = playlist_id
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty());
    let mut created_new = false;
    let mut playlist = match requested_id {
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

    playlist = playlists::add_streams_to_playlist(&output, &playlist.id, &picked)?;

    if created_new
        && let Some(cover_url) = collection.cover.as_deref().map(str::trim).filter(|u| !u.is_empty())
        && let Ok(local) = cover_cache::cache_cover(&user_data, cover_url).await
    {
        match playlists::set_playlist_cover(&output, &user_data, &playlist.id, Some(local)) {
            Ok(updated) => playlist = updated,
            Err(error) => {
                eprintln!("[Vynl] could not set the imported playlist cover: {error}")
            }
        }
    }

    crate::commands::mirror_playlist(&app, &playlist);

    Ok(StreamImportResult {
        playlist,
        matched,
        total,
        unmatched,
    })
}

#[cfg(test)]
mod tests {
    use super::extract_video_id;

    #[test]
    fn parses_watch_urls() {
        assert_eq!(
            extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ").as_deref(),
            Some("dQw4w9WgXcQ")
        );
        assert_eq!(
            extract_video_id("https://music.youtube.com/watch?v=dQw4w9WgXcQ&list=RDAMVM")
                .as_deref(),
            Some("dQw4w9WgXcQ")
        );
        assert_eq!(
            extract_video_id("https://www.youtube.com/watch?time_continue=12&v=dQw4w9WgXcQ&feature=share")
                .as_deref(),
            Some("dQw4w9WgXcQ")
        );
    }

    #[test]
    fn parses_short_and_embed_urls() {
        assert_eq!(
            extract_video_id("https://youtu.be/dQw4w9WgXcQ?si=abc123").as_deref(),
            Some("dQw4w9WgXcQ")
        );
        assert_eq!(
            extract_video_id("https://www.youtube.com/shorts/dQw4w9WgXcQ").as_deref(),
            Some("dQw4w9WgXcQ")
        );
        assert_eq!(
            extract_video_id("dQw4w9WgXcQ").as_deref(),
            Some("dQw4w9WgXcQ")
        );
    }

    #[test]
    fn rejects_junk() {
        assert!(extract_video_id("").is_none());
        assert!(extract_video_id("https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC").is_none());
        assert!(extract_video_id("https://www.youtube.com/watch?v=%&*").is_none());
        assert!(extract_video_id("https://www.youtube.com/watch").is_none());
    }
}
