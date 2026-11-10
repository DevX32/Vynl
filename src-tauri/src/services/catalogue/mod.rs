pub mod stream;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use ytmusic::{Album, AlbumKind, Playlist as YtPlaylist, Track, YtMusic};

use crate::commands::types::*;
use crate::services::{downloader, playlists};

use stream::{PreparedAudio, PreparedStream, StreamCache, UrlResolver};

const STREAM_CACHE_DIR: &str = "catalogue";
const CACHE_TAG: &str = "stream-cache";
const PREPARED_LIMIT: usize = 4;
const PREPARED_TTL: Duration = Duration::from_secs(120);
const SEARCH_LIMIT: usize = 20;
const TASTE_LIMIT: usize = 60;
const TASTE_SEEDS: usize = 6;

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

fn catalogue_error(context: &str, error: impl std::fmt::Display) -> String {
    let detail = sanitize_error(&error.to_string());
    let detail = detail
        .rsplit_once(" -> ")
        .map(|(_, tail)| tail.trim().to_string())
        .unwrap_or(detail);
    if detail.is_empty() {
        return format!("{context} failed");
    }
    format!("{context} failed: {detail}")
}

static SERVICE: OnceLock<Arc<CatalogueService>> = OnceLock::new();

type RuntimeHolder = Arc<tokio::runtime::Runtime>;

pub struct CatalogueService {
    ytmusic: Arc<YtMusic>,
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
            ytmusic,
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
        let query = query.trim();
        if query.is_empty() {
            return Ok(CatalogueSearchResults {
                songs: Vec::new(),
                albums: Vec::new(),
                playlists: Vec::new(),
            });
        }

        let songs = self.ytmusic.search_songs(query).await;

        Ok(CatalogueSearchResults {
            songs: songs
                .map_err(|e| catalogue_error("song search", e))?
                .iter()
                .filter_map(track_dto)
                .take(SEARCH_LIMIT)
                .collect(),
            albums: Vec::new(),
            playlists: Vec::new(),
        })
    }

    async fn album(&self, browse_id: &str) -> Result<CatalogueAlbumDetail, String> {
        let detail = self
            .ytmusic
            .album(browse_id)
            .await
            .map_err(|e| catalogue_error("album lookup", e))?;
        Ok(CatalogueAlbumDetail {
            album: album_dto(&detail.album),
            description: detail.description,
            duration_text: detail.duration_text,
            tracks: detail.tracks.iter().filter_map(track_dto).collect(),
        })
    }

    async fn artist(&self, browse_id: &str) -> Result<CatalogueArtist, String> {
        let artist = self
            .ytmusic
            .artist(browse_id)
            .await
            .map_err(|e| catalogue_error("artist lookup", e))?;
        Ok(CatalogueArtist {
            id: artist.browse_id,
            name: artist.name,
            description: artist.description,
            subscribers: artist.subscribers,
            cover: cover_of(&artist.thumbnails),
            top_tracks: artist.top_tracks.iter().filter_map(track_dto).collect(),
            albums: artist.albums.iter().map(album_dto).collect(),
            singles: artist.singles.iter().map(album_dto).collect(),
        })
    }

    async fn taste(&self, seeds: &[String]) -> Result<Vec<CatalogueTrack>, String> {
        let mut seen_seeds = HashSet::new();
        let seeds: Vec<String> = seeds
            .iter()
            .map(|seed| seed.trim().to_string())
            .filter(|seed| !seed.is_empty() && seen_seeds.insert(seed.clone()))
            .take(TASTE_SEEDS)
            .collect();
        if seeds.is_empty() {
            return Ok(Vec::new());
        }

        let mut tasks = Vec::new();
        for seed in &seeds {
            let client = self.ytmusic.clone();
            let seed = seed.clone();
            tasks.push(tokio::spawn(async move { client.track_radio(&seed).await }));
        }

        let mut lists: Vec<Vec<Track>> = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Ok(tracks)) => lists.push(tracks),
                Ok(Err(error)) => eprintln!("taste radio failed: {error:#}"),
                Err(error) => eprintln!("taste radio task failed: {error}"),
            }
        }
        if lists.is_empty() {
            return Err("could not build your mix".into());
        }

        let seed_ids: HashSet<&str> = seeds.iter().map(String::as_str).collect();
        let mut seen = HashSet::new();
        let mut songs = Vec::new();
        let longest = lists.iter().map(Vec::len).max().unwrap_or(0);
        'outer: for index in 0..longest {
            for list in &lists {
                let Some(track) = list.get(index) else {
                    continue;
                };
                let Some(video_id) = track.video_id.as_deref() else {
                    continue;
                };
                if seed_ids.contains(video_id) || !seen.insert(video_id.to_string()) {
                    continue;
                }
                if let Some(dto) = track_dto(track) {
                    songs.push(dto);
                    if songs.len() >= TASTE_LIMIT {
                        break 'outer;
                    }
                }
            }
        }

        if songs.is_empty() {
            return Err("could not build your mix".into());
        }
        Ok(songs)
    }

    async fn playlist(&self, playlist_id: &str) -> Result<CataloguePlaylistDetail, String> {
        let detail = self
            .ytmusic
            .playlist_page(playlist_id)
            .await
            .map_err(|e| catalogue_error("playlist lookup", e))?;
        Ok(CataloguePlaylistDetail {
            playlist: playlist_dto(&detail.playlist),
            tracks: detail.tracks.iter().filter_map(track_dto).collect(),
            continuation: detail.continuation,
        })
    }

    async fn playlist_page(&self, token: &str) -> Result<CataloguePlaylistPage, String> {
        let page = self
            .ytmusic
            .playlist_continuation(token)
            .await
            .map_err(|e| catalogue_error("playlist page", e))?;
        Ok(CataloguePlaylistPage {
            tracks: page.tracks.iter().filter_map(track_dto).collect(),
            continuation: page.continuation,
        })
    }

    async fn prepare(
        &self,
        video_id: &str,
        start: f64,
        duration_hint: Option<f64>,
    ) -> Result<PreparedStream, String> {
        stream::prepare(
            &self.cache,
            &self.resolver,
            video_id,
            start,
            duration_hint,
        )
        .await
    }

    fn prepare_blocking(
        &self,
        video_id: &str,
        start: f64,
        duration_hint: Option<f64>,
    ) -> Result<PreparedStream, String> {
        self.runtime.block_on(self.prepare(video_id, start, duration_hint))
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
        if (entry.stream.start - start).abs() > 0.01 || entry.stream.session().is_some_and(|s| s.is_finished())
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

        let url = format!("https://music.youtube.com/watch?v={video_id}");
        let album = track.album.as_ref().map(|a| a.name.clone()).unwrap_or_default();
        let meta = TrackMeta {
            id: video_id.to_string(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            album,
            year: None,
            duration: track.duration,
            cover: track.cover.clone(),
            track_number: None,
            url: url.clone(),
        };
        let ext = downloader::audio_format_ext(&settings.format);
        let base = downloader::sanitize(&downloader::render_pattern(
            &settings.filename_pattern,
            &meta,
        ));
        let path = PathBuf::from(&settings.output_dir).join(format!("{base}.{ext}"));
        let existed = path.is_file();

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
                    url,
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
        }

        if !path.is_file() {
            return Err("The track downloaded but its file could not be found.".into());
        }
        let path_string = path.to_string_lossy().to_string();

        let playlist = match playlist_id {
            Some(id) if !id.trim().is_empty() => {
                let playlist = playlists::add_to_playlist(
                    Path::new(&settings.output_dir),
                    &id,
                    &[path_string.clone()],
                )?;
                Some(playlist)
            }
            _ => {
                let name = playlist_name
                    .map(|name| name.trim().to_string())
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| "Liked Songs".to_string());
                let playlist =
                    playlists::create_playlist(Path::new(&settings.output_dir), &name)?;
                let playlist = playlists::add_to_playlist(
                    Path::new(&settings.output_dir),
                    &playlist.id,
                    &[path_string.clone()],
                )?;
                Some(playlist)
            }
        };

        Ok(CatalogueAddResult {
            path: path_string,
            downloaded: !existed,
            playlist,
        })
    }
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

