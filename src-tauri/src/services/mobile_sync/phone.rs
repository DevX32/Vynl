use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::commands::types::{
    CatalogueAlbumDetail, CatalogueArtist, CataloguePlaylistDetail, CatalogueSearchResults,
    MediaControlAction,
};
use crate::services::catalogue;
use crate::services::mobile_sync::server::SharedState;
use crate::services::player;

const CATALOGUE_PREFIX: &str = "ytm:";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MobileTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Option<f64>,
    pub cover: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MobileNowPlaying {
    #[serde(default)]
    pub track: Option<MobileTrack>,
    #[serde(default)]
    pub queue: Vec<MobileTrack>,
}

static NOW_PLAYING: Lazy<tokio::sync::RwLock<Option<MobileNowPlaying>>> =
    Lazy::new(|| tokio::sync::RwLock::new(None));

pub async fn set_now_playing(state: MobileNowPlaying) {
    let mut guard = NOW_PLAYING.write().await;
    *guard = Some(state);
}

pub async fn now_playing() -> MobileNowPlaying {
    NOW_PLAYING.read().await.clone().unwrap_or_default()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobilePlayerState {
    pub playing: bool,
    pub position: f64,
    pub duration: Option<f64>,
    pub volume: u32,
    pub track: Option<MobileTrack>,
    pub queue: Vec<MobileTrack>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobilePlayerAction {
    pub action: String,
    pub time: Option<f64>,
    pub volume: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileError {
    pub error: String,
}

fn error(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(MobileError {
            error: message.into(),
        }),
    )
        .into_response()
}

fn clean(message: String) -> String {
    let trimmed = catalogue::sanitize_error(&message);
    if trimmed.is_empty() {
        "The request failed.".to_string()
    } else {
        trimmed
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileTaste {
    pub liked: Vec<MobileTrack>,
    pub summary: crate::services::taste::TasteSummary,
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/v1/catalogue/search", get(catalogue_search))
        .route("/v1/catalogue/album/{id}", get(catalogue_album))
        .route("/v1/catalogue/artist/{id}", get(catalogue_artist))
        .route("/v1/catalogue/playlist/{id}", get(catalogue_playlist))
        .route("/v1/stream/{id}/audio", get(stream_audio))
        .route("/v1/player", get(player_state))
        .route("/v1/player/action", post(player_action))
        .route("/v1/taste", get(taste_state))
}

async fn taste_state(State(state): State<SharedState>) -> Response {
    let user_data = state.user_data.clone();
    let entries = tokio::task::spawn_blocking(move || crate::services::taste::load(&user_data))
        .await
        .unwrap_or_default();

    let liked = crate::services::taste::liked(&entries)
        .into_iter()
        .map(|entry| MobileTrack {
            id: entry.track.id.clone(),
            title: entry.track.title.clone(),
            artist: entry.track.artist.clone(),
            album: entry.track.album.as_ref().map(|a| a.name.clone()),
            duration: entry.track.duration,
            cover: entry.track.cover.clone(),
        })
        .collect();

    Json(MobileTaste {
        liked,
        summary: crate::services::taste::summary(&entries),
    })
    .into_response()
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

async fn catalogue_search(
    State(_state): State<SharedState>,
    Query(query): Query<SearchQuery>,
) -> Response {
    let trimmed = query.q.trim();
    if trimmed.is_empty() {
        return error(StatusCode::BAD_REQUEST, "Enter something to search for.");
    }
    match catalogue::catalogue_search(trimmed.to_string()).await {
        Ok(results) => Json(CatalogueSearchResults {
            songs: results.songs,
            albums: results.albums,
            playlists: results.playlists,
        })
        .into_response(),
        Err(message) => error(StatusCode::BAD_GATEWAY, clean(message)),
    }
}

async fn catalogue_album(
    State(_state): State<SharedState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    match catalogue::catalogue_album(id).await {
        Ok(detail) => Json(CatalogueAlbumDetail {
            album: detail.album,
            description: detail.description,
            duration_text: detail.duration_text,
            tracks: detail.tracks,
        })
        .into_response(),
        Err(message) => error(StatusCode::BAD_GATEWAY, clean(message)),
    }
}

async fn catalogue_artist(
    State(_state): State<SharedState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    match catalogue::catalogue_artist(id).await {
        Ok(artist) => Json(CatalogueArtist {
            id: artist.id,
            name: artist.name,
            description: artist.description,
            subscribers: artist.subscribers,
            cover: artist.cover,
            top_tracks: artist.top_tracks,
            albums: artist.albums,
            singles: artist.singles,
        })
        .into_response(),
        Err(message) => error(StatusCode::BAD_GATEWAY, clean(message)),
    }
}

async fn catalogue_playlist(
    State(_state): State<SharedState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    match catalogue::catalogue_playlist(id).await {
        Ok(detail) => Json(CataloguePlaylistDetail {
            playlist: detail.playlist,
            tracks: detail.tracks,
            continuation: detail.continuation,
        })
        .into_response(),
        Err(message) => error(StatusCode::BAD_GATEWAY, clean(message)),
    }
}

async fn stream_audio(
    State(_state): State<SharedState>,
    AxumPath(id): AxumPath<String>,
    headers: HeaderMap,
) -> Response {
    let video_id = id.trim();
    if video_id.is_empty() {
        return error(StatusCode::BAD_REQUEST, "This track has no video id.");
    }
    if video_id.starts_with(CATALOGUE_PREFIX) {
        return error(StatusCode::BAD_REQUEST, "This track has no video id.");
    }

    let path = match catalogue::fetch_to_cache(video_id).await {
        Ok(path) => path,
        Err(message) => return error(StatusCode::BAD_GATEWAY, clean(message)),
    };

    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let content_type = match ext.as_str() {
        "webm" => "audio/webm",
        "m4a" => "audio/mp4",
        "mp3" => "audio/mpeg",
        _ => "application/octet-stream",
    };
    crate::services::mobile_sync::server::serve_file_with_range(path, headers, content_type).await
}

async fn player_state(State(_state): State<SharedState>) -> Response {
    let status = match player::status() {
        Ok(status) => status,
        Err(message) => return error(StatusCode::SERVICE_UNAVAILABLE, clean(message)),
    };
    let np = now_playing().await;
    Json(MobilePlayerState {
        playing: status.playing,
        position: status.position,
        duration: status.duration,
        volume: status.volume,
        track: np.track,
        queue: np.queue,
    })
    .into_response()
}

async fn player_action(
    State(_state): State<SharedState>,
    Json(action): Json<MobilePlayerAction>,
) -> Response {
    let name = action.action.trim().to_ascii_lowercase();
    let command = match name.as_str() {
        "pause" => MediaControlAction::Pause,
        "play" | "resume" => MediaControlAction::Play,
        "toggle" => MediaControlAction::Toggle,
        "next" => MediaControlAction::Next,
        "previous" => MediaControlAction::Previous,
        "seek" => {
            let Some(position) = action.time.filter(|t| t.is_finite() && *t >= 0.0) else {
                return error(StatusCode::BAD_REQUEST, "That seek position is not valid.");
            };
            MediaControlAction::Seek { position }
        }
        "volume" => {
            let Some(volume) = action.volume.filter(|v| *v <= 100) else {
                return error(StatusCode::BAD_REQUEST, "That volume is not valid.");
            };
            MediaControlAction::SetVolume {
                volume: f64::from(volume) / 100.0,
            }
        }
        _ => {
            return error(StatusCode::BAD_REQUEST, "That action is not supported.");
        }
    };

    let Some(app) = player::app_handle() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "The player is not ready yet.",
        );
    };
    if app.emit("vynl:media-control", command).is_err() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "The player is not ready yet.",
        );
    }

    Json(serde_json::json!({ "ok": true })).into_response()
}
