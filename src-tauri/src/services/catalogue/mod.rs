pub mod stream;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use vynl_catalogue::client::CatalogueClient;
use ytmusic::YtMusic;

use crate::commands::types::*;
use crate::services::{downloader, playlists};

use stream::{PreparedAudio, PreparedStream, StreamCache, UrlResolver};

const STREAM_CACHE_DIR: &str = "catalogue";
const CACHE_TAG: &str = "stream-cache";
const PREPARED_LIMIT: usize = 4;
const PREPARED_TTL: Duration = Duration::from_secs(120);

pub(super) fn sanitize_error(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(start) = rest.find("http://").or_else(|| rest.find("https://")) {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let end = tail
            .find(|c: char| c.is_whitespace() || c == ')' || c == ']')
            .unwrap_or(tail.len());
        rest = &tail[end..];
    }
    out.push_str(rest);
    let cleaned = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.len() > 200 {
        let mut cut = 197;
        while !cleaned.is_char_boundary(cut) {
            cut -= 1;
        }
        format!("{}...", &cleaned[..cut])
    } else {
        cleaned
    }
}

static SERVICE: OnceLock<Arc<CatalogueService>> = OnceLock::new();

type RuntimeHolder = Arc<tokio::runtime::Runtime>;

pub struct CatalogueService {
    client: CatalogueClient,
    resolver: Arc<UrlResolver>,
    cache: Arc<StreamCache>,
    runtime: RuntimeHolder,
    prepared: Mutex<HashMap<String, PreparedEntry>>,
    seeks: Mutex<(u64, u64)>,
}

struct PreparedEntry {
    stream: PreparedStream,
    stored: Instant,
}

