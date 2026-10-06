use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use axum::body::{Body, Bytes};
use axum::extract::{Path as AxumPath, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncSeekExt, ReadBuf, SeekFrom};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock, oneshot};

use crate::commands::types::LibraryTrack;
use crate::services::mobile_sync::{self, load_library_tracks, load_playlists};
use axum::extract::ConnectInfo;

pub struct AppStateInner {
    pub user_data: PathBuf,
    pub pin: Mutex<String>,
    pub token: Mutex<String>,
    pub attempts: Mutex<HashMap<String, Attempt>>,
    pub tracks: RwLock<Option<Arc<TrackSnapshot>>>,
    pub tracks_load: Mutex<()>,
    pub manifest: RwLock<Option<Arc<ManifestSnapshot>>>,
    pub manifest_load: Mutex<()>,
    pub next_generation: AtomicU64,
}

#[derive(Clone, Copy)]
pub struct Attempt {
    pub failures: u32,
    pub locked_until: Option<u64>,
    pub lockouts: u32,
}

type SharedState = Arc<AppStateInner>;

const MAX_PAIR_FAILURES: u32 = 5;
const LOCKOUT_SECS: u64 = 30;
const LOCKOUT_MAX_SECS: u64 = 900;
const SNAPSHOT_TTL: Duration = Duration::from_secs(10);
const STREAM_CHUNK: usize = 128 * 1024;

pub struct TrackSnapshot {
    tracks: Arc<Vec<LibraryTrack>>,
    by_id: HashMap<String, usize>,
    generation: u64,
    loaded_at: Instant,
}

impl TrackSnapshot {
    fn build(tracks: Vec<LibraryTrack>, generation: u64) -> Self {
        let by_id = tracks
            .iter()
            .enumerate()
            .map(|(i, t)| (t.id.clone(), i))
            .collect();
        Self {
            tracks: Arc::new(tracks),
            by_id,
            generation,
            loaded_at: Instant::now(),
        }
    }

    fn is_fresh(&self) -> bool {
        self.loaded_at.elapsed() < SNAPSHOT_TTL
    }

    fn get(&self, id: &str) -> Option<&LibraryTrack> {
        self.by_id.get(id).and_then(|i| self.tracks.get(*i))
    }
}

async fn track_snapshot(state: &SharedState) -> Arc<TrackSnapshot> {
    {
        let cached = state.tracks.read().await;
        if let Some(snap) = cached.as_ref()
            && snap.is_fresh()
        {
            return Arc::clone(snap);
        }
    }

    let _load = state.tracks_load.lock().await;
    {
        let cached = state.tracks.read().await;
        if let Some(snap) = cached.as_ref()
            && snap.is_fresh()
        {
            return Arc::clone(snap);
        }
    }

    let user_data = state.user_data.clone();
    let generation = state.next_generation.fetch_add(1, Ordering::Relaxed) + 1;
    let snap = tokio::task::spawn_blocking(move || {
        TrackSnapshot::build(load_library_tracks(&user_data), generation)
    })
    .await
    .unwrap_or_else(|_| TrackSnapshot::build(Vec::new(), generation));

    let snap = Arc::new(snap);
    *state.tracks.write().await = Some(Arc::clone(&snap));
    snap
}

/// The manifest is derived from the track snapshot, so it is cached against
/// that snapshot's generation. A fresh library load produces a new generation
/// and the manifest is rebuilt with it, which keeps a desktop rescan visible
/// without re-stat'ing every file on every request.
async fn cached_manifest(state: &SharedState) -> Arc<Vec<MobileSyncManifestEntry>> {
    let snap = track_snapshot(state).await;

    {
        let cached = state.manifest.read().await;
        if let Some(entries) = cached.as_ref()
            && entries.generation == snap.generation
        {
            return Arc::clone(&entries.entries);
        }
    }

    let _load = state.manifest_load.lock().await;
    {
        let cached = state.manifest.read().await;
        if let Some(entries) = cached.as_ref()
            && entries.generation == snap.generation
        {
            return Arc::clone(&entries.entries);
        }
    }

    let tracks = Arc::clone(&snap.tracks);
    let generation = snap.generation;
    let entries = tokio::task::spawn_blocking(move || build_manifest(&tracks))
        .await
        .unwrap_or_default();

    let entries = Arc::new(ManifestSnapshot {
        entries: Arc::new(entries),
        generation,
    });
    *state.manifest.write().await = Some(Arc::clone(&entries));
    Arc::clone(&entries.entries)
}

pub struct ManifestSnapshot {
    entries: Arc<Vec<MobileSyncManifestEntry>>,
    generation: u64,
}

