use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path as AxumPath, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};
use tower_http::cors::{Any, CorsLayer};

use crate::commands::types::LibraryTrack;
use crate::services::mobile_sync::{self, load_library_tracks, load_playlists};

pub struct AppStateInner {
    pub user_data: PathBuf,
    pub pin: Mutex<String>,
    pub token: Mutex<String>,
}

type SharedState = Arc<AppStateInner>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileCatalogTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: Option<i64>,
    pub duration: f64,
    pub track_number: Option<i64>,
    pub lyrics: Option<String>,
    pub ext: String,
    pub mtime: Option<u64>,
    pub has_cover: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobilePlaylistDto {
    pub id: String,
    pub name: String,
    pub track_ids: Vec<String>,
    pub cover: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileSyncManifestEntry {
    pub id: String,
    pub mtime: Option<u64>,
    pub size: u64,
    pub has_cover: bool,
    pub ext: String,
}

#[derive(Deserialize)]
struct PairBody {
    pin: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PairResponse {
    token: String,
    port: u16,
}

pub async fn serve(
    port: u16,
    state: SharedState,
    stop: oneshot::Receiver<()>,
) -> Result<(), String> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS, Method::HEAD])
        .allow_headers(Any);

    let app = Router::new()
        .route("/pair", post(pair))
        .route("/v1/catalog", get(catalog))
        .route("/v1/playlists", get(playlists_handler))
        .route("/v1/sync-manifest", get(sync_manifest))
        .route("/v1/tracks/{id}/audio", get(track_audio))
        .route("/v1/tracks/{id}/cover", get(track_cover))
        .route("/health", get(|| async { Json(serde_json::json!({"ok": true})) }))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .layer(cors)
        .with_state(state);

    let listener = TcpListener::bind(("0.0.0.0", port))
        .await
        .map_err(|e| format!("bind {port}: {e}"))?;

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = stop.await;
        })
        .await
        .map_err(|e| e.to_string())
}

async fn auth_middleware(
    State(state): State<SharedState>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path();
    if path == "/pair" || path == "/health" {
        return next.run(req).await;
    }

    let expected = state.token.lock().await.clone();
    let authorized = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|t| !expected.is_empty() && t == expected);

    if authorized {
        next.run(req).await
    } else {
        (StatusCode::UNAUTHORIZED, "unauthorized").into_response()
    }
}

async fn pair(
    State(state): State<SharedState>,
    Json(body): Json<PairBody>,
) -> Result<Json<PairResponse>, StatusCode> {
    let pin = state.pin.lock().await.clone();
    if body.pin.trim() != pin {
        return Err(StatusCode::FORBIDDEN);
    }
    let token = state.token.lock().await.clone();
    let cfg = mobile_sync::load_config(&state.user_data);
    Ok(Json(PairResponse {
        token,
        port: cfg.port,
    }))
}

fn to_catalog(track: &LibraryTrack) -> MobileCatalogTrack {
    MobileCatalogTrack {
        id: track.id.clone(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        year: track.year,
        duration: track.duration,
        track_number: track.track_number,
        lyrics: track.lyrics.clone(),
        ext: track.ext.clone(),
        mtime: track.mtime,
        has_cover: track
            .cover
            .as_ref()
            .is_some_and(|c| Path::new(c).is_file()),
    }
}

fn path_to_id_map(tracks: &[LibraryTrack]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for t in tracks {
        map.insert(t.path.clone(), t.id.clone());
        // Also index by normalized separators for Windows playlists.
        let alt = t.path.replace('/', "\\");
        if alt != t.path {
            map.insert(alt, t.id.clone());
        }
        let alt2 = t.path.replace('\\', "/");
        if alt2 != t.path {
            map.insert(alt2, t.id.clone());
        }
    }
    map
}

fn find_track<'a>(tracks: &'a [LibraryTrack], id: &str) -> Option<&'a LibraryTrack> {
    tracks.iter().find(|t| t.id == id)
}

async fn catalog(State(state): State<SharedState>) -> Json<Vec<MobileCatalogTrack>> {
    let tracks = tokio::task::spawn_blocking({
        let ud = state.user_data.clone();
        move || load_library_tracks(&ud)
    })
    .await
    .unwrap_or_default();
    Json(tracks.iter().map(to_catalog).collect())
}

async fn playlists_handler(State(state): State<SharedState>) -> Json<Vec<MobilePlaylistDto>> {
    let user_data = state.user_data.clone();
    let (tracks, playlists) = tokio::task::spawn_blocking(move || {
        let tracks = load_library_tracks(&user_data);
        let playlists = load_playlists(&user_data);
        (tracks, playlists)
    })
    .await
    .unwrap_or_default();

    let map = path_to_id_map(&tracks);
    let out = playlists
        .into_iter()
        .map(|pl| {
            let track_ids = pl
                .paths
                .iter()
                .filter_map(|p| map.get(p).cloned())
                .collect();
            MobilePlaylistDto {
                id: pl.id,
                name: pl.name,
                track_ids,
                cover: None,
                created_at: pl.created_at,
                updated_at: pl.updated_at,
            }
        })
        .collect();
    Json(out)
}