impl CatalogueService {
    fn new(user_data: &Path) -> Result<Self, String> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(2)
                .thread_name("vynl-catalogue")
                .build()
                .map_err(|e| format!("could not start the catalogue runtime: {e}"))?,
        );
        let ytmusic = Arc::new(YtMusic::anonymous());
        let client = CatalogueClient::with_client(ytmusic.clone());
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("could not build the catalogue client: {e}"))?;
        let cache_dir = user_data.join(STREAM_CACHE_DIR).join(CACHE_TAG);
        let cache = StreamCache::open(cache_dir.clone());
        let resolver = Arc::new(UrlResolver {
            ytmusic: ytmusic.clone(),
            runtime: runtime.clone(),
            http,
            data_dir: user_data.to_path_buf(),
        });

        Ok(Self {
            client,
            resolver,
            cache,
            runtime,
            prepared: Mutex::new(HashMap::new()),
            seeks: Mutex::new((0, 0)),
        })
    }

    fn note_seek(&self, generation: u64, request_id: u64) {
        let mut latest = self.seeks.lock().unwrap_or_else(|e| e.into_inner());
        if generation != latest.0 || request_id >= latest.1 {
            *latest = (generation, request_id);
        }
    }

    fn seek_is_current(&self, generation: u64, request_id: u64) -> bool {
        let latest = self.seeks.lock().unwrap_or_else(|e| e.into_inner());
        latest.0 == generation && latest.1 == request_id
    }

    async fn search(&self, query: &str) -> Result<CatalogueSearchResults, String> {
        self.client.search(query).await
    }

    async fn album(&self, browse_id: &str) -> Result<CatalogueAlbumDetail, String> {
        self.client.album(browse_id).await
    }

    async fn artist(&self, browse_id: &str) -> Result<CatalogueArtist, String> {
        self.client.artist(browse_id).await
    }

    async fn taste(&self, seeds: &[String]) -> Result<Vec<CatalogueTrack>, String> {
        self.client.taste(seeds).await
    }

    async fn playlist(&self, playlist_id: &str) -> Result<CataloguePlaylistDetail, String> {
        self.client.playlist(playlist_id).await
    }

    async fn playlist_page(&self, token: &str) -> Result<CataloguePlaylistPage, String> {
        self.client.playlist_page(token).await
    }

    async fn prepare(
        &self,
        video_id: &str,
        start: f64,
        duration_hint: Option<f64>,
    ) -> Result<PreparedStream, String> {
        stream::prepare(&self.cache, &self.resolver, video_id, start, duration_hint).await
    }

    fn prepare_blocking(
        &self,
        video_id: &str,
        start: f64,
        duration_hint: Option<f64>,
    ) -> Result<PreparedStream, String> {
        self.runtime
            .block_on(self.prepare(video_id, start, duration_hint))
    }

    fn stash(&self, prepared: PreparedStream) {
        let mut map = self.prepared.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        map.retain(|_, entry| now.duration_since(entry.stored) < PREPARED_TTL);
        while map.len() >= PREPARED_LIMIT {
            let oldest = map
                .iter()
                .min_by_key(|(_, entry)| entry.stored)
                .map(|(id, _)| id.clone());
            let Some(id) = oldest else { break };
            map.remove(&id);
        }
        map.insert(
            prepared.video_id.clone(),
            PreparedEntry {
                stream: prepared,
                stored: now,
            },
        );
    }

    fn take_prepared(&self, video_id: &str, start: f64) -> Option<PreparedStream> {
        let mut map = self.prepared.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        map.retain(|_, entry| now.duration_since(entry.stored) < PREPARED_TTL);
        let entry = map.get(video_id)?;
        if (entry.stream.start - start).abs() > 0.01
            || entry.stream.session().is_some_and(|s| s.is_finished())
        {
            map.remove(video_id);
            return None;
        }
        map.remove(video_id).map(|entry| entry.stream)
    }

    fn discard_prepared(&self, video_id: &str) {
        let mut map = self.prepared.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(video_id);
    }

    async fn prebuffer(
        &self,
        video_id: &str,
        duration_hint: Option<f64>,
    ) -> Result<CatalogueStreamInfo, String> {
        {
            let mut map = self.prepared.lock().unwrap_or_else(|e| e.into_inner());
            let now = Instant::now();
            if let Some(entry) = map.get_mut(video_id) {
                if now.duration_since(entry.stored) < PREPARED_TTL {
                    entry.stored = now;
                    return Ok(CatalogueStreamInfo {
                        video_id: video_id.to_string(),
                        duration: entry.stream.duration.or(duration_hint),
                        cached: entry.stream.cached,
                    });
                }
            }
        }

        let prepared = self.prepare(video_id, 0.0, duration_hint).await?;
        let info = CatalogueStreamInfo {
            video_id: video_id.to_string(),
            duration: prepared.duration.or(duration_hint),
            cached: prepared.cached,
        };
        self.stash(prepared);
        Ok(info)
    }

    async fn add_to_playlist(
        &self,
        track: &CatalogueTrack,
        playlist_id: Option<String>,
        playlist_name: Option<String>,
        settings: &Settings,
        user_data: &Path,
        app: &AppHandle,
    ) -> Result<CatalogueAddResult, String> {
        let video_id = track.id.trim();
        if video_id.is_empty() {
            return Err("This track has no video id.".into());
        }

        let output = Path::new(&settings.output_dir);
        let (mut path, meta) = local_file_for(track, settings, output);
        let existed = path.is_file();
        let mut downloaded = false;

        if !existed {
            let collection = Collection {
                kind: CollectionKind::Track,
                id: video_id.to_string(),
                title: track.title.clone(),
                owner: None,
                cover: track.cover.clone(),
                tracks: vec![meta.clone()],
            };
            let mut matches = HashMap::new();
            matches.insert(
                video_id.to_string(),
                vec![SearchCandidate {
                    url: meta.url.clone(),
                    title: track.title.clone(),
                    duration: track.duration,
                    channel: Some(track.artist.clone()),
                    source: MatchSource::YouTubeMusic,
                    view_count: None,
                    channel_verified: Some(true),
                }],
            );
            let mut picks = HashMap::new();
            picks.insert(video_id.to_string(), 0i64);
            downloader::download_collection(
                collection,
                settings.clone(),
                user_data,
                app.clone(),
                DownloadOpts {
                    matches: Some(matches),
                    picks: Some(picks),
                    skip_ids: None,
                },
            )
            .await?;
            downloaded = path.is_file();
        }

        if !path.is_file() {
            let found = downloader::find_song_on_disk(output, user_data, &meta);
            let Some(found) = found else {
                return Err("The track downloaded but its file could not be found.".into());
            };
            path = found;
            downloaded = false;
        }
        let path_string = path.to_string_lossy().to_string();

        let playlist = match playlist_id {
            Some(id) if !id.trim().is_empty() => Some(playlists::add_local_replacing_stream(
                output,
                &id,
                &path_string,
                video_id,
            )?),
            _ => {
                let name = playlist_name
                    .map(|name| name.trim().to_string())
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| "Liked Songs".to_string());
                let playlist = playlist_by_name(output, &name)?;
                Some(playlists::add_local_replacing_stream(
                    output,
                    &playlist.id,
                    &path_string,
                    video_id,
                )?)
            }
        };

        Ok(CatalogueAddResult {
            path: path_string,
            downloaded,
            playlist,
        })
    }
}