fn build_manifest(tracks: &[LibraryTrack]) -> Vec<MobileSyncManifestEntry> {
    let mut out = Vec::with_capacity(tracks.len());
    for t in tracks {
        let size = std::fs::metadata(&t.path).map(|m| m.len()).unwrap_or(0);
        out.push(MobileSyncManifestEntry {
            id: t.id.clone(),
            mtime: t.mtime,
            size,
            has_cover: t.cover.as_ref().is_some_and(|c| Path::new(c).is_file()),
            ext: t.ext.clone(),
        });
    }
    out
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn secret_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

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
pub struct MobileLyrics {
    pub kind: String,
    pub text: String,
    pub source: String,
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
    let app = Router::new()
        .route("/pair", post(pair))
        .route("/v1/catalog", get(catalog))
        .route("/v1/playlists", get(playlists_handler))
        .route("/v1/sync-manifest", get(sync_manifest))
        .route("/v1/tracks/{id}/audio", get(track_audio))
        .route("/v1/tracks/{id}/cover", get(track_cover))
        .route("/v1/tracks/{id}/synced-lyrics", get(track_synced_lyrics))
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"ok": true})) }),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state);

    let listener = TcpListener::bind(("0.0.0.0", port))
        .await
        .map_err(|e| format!("bind {port}: {e}"))?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = stop.await;
    })
    .await
    .map_err(|e| e.to_string())
}

async fn auth_middleware(State(state): State<SharedState>, req: Request, next: Next) -> Response {
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
        .is_some_and(|t| !expected.is_empty() && secret_eq(t, &expected));

    if authorized {
        next.run(req).await
    } else {
        (StatusCode::UNAUTHORIZED, "unauthorized").into_response()
    }
}