async fn sync_manifest(
    State(state): State<SharedState>,
) -> Json<Vec<MobileSyncManifestEntry>> {
    let tracks = tokio::task::spawn_blocking({
        let ud = state.user_data.clone();
        move || load_library_tracks(&ud)
    })
    .await
    .unwrap_or_default();

    let mut out = Vec::with_capacity(tracks.len());
    for t in &tracks {
        let size = std::fs::metadata(&t.path).map(|m| m.len()).unwrap_or(0);
        out.push(MobileSyncManifestEntry {
            id: t.id.clone(),
            mtime: t.mtime,
            size,
            has_cover: t.cover.as_ref().is_some_and(|c| Path::new(c).is_file()),
            ext: t.ext.clone(),
        });
    }
    Json(out)
}

async fn track_audio(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<String>,
    headers: HeaderMap,
) -> Response {
    let tracks = tokio::task::spawn_blocking({
        let ud = state.user_data.clone();
        move || load_library_tracks(&ud)
    })
    .await
    .unwrap_or_default();

    let Some(track) = find_track(&tracks, &id) else {
        return (StatusCode::NOT_FOUND, "track not found").into_response();
    };
    let path = PathBuf::from(&track.path);
    if !path.is_file() {
        return (StatusCode::NOT_FOUND, "file missing").into_response();
    }

    serve_file_with_range(path, headers, content_type_for_ext(&track.ext)).await
}

async fn track_cover(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let tracks = tokio::task::spawn_blocking({
        let ud = state.user_data.clone();
        move || load_library_tracks(&ud)
    })
    .await
    .unwrap_or_default();

    let Some(track) = find_track(&tracks, &id) else {
        return (StatusCode::NOT_FOUND, "track not found").into_response();
    };
    let Some(cover) = track.cover.as_ref() else {
        return (StatusCode::NOT_FOUND, "no cover").into_response();
    };
    let path = PathBuf::from(cover);
    if !path.is_file() {
        return (StatusCode::NOT_FOUND, "cover missing").into_response();
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();
    let ct = match ext.as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };
    serve_file_with_range(path, HeaderMap::new(), ct).await
}

fn content_type_for_ext(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "mp3" => "audio/mpeg",
        "m4a" | "aac" => "audio/mp4",
        "flac" => "audio/flac",
        "ogg" | "opus" => "audio/ogg",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    }
}

async fn serve_file_with_range(path: PathBuf, headers: HeaderMap, content_type: &str) -> Response {
    let meta = match tokio::fs::metadata(&path).await {
        Ok(m) => m,
        Err(_) => return (StatusCode::NOT_FOUND, "file missing").into_response(),
    };
    let len = meta.len();

    let range = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_bytes_range);

    let mut file = match File::open(&path).await {
        Ok(f) => f,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "open failed").into_response(),
    };

    if let Some((start, end_inclusive)) = range {
        if start >= len {
            return (
                StatusCode::RANGE_NOT_SATISFIABLE,
                [(header::CONTENT_RANGE, format!("bytes */{len}"))],
                "range not satisfiable",
            )
                .into_response();
        }
        let end = end_inclusive.unwrap_or(len - 1).min(len - 1);
        let content_len = end - start + 1;
        if file.seek(SeekFrom::Start(start)).await.is_err() {
            return (StatusCode::INTERNAL_SERVER_ERROR, "seek failed").into_response();
        }
        let mut buf = vec![0u8; content_len as usize];
        if file.read_exact(&mut buf).await.is_err() {
            return (StatusCode::INTERNAL_SERVER_ERROR, "read failed").into_response();
        }
        let mut res = Response::new(Body::from(buf));
        *res.status_mut() = StatusCode::PARTIAL_CONTENT;
        let headers = res.headers_mut();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_str(content_type).unwrap_or(HeaderValue::from_static("application/octet-stream")));
        headers.insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&content_len.to_string()).unwrap(),
        );
        headers.insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{len}")).unwrap(),
        );
        headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
        res
    } else {
        let mut buf = Vec::with_capacity(len as usize);
        if file.read_to_end(&mut buf).await.is_err() {
            return (StatusCode::INTERNAL_SERVER_ERROR, "read failed").into_response();
        }
        let mut res = Response::new(Body::from(buf));
        *res.status_mut() = StatusCode::OK;
        let headers = res.headers_mut();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_str(content_type).unwrap_or(HeaderValue::from_static("application/octet-stream")));
        headers.insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&len.to_string()).unwrap(),
        );
        headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
        res
    }
}

fn parse_bytes_range(header: &str) -> Option<(u64, Option<u64>)> {
    let s = header.strip_prefix("bytes=")?;
    let (start_s, end_s) = s.split_once('-')?;
    let start: u64 = start_s.parse().ok()?;
    let end = if end_s.is_empty() {
        None
    } else {
        Some(end_s.parse().ok()?)
    };
    Some((start, end))
}