fn local_file_for(
    track: &CatalogueTrack,
    settings: &Settings,
    output_dir: &Path,
) -> (PathBuf, TrackMeta) {
    let video_id = track.id.trim();
    let album = track
        .album
        .as_ref()
        .map(|a| a.name.clone())
        .unwrap_or_default();
    let meta = TrackMeta {
        id: video_id.to_string(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        album,
        year: None,
        duration: track.duration,
        cover: track.cover.clone(),
        track_number: None,
        url: format!("https://music.youtube.com/watch?v={video_id}"),
    };
    let ext = downloader::audio_format_ext(&settings.format);
    let base = downloader::sanitize(&downloader::render_pattern(
        &settings.filename_pattern,
        &meta,
    ));
    let path = output_dir.join(format!("{base}.{ext}"));
    (path, meta)
}

fn playlist_by_name(output_dir: &Path, name: &str) -> Result<Playlist, String> {
    if let Some(existing) = playlists::find_playlist_by_name(output_dir, name) {
        return Ok(existing);
    }
    playlists::create_playlist(output_dir, name)
}

pub fn init(user_data: &Path) -> Result<Arc<CatalogueService>, String> {
    if let Some(service) = SERVICE.get() {
        return Ok(service.clone());
    }
    let service = Arc::new(CatalogueService::new(user_data)?);
    let _ = SERVICE.set(service.clone());
    Ok(service)
}

pub fn get() -> Option<Arc<CatalogueService>> {
    SERVICE.get().cloned()
}

pub async fn fetch_to_cache(video_id: &str) -> Result<PathBuf, String> {
    let service = service()?;
    let video_id = video_id.trim();
    if video_id.is_empty() {
        return Err("This track has no video id.".into());
    }
    stream::fetch_complete_file(&service.cache, &service.resolver, video_id).await
}

fn service() -> Result<Arc<CatalogueService>, String> {
    get().ok_or_else(|| "The catalogue is not ready yet.".to_string())
}

pub async fn prepare_for_playback(
    video_id: &str,
    start: f64,
    duration_hint: Option<f64>,
) -> Result<PreparedStream, String> {
    let service = service()?;
    if start <= 0.01
        && let Some(prepared) = service.take_prepared(video_id, start)
    {
        return Ok(prepared);
    }
    service.prepare(video_id, start, duration_hint).await
}

pub fn prepare_blocking(
    video_id: &str,
    start: f64,
    duration_hint: Option<f64>,
) -> Result<PreparedStream, String> {
    let service = service()?;
    service.prepare_blocking(video_id, start, duration_hint)
}

pub fn take_prebuffered(video_id: &str) -> Option<PreparedStream> {
    get().and_then(|service| service.take_prepared(video_id, 0.0))
}

pub fn discard_prebuffered(video_id: &str) {
    if let Some(service) = get() {
        service.discard_prepared(video_id);
    }
}

#[tauri::command]
pub async fn catalogue_search(query: String) -> Result<CatalogueSearchResults, String> {
    service()?.search(&query).await
}

#[tauri::command]
pub async fn catalogue_cover(app: AppHandle, url: String) -> Result<String, String> {
    let user_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("could not resolve the app data directory: {e}"))?;
    crate::services::cover_cache::cache_cover(&user_data, &url).await
}

#[tauri::command]
pub async fn catalogue_album(id: String) -> Result<CatalogueAlbumDetail, String> {
    service()?.album(&id).await
}

#[tauri::command]
pub async fn catalogue_artist(id: String) -> Result<CatalogueArtist, String> {
    service()?.artist(&id).await
}

#[tauri::command]
pub async fn catalogue_playlist(id: String) -> Result<CataloguePlaylistDetail, String> {
    service()?.playlist(&id).await
}

#[tauri::command]
pub async fn catalogue_playlist_page(token: String) -> Result<CataloguePlaylistPage, String> {
    service()?.playlist_page(&token).await
}

#[tauri::command]
pub async fn catalogue_prebuffer(
    video_id: String,
    duration: Option<f64>,
) -> Result<CatalogueStreamInfo, String> {
    service()?.prebuffer(&video_id, duration).await
}