async fn pair(
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Json(body): Json<PairBody>,
) -> Result<Json<PairResponse>, (StatusCode, &'static str)> {
    let key = addr.ip().to_string();

    if lockout_for(&state, &key).await {
        return Err((StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }

    let pin = state.pin.lock().await.clone();
    if !secret_eq(body.pin.trim(), &pin) {
        record_failure(&state, &key).await;
        return Err((StatusCode::FORBIDDEN, "wrong pin"));
    }

    clear_attempts(&state, &key).await;

    let token = state.token.lock().await.clone();
    let cfg = mobile_sync::load_config(&state.user_data);
    Ok(Json(PairResponse {
        token,
        port: cfg.port,
    }))
}

async fn lockout_for(state: &SharedState, key: &str) -> bool {
    let mut attempts = state.attempts.lock().await;
    let Some(entry) = attempts.get_mut(key) else {
        return false;
    };
    match entry.locked_until {
        Some(until) if until > now_secs() => true,
        Some(_) => {
            entry.locked_until = None;
            false
        }
        None => false,
    }
}

async fn record_failure(state: &SharedState, key: &str) {
    let mut attempts = state.attempts.lock().await;
    let entry = attempts.entry(key.to_string()).or_insert(Attempt {
        failures: 0,
        locked_until: None,
        lockouts: 0,
    });
    entry.failures += 1;
    if entry.failures < MAX_PAIR_FAILURES {
        return;
    }
    entry.failures = 0;
    entry.lockouts = entry.lockouts.saturating_add(1);
    let shift = entry.lockouts.min(6) - 1;
    let secs = LOCKOUT_SECS
        .saturating_mul(1u64 << shift)
        .min(LOCKOUT_MAX_SECS);
    entry.locked_until = Some(now_secs() + secs);
}

async fn clear_attempts(state: &SharedState, key: &str) {
    state.attempts.lock().await.remove(key);
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
        has_cover: track.cover.as_ref().is_some_and(|c| Path::new(c).is_file()),
    }
}

fn path_to_id_map(tracks: &[LibraryTrack]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for t in tracks {
        map.insert(t.path.clone(), t.id.clone());
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

async fn catalog(State(state): State<SharedState>) -> Json<Vec<MobileCatalogTrack>> {
    let snap = track_snapshot(&state).await;
    let tracks = Arc::clone(&snap.tracks);
    tokio::task::spawn_blocking(move || Json(tracks.iter().map(to_catalog).collect()))
        .await
        .unwrap_or_else(|_| Json(Vec::new()))
}

async fn playlists_handler(State(state): State<SharedState>) -> Json<Vec<MobilePlaylistDto>> {
    let snap = track_snapshot(&state).await;
    let tracks = Arc::clone(&snap.tracks);
    let user_data = state.user_data.clone();

    tokio::task::spawn_blocking(move || {
        let playlists = load_playlists(&user_data);
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
    })
    .await
    .unwrap_or_else(|_| Json(Vec::new()))
}

async fn sync_manifest(State(state): State<SharedState>) -> Json<Vec<MobileSyncManifestEntry>> {
    Json(cached_manifest(&state).await.as_ref().clone())
}

async fn track_audio(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<String>,
    headers: HeaderMap,
) -> Response {
    let snap = track_snapshot(&state).await;

    let Some(track) = snap.get(&id) else {
        return (StatusCode::NOT_FOUND, "track not found").into_response();
    };
    let path = PathBuf::from(&track.path);
    let ext = track.ext.clone();

    serve_file_with_range(path, headers, content_type_for_ext(&ext)).await
}

async fn track_cover(State(state): State<SharedState>, AxumPath(id): AxumPath<String>) -> Response {
    let snap = track_snapshot(&state).await;

    let Some(track) = snap.get(&id) else {
        return (StatusCode::NOT_FOUND, "track not found").into_response();
    };
    let Some(cover) = track.cover.as_ref() else {
        return (StatusCode::NOT_FOUND, "no cover").into_response();
    };
    let path = PathBuf::from(cover);

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

async fn track_synced_lyrics(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let snap = track_snapshot(&state).await;

    let Some(track) = snap.get(&id) else {
        return (StatusCode::NOT_FOUND, "track not found").into_response();
    };

    let embedded = track
        .lyrics
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_string();
    let is_synced = |t: &str| t.contains('[') && t.contains(':');

    if !embedded.is_empty() && is_synced(&embedded) {
        return Json(MobileLyrics {
            kind: "lrc".into(),
            text: embedded,
            source: "embedded".into(),
        })
        .into_response();
    }

    let lookup = crate::commands::types::LyricsLookup {
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        duration: track.duration,
    };
    let ud = state.user_data.clone();

    let fetched = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        match rt {
            Ok(rt) => rt.block_on(crate::services::lyrics::fetch_lyrics_with_cache(
                &lookup, &ud,
            )),
            Err(_) => None,
        }
    })
    .await
    .unwrap_or(None);

    match fetched {
        Some(res) if is_synced(&res.text) => Json(MobileLyrics {
            kind: "lrc".into(),
            text: res.text,
            source: format!("{:?}", res.source).to_lowercase(),
        })
        .into_response(),
        _ => (StatusCode::NOT_FOUND, "no synced lyrics").into_response(),
    }
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

struct FileStream {
    file: File,
    remaining: u64,
    buf: Box<[u8]>,
}

impl FileStream {
    fn new(file: File, remaining: u64) -> Self {
        Self {
            file,
            remaining,
            buf: vec![0u8; STREAM_CHUNK].into_boxed_slice(),
        }
    }
}

impl Stream for FileStream {
    type Item = std::io::Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let me = self.get_mut();
        if me.remaining == 0 {
            return Poll::Ready(None);
        }

        let want = me.buf.len().min(me.remaining as usize);
        let mut read_buf = ReadBuf::new(&mut me.buf[..want]);
        match Pin::new(&mut me.file).poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => {
                let filled = read_buf.filled().len();
                if filled == 0 {
                    return Poll::Ready(None);
                }
                me.remaining -= filled as u64;
                Poll::Ready(Some(Ok(Bytes::copy_from_slice(read_buf.filled()))))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
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

    let (status, start, content_len) = match range {
        Some((start, end_inclusive)) => {
            if start >= len {
                return (
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    [(header::CONTENT_RANGE, format!("bytes */{len}"))],
                    "range not satisfiable",
                )
                    .into_response();
            }
            let end = end_inclusive.unwrap_or(len - 1).min(len - 1);
            if file.seek(SeekFrom::Start(start)).await.is_err() {
                return (StatusCode::INTERNAL_SERVER_ERROR, "seek failed").into_response();
            }
            (StatusCode::PARTIAL_CONTENT, Some(start), end - start + 1)
        }
        None => (StatusCode::OK, None, len),
    };

    let mut res = Response::new(Body::from_stream(FileStream::new(file, content_len)));
    *res.status_mut() = status;
    let res_headers = res.headers_mut();
    res_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    if let Ok(value) = HeaderValue::from_str(&content_len.to_string()) {
        res_headers.insert(header::CONTENT_LENGTH, value);
    }
    if let Some(start) = start {
        let end = start + content_len - 1;
        if let Ok(value) = HeaderValue::from_str(&format!("bytes {start}-{end}/{len}")) {
            res_headers.insert(header::CONTENT_RANGE, value);
        }
    }
    res_headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    res
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