fn cover_of(thumbnails: &[ytmusic::Thumbnail]) -> Option<String> {
    ytmusic::best_thumbnail(thumbnails).map(|thumbnail| thumbnail.url.clone())
}

fn artist_names(artists: &[ytmusic::ArtistRef]) -> String {
    artists
        .iter()
        .map(|artist| artist.name.trim())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

fn artist_refs(artists: &[ytmusic::ArtistRef]) -> Vec<CatalogueArtistRef> {
    artists
        .iter()
        .map(|artist| CatalogueArtistRef {
            name: artist.name.clone(),
            id: artist.id.clone(),
        })
        .collect()
}

fn track_dto(track: &Track) -> Option<CatalogueTrack> {
    let id = track.video_id.clone()?;
    if id.is_empty() {
        return None;
    }
    Some(CatalogueTrack {
        id,
        title: track.title.clone(),
        artist: artist_names(&track.artists),
        artists: artist_refs(&track.artists),
        album: track.album.as_ref().map(|album| CatalogueAlbumRef {
            name: album.name.clone(),
            id: album.id.clone(),
        }),
        duration: track.duration.map(|d| d.as_secs_f64()),
        cover: cover_of(&track.thumbnails),
        explicit: track.explicit,
    })
}

fn album_kind(kind: AlbumKind) -> &'static str {
    match kind {
        AlbumKind::Album => "album",
        AlbumKind::Single => "single",
        AlbumKind::Ep => "ep",
        AlbumKind::Compilation => "compilation",
    }
}

fn album_dto(album: &Album) -> CatalogueAlbum {
    CatalogueAlbum {
        id: album.browse_id.clone(),
        playlist_id: album.playlist_id.clone(),
        title: album.title.clone(),
        artist: artist_names(&album.artists),
        artists: artist_refs(&album.artists),
        kind: album_kind(album.kind).to_string(),
        year: album.year,
        track_count: album.track_count,
        cover: cover_of(&album.thumbnails),
    }
}

fn playlist_dto(playlist: &YtPlaylist) -> CataloguePlaylist {
    CataloguePlaylist {
        id: playlist.id.clone(),
        title: playlist.title.clone(),
        author: playlist.author.clone(),
        track_count: playlist.track_count,
        cover: cover_of(&playlist.thumbnails),
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
    let start = start.filter(|time| time.is_finite() && *time > 0.0).unwrap_or(0.0);
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
    let prepared = take_prebuffered(&video_id)
        .ok_or_else(|| "the stream is not prebuffered".to_string())?;
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
    let id = match playlist_id {
        Some(id) if !id.trim().is_empty() => id.trim().to_string(),
        _ => {
            let name = playlist_name
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Liked Songs".to_string());
            playlists::create_playlist(&output, &name)?.id
        }
    };
    playlists::add_stream_to_playlist(&output, &id, &track)
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
        .add_to_playlist(&track, playlist_id, playlist_name, &settings, &user_data, &app)
        .await
}

#[allow(dead_code)]
fn _keep_runtime_alive(service: &CatalogueService) -> Arc<tokio::runtime::Runtime> {
    service.runtime.clone()
}