#[tauri::command]
pub async fn catalogue_discard_prebuffer(video_id: String) -> Result<(), String> {
    discard_prebuffered(&video_id);
    Ok(())
}

#[tauri::command]
pub async fn catalogue_stream(
    video_id: String,
    start: Option<f64>,
    duration: Option<f64>,
    generation: u64,
) -> Result<CatalogueStreamInfo, String> {
    let start = start
        .filter(|time| time.is_finite() && *time > 0.0)
        .unwrap_or(0.0);
    let prepared = prepare_for_playback(&video_id, start, duration).await?;
    let path = format!("ytm:{video_id}");
    let prepared_duration = prepared.duration.or(duration);

    match prepared.audio.clone() {
        PreparedAudio::Stream(session) => {
            crate::services::player::play_stream(path, session, prepared.start, generation)?;
        }
        PreparedAudio::File(file) => {
            let file = file.to_string_lossy().to_string();
            crate::services::player::play_cached_stream(
                path,
                file,
                (prepared.start > 0.0).then_some(prepared.start),
                generation,
            )?;
        }
    }

    Ok(CatalogueStreamInfo {
        video_id,
        duration: prepared_duration,
        cached: prepared.cached,
    })
}

#[tauri::command]
pub async fn catalogue_crossfade_stream(
    video_id: String,
    _duration: Option<f64>,
    generation: u64,
    fade_secs: f64,
) -> Result<(), String> {
    let prepared =
        take_prebuffered(&video_id).ok_or_else(|| "the stream is not prebuffered".to_string())?;
    let path = format!("ytm:{video_id}");
    match prepared.audio.clone() {
        PreparedAudio::Stream(session) => crate::services::player::crossfade_stream(
            path,
            session,
            prepared.start,
            generation,
            fade_secs,
        ),
        PreparedAudio::File(file) => {
            let file = file.to_string_lossy().to_string();
            crate::services::player::crossfade_cached_stream(
                path,
                file,
                prepared.start,
                generation,
                fade_secs,
            )
        }
    }
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
    if !time.is_finite() || time < 0.0 {
        return Err("invalid seek time".into());
    }
    let service = service()?;
    service.note_seek(generation, request_id);
    let prepared = service.prepare(&video_id, time, duration).await?;
    if !service.seek_is_current(generation, request_id) {
        return Ok(());
    }
    let path = format!("ytm:{video_id}");
    match prepared.audio.clone() {
        PreparedAudio::Stream(session) => crate::services::player::seek_stream_source(
            path,
            session,
            prepared.start,
            generation,
            playing,
        ),
        PreparedAudio::File(file) => {
            let file = file.to_string_lossy().to_string();
            crate::services::player::seek_cached_stream_source(
                path,
                file,
                prepared.start,
                generation,
                playing,
            )
        }
    }
}

#[tauri::command]
pub async fn catalogue_taste(seeds: Vec<String>) -> Result<Vec<CatalogueTrack>, String> {
    service()?.taste(&seeds).await
}

#[tauri::command]
pub async fn catalogue_save_to_playlist(
    track: CatalogueTrack,
    playlist_id: Option<String>,
    playlist_name: Option<String>,
    app: AppHandle,
) -> Result<Playlist, String> {
    let output = crate::commands::output_dir_from_settings(&app)?;
    let user_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    let settings = crate::services::settings::get_settings(&user_data);
    let id = match playlist_id {
        Some(id) if !id.trim().is_empty() => id.trim().to_string(),
        _ => {
            let name = playlist_name
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Liked Songs".to_string());
            playlist_by_name(&output, &name)?.id
        }
    };
    let (local, _) = local_file_for(&track, &settings, &output);
    let local_string = local.to_string_lossy().to_string();
    playlists::add_stream_to_playlist(&output, &id, &track, Some(&local_string))
}

#[tauri::command]
pub async fn catalogue_add_to_playlist(
    track: CatalogueTrack,
    playlist_id: Option<String>,
    playlist_name: Option<String>,
    app: AppHandle,
) -> Result<CatalogueAddResult, String> {
    let service = service()?;
    let user_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    let settings = crate::services::settings::get_settings(&user_data);
    service
        .add_to_playlist(
            &track,
            playlist_id,
            playlist_name,
            &settings,
            &user_data,
            &app,
        )
        .await
}

#[allow(dead_code)]
fn _keep_runtime_alive(service: &CatalogueService) -> Arc<tokio::runtime::Runtime> {
    service.runtime.clone()
}
