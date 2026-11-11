use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::{ACCEPT, CONTENT_RANGE, RANGE, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo, TrackType};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use ytmusic::{AudioFormat as YtAudioFormat, Client, YtMusic};

use crate::services::audio;

const FETCH_CHUNK: u64 = 256 * 1024;
const READAHEAD: u64 = 1024 * 1024;
const FETCH_STALL: Duration = Duration::from_secs(15);
const MAX_FETCH_ATTEMPTS: u32 = 4;
const MAX_REFRESH_ATTEMPTS: u32 = 3;
const CACHE_LIMIT: u64 = 2 * 1024 * 1024 * 1024;
const MAX_BUFFER_SECS: f64 = 6.0;
const DEFAULT_RATE: u32 = 48_000;
const DEFAULT_CHANNELS: u16 = 2;
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

static FILE_SERIAL: AtomicU64 = AtomicU64::new(0);

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

fn ext_for(format: &YtAudioFormat) -> &'static str {
    let mime = format.mime.to_ascii_lowercase();
    if mime.contains("webm") {
        "webm"
    } else if mime.contains("mp4") || mime.contains("m4a") || format.itag == 140 || format.itag == 139
    {
        "m4a"
    } else if mime.contains("mpeg") {
        "mp3"
    } else {
        "webm"
    }
}

pub async fn fetch_complete_file(
    cache: &Arc<StreamCache>,
    resolver: &Arc<UrlResolver>,
    video_id: &str,
) -> Result<PathBuf, String> {
    if let Some(path) = cache.lookup_complete(video_id) {
        return Ok(path);
    }

    let mut format = resolver.resolve(video_id, None).await?;
    let total = format
        .content_length
        .ok_or_else(|| "this stream has no known length".to_string())?;
    if total == 0 {
        return Err("this stream is empty".into());
    }

    let ext = ext_for(&format);
    let (path, file_name) = cache.begin(video_id, ext, total)?;

    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .await
        .map_err(|e| format!("could not open the stream cache: {e}"))?;

    let mut at = 0u64;
    let mut refreshes = 0u32;
    while at < total {
        let end = (at + FETCH_CHUNK).min(total);
        match download_range(resolver, &format, &mut file, at, end).await {
            Ok(()) => at = end,
            Err(FetchError::Auth) if refreshes < MAX_REFRESH_ATTEMPTS => {
                refreshes += 1;
                format = resolver.resolve(video_id, Some(format.itag)).await?;
            }
            Err(error) => {
                let _ = std::fs::remove_file(&path);
                return Err(format!("stream download failed: {error}"));
            }
        }
    }

    cache.mark_complete(video_id, &file_name, total);
    Ok(path)
}

async fn download_range(
    resolver: &Arc<UrlResolver>,
    format: &YtAudioFormat,
    file: &mut tokio::fs::File,
    start: u64,
    end: u64,
) -> Result<(), FetchError> {
    let mut response = resolver
        .http
        .get(&format.url)
        .header(RANGE, format!("bytes={start}-{}", end - 1))
        .header(USER_AGENT, format.user_agent)
        .header(ACCEPT, "*/*")
        .send()
        .await
        .map_err(|e| FetchError::Other(format!("range request: {e}")))?;

    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN
        || status == reqwest::StatusCode::UNAUTHORIZED
        || status == reqwest::StatusCode::GONE
        || status == reqwest::StatusCode::NOT_FOUND
    {
        return Err(FetchError::Auth);
    }
    if status != reqwest::StatusCode::PARTIAL_CONTENT {
        return Err(FetchError::Other(format!(
            "range request returned {status}"
        )));
    }

    let mut at = start;
    while at < end {
        let chunk = tokio::time::timeout(FETCH_STALL, response.chunk())
            .await
            .map_err(|_| FetchError::Other("the stream stalled".into()))?
            .map_err(|e| FetchError::Other(format!("range body: {e}")))?;
        let Some(bytes) = chunk else {
            break;
        };
        if bytes.is_empty() {
            continue;
        }
        let take = bytes.len().min((end - at) as usize);
        file.seek(SeekFrom::Start(at))
            .await
            .map_err(|e| FetchError::Other(format!("cache seek: {e}")))?;
        file.write_all(&bytes[..take])
            .await
            .map_err(|e| FetchError::Other(format!("cache write: {e}")))?;
        at += take as u64;
    }
    if at < end {
        return Err(FetchError::Other("the stream ended early".into()));
    }
    Ok(())
}

pub struct UrlResolver {
    pub ytmusic: Arc<YtMusic>,
    pub runtime: Arc<tokio::runtime::Runtime>,
    pub http: reqwest::Client,
    pub data_dir: PathBuf,
}

impl UrlResolver {
    pub async fn resolve(
        &self,
        video_id: &str,
        itag: Option<u32>,
    ) -> Result<YtAudioFormat, String> {
        if let Some(itag) = itag {
            if itag == 251 {
                let format = webm_format(&self.ytmusic, video_id).await?;
                if format.itag != 251 {
                    return Err("YouTube no longer offers the WebM stream for this track".into());
                }
                return Ok(format);
            }
            let format = self
                .ytmusic
                .best_audio(video_id)
                .await
                .map_err(|e| format!("stream url refresh failed: {}", super::sanitize_error(&e.to_string())))?;
            if format.itag != itag {
                return Err("YouTube changed the stream format for this track".into());
            }
            return Ok(format);
        }

        let webm = webm_format(&self.ytmusic, video_id).await;
        if let Ok(format) = &webm
            && url_serves(&self.http, format).await
        {
            return Ok(format.clone());
        }

        match self.ytmusic.best_audio(video_id).await {
            Ok(format) if url_serves(&self.http, &format).await => return Ok(format),
            Ok(_) => {}
            Err(error) => {
                if webm.is_ok() {
                    eprintln!("catalogue: webm stream unusable for {video_id}: {error:#}");
                }
            }
        }

        self.ytdlp_format(video_id).await
    }

    pub fn resolve_blocking(
        &self,
        video_id: &str,
        itag: Option<u32>,
    ) -> Result<YtAudioFormat, String> {
        self.runtime.block_on(self.resolve(video_id, itag))
    }

    async fn ytdlp_format(&self, video_id: &str) -> Result<YtAudioFormat, String> {
        let ytdlp = crate::services::downloader::resolve_tool("yt-dlp", &self.data_dir)
            .ok_or_else(|| "yt-dlp is required to stream this track.".to_string())?;
        let watch_url = format!("https://music.youtube.com/watch?v={video_id}");
        let mut command = crate::services::process::hidden_tokio(tokio::process::Command::new(
            &ytdlp,
        ));
        command
            .args([
                "--no-playlist",
                "--no-warnings",
                "--no-progress",
                "-f",
                "bestaudio/best",
                "--print",
                "%(url)s\t%(ext)s\t%(filesize,filesize_approx)s\t%(duration)s",
            ])
            .arg(&watch_url)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let output = tokio::time::timeout(Duration::from_secs(30), command.output())
            .await
            .map_err(|_| "yt-dlp timed out while resolving the stream".to_string())?
            .map_err(|e| format!("yt-dlp failed: {}", super::sanitize_error(&e.to_string())))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let line = stderr
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("yt-dlp could not resolve this track");
            return Err(super::sanitize_error(line.trim()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout
            .lines()
            .rev()
            .find(|l| l.split('\t').count() >= 4)
            .ok_or_else(|| "yt-dlp did not return a stream url".to_string())?;
        let fields: Vec<&str> = line.split('\t').collect();
        let url = fields[0].trim().to_string();
        if url.is_empty() || url == "NA" {
            return Err("yt-dlp did not return a stream url".into());
        }
        let ext = fields[1].trim();
        let size = fields[2].trim().parse::<u64>().ok();
        let duration = fields[3]
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|d| d.is_finite() && *d > 0.0)
            .map(Duration::from_secs_f64);

        let (content_length, mime) = probe_content(&self.http, &url, UA)
            .await
            .unwrap_or((size, None));
        let content_length =
            content_length.ok_or_else(|| "the fallback stream has no length".to_string())?;
        let mime = mime.unwrap_or_else(|| match ext {
            "webm" | "opus" => "audio/webm".to_string(),
            "mp4" | "m4a" => "audio/mp4".to_string(),
            "mp3" => "audio/mpeg".to_string(),
            _ => "audio/webm".to_string(),
        });

        Ok(YtAudioFormat {
            itag: 0,
            url,
            mime,
            codec: String::new(),
            bitrate: 0,
            duration,
            content_length: Some(content_length),
            loudness_db: None,
            user_agent: UA,
        })
    }
}

async fn webm_format(ytmusic: &YtMusic, video_id: &str) -> Result<YtAudioFormat, String> {
    let response = ytmusic
        .player_response(video_id, Client::VisionOs)
        .await
        .map_err(|e| format!("player response failed: {}", super::sanitize_error(&e.to_string())))?;
    let formats = response
        .pointer("/streamingData/adaptiveFormats")
        .and_then(Value::as_array)
        .ok_or_else(|| "this track has no audio streams".to_string())?;
    let format = formats
        .iter()
        .find(|format| format.get("itag").and_then(Value::as_u64) == Some(251))
        .or_else(|| {
            formats.iter().find(|format| {
                format
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .is_some_and(|mime| mime.starts_with("audio/webm"))
            })
        })
        .ok_or_else(|| "this track has no WebM stream".to_string())?;

    let url = format
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| "the WebM stream has no direct url".to_string())?
        .to_string();
    let mime = format
        .get("mimeType")
        .and_then(Value::as_str)
        .unwrap_or("audio/webm")
        .to_string();
    let codec = mime
        .split_once("codecs=\"")
        .map(|(_, tail)| tail.trim_end_matches('"'))
        .unwrap_or_default()
        .to_string();
    let content_length = parse_u64(format.get("contentLength"));
    let duration = parse_u64(format.get("approxDurationMs")).map(Duration::from_millis);

    Ok(YtAudioFormat {
        itag: format.get("itag").and_then(Value::as_u64).unwrap_or(251) as u32,
        url,
        mime,
        codec,
        bitrate: format.get("bitrate").and_then(Value::as_u64).unwrap_or(0) as u32,
        duration,
        content_length,
        loudness_db: format
            .get("loudnessDb")
            .and_then(Value::as_f64)
            .map(|value| value as f32),
        user_agent: Client::VisionOs.user_agent(),
    })
}

fn parse_u64(value: Option<&Value>) -> Option<u64> {
    let value = value?;
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<u64>().ok()))
}

async fn url_serves(http: &reqwest::Client, format: &YtAudioFormat) -> bool {
    let request = http
        .get(&format.url)
        .header(RANGE, "bytes=0-1023")
        .header(USER_AGENT, format.user_agent)
        .header(ACCEPT, "*/*")
        .send();
    match tokio::time::timeout(Duration::from_secs(12), request).await {
        Ok(Ok(response)) => {
            response.status() == reqwest::StatusCode::PARTIAL_CONTENT
                || (response.status() == reqwest::StatusCode::OK
                    && format.content_length.unwrap_or(0) <= 1024)
        }
        _ => false,
    }
}

async fn probe_content(
    http: &reqwest::Client,
    url: &str,
    agent: &str,
) -> Option<(Option<u64>, Option<String>)> {
    let response = tokio::time::timeout(
        Duration::from_secs(15),
        http.get(url)
            .header(RANGE, "bytes=0-0")
            .header(USER_AGENT, agent)
            .header(ACCEPT, "*/*")
            .send(),
    )
    .await
    .ok()?
    .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(';').next().unwrap_or(value).trim().to_string());
    let total = response
        .headers()
        .get(CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit_once('/'))
        .and_then(|(_, total)| total.trim().parse::<u64>().ok());
    Some((total, mime))
}

#[derive(Clone, Serialize, Deserialize)]
struct CacheEntry {
    file: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    complete: bool,
    #[serde(default)]
    last_used: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct CacheIndex {
    #[serde(default = "index_version")]
    version: u32,
    #[serde(default)]
    entries: HashMap<String, CacheEntry>,
}

fn index_version() -> u32 {
    1
}

struct CacheState {
    index: CacheIndex,
    active: HashSet<String>,
    dirty: bool,
}

pub struct StreamCache {
    dir: PathBuf,
    state: Mutex<CacheState>,
}

impl StreamCache {
    pub fn open(dir: PathBuf) -> Arc<Self> {
        let _ = std::fs::create_dir_all(&dir);
        let index_path = dir.join("index.json");
        let index: CacheIndex = std::fs::read(&index_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .filter(|index: &CacheIndex| index.version == index_version())
            .unwrap_or_else(|| CacheIndex {
                version: index_version(),
                entries: HashMap::new(),
            });

        let cache = Arc::new(Self {
            dir,
            state: Mutex::new(CacheState {
                index,
                active: HashSet::new(),
                dirty: false,
            }),
        });
        cache.cleanup();
        cache
    }

    fn cleanup(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let mut keep: HashMap<String, CacheEntry> = HashMap::new();
        for (video_id, entry) in state.index.entries.drain() {
            let path = self.dir.join(&entry.file);
            if entry.complete && path.is_file() {
                keep.insert(video_id, entry);
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
        state.index.entries = keep;

        if let Ok(read) = std::fs::read_dir(&self.dir) {
            for entry in read.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == "index.json" || !entry.path().is_file() {
                    continue;
                }
                if !state.index.entries.values().any(|e| e.file == name) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        state.dirty = true;
        self.save_locked(&mut state);
    }

    fn index_path(&self) -> PathBuf {
        self.dir.join("index.json")
    }

    fn save_locked(&self, state: &mut CacheState) {
        if !state.dirty {
            return;
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&state.index) {
            let _ = std::fs::write(self.index_path(), bytes);
            state.dirty = false;
        }
    }

    pub fn lookup_complete(&self, video_id: &str) -> Option<PathBuf> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let entry = state.index.entries.get(video_id)?.clone();
        if !entry.complete {
            return None;
        }
        let path = self.dir.join(&entry.file);
        if !path.is_file() {
            state.index.entries.remove(video_id);
            state.dirty = true;
            self.save_locked(&mut state);
            return None;
        }
        if let Some(entry) = state.index.entries.get_mut(video_id) {
            entry.last_used = now_secs();
        }
        state.dirty = true;
        self.save_locked(&mut state);
        Some(path)
    }

    pub fn begin(&self, video_id: &str, ext: &str, total: u64) -> Result<(PathBuf, String), String> {
        let serial = FILE_SERIAL.fetch_add(1, Ordering::Relaxed);
        let file_name = format!("{}-{}-{}.{}", video_id, now_secs(), serial, ext);
        let path = self.dir.join(&file_name);
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| format!("could not create the stream cache: {e}"))?;
        file.set_len(total)
            .map_err(|e| format!("could not prepare the stream cache: {e}"))?;
        drop(file);

        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.index.entries.insert(
            video_id.to_string(),
            CacheEntry {
                file: file_name.clone(),
                size: total,
                complete: false,
                last_used: now_secs(),
            },
        );
        state.active.insert(video_id.to_string());
        state.dirty = true;
        self.save_locked(&mut state);
        Ok((path, file_name))
    }

    pub fn mark_complete(&self, video_id: &str, file_name: &str, total: u64) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let Some(entry) = state.index.entries.get_mut(video_id) else {
            return;
        };
        if entry.file != file_name {
            return;
        }
        entry.complete = true;
        entry.size = total;
        entry.last_used = now_secs();
        state.dirty = true;
        self.save_locked(&mut state);
    }

    pub fn finish(&self, video_id: &str, file_name: &str, complete: bool) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.active.remove(video_id);
        let keep = state
            .index
            .entries
            .get(video_id)
            .is_some_and(|entry| entry.file == file_name && (entry.complete || complete));
        if keep {
            if let Some(entry) = state.index.entries.get_mut(video_id) {
                entry.complete = true;
                entry.last_used = now_secs();
            }
        } else if state
            .index
            .entries
            .get(video_id)
            .is_some_and(|entry| entry.file == file_name)
        {
            state.index.entries.remove(video_id);
        }
        let path = self.dir.join(file_name);
        if !keep {
            let _ = std::fs::remove_file(&path);
        }
        state.dirty = true;
        self.evict_locked(&mut state);
        self.save_locked(&mut state);
    }

    fn evict_locked(&self, state: &mut CacheState) {
        let mut total: u64 = state.index.entries.values().map(|e| e.size).sum();
        if total <= CACHE_LIMIT {
            return;
        }
        let mut order: Vec<(String, u64)> = state
            .index
            .entries
            .iter()
            .map(|(id, entry)| (id.clone(), entry.last_used))
            .collect();
        order.sort_by_key(|(_, used)| *used);
        for (video_id, _) in order {
            if total <= CACHE_LIMIT {
                break;
            }
            if state.active.contains(&video_id) {
                continue;
            }
            if let Some(entry) = state.index.entries.remove(&video_id) {
                let _ = std::fs::remove_file(self.dir.join(&entry.file));
                total = total.saturating_sub(entry.size);
                state.dirty = true;
            }
        }
    }
}

#[derive(Default)]
pub struct SessionControl {
    cancelled: AtomicBool,
    complete: AtomicBool,
    fetched: AtomicU64,
}

impl SessionControl {
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

struct RangeState {
    coverage: Vec<(u64, u64)>,
    total: u64,
    demand_start: u64,
    demand_end: u64,
    demand_gen: u64,
    error: Option<String>,
}

pub struct RangeStore {
    state: Mutex<RangeState>,
    cv: Condvar,
    control: Arc<SessionControl>,
}

impl RangeStore {
    fn new(total: u64, control: Arc<SessionControl>) -> Self {
        Self {
            state: Mutex::new(RangeState {
                coverage: Vec::new(),
                total,
                demand_start: 0,
                demand_end: 0,
                demand_gen: 0,
                error: None,
            }),
            cv: Condvar::new(),
            control,
        }
    }

    fn covered(state: &RangeState, start: u64, end: u64) -> bool {
        state
            .coverage
            .iter()
            .any(|&(a, b)| a <= start && end <= b)
    }

    fn mark(&self, start: u64, end: u64) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        mark_range(&mut state.coverage, start, end);
        if !self.control.complete.load(Ordering::Relaxed)
            && Self::covered(&state, 0, state.total)
        {
            self.control.complete.store(true, Ordering::SeqCst);
        }
        self.cv.notify_all();
    }

    fn ensure(&self, start: u64, end: u64) -> Result<(), String> {
        let end = end.min(self.state.lock().unwrap().total);
        if start >= end {
            return Ok(());
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if Self::covered(&state, start, end) {
            return Ok(());
        }
        state.demand_start = start;
        state.demand_end = end;
        state.demand_gen = state.demand_gen.wrapping_add(1);
        self.cv.notify_all();

        loop {
            if Self::covered(&state, start, end) {
                return Ok(());
            }
            if let Some(error) = &state.error {
                return Err(error.clone());
            }
            if self.control.is_cancelled() {
                return Err("stream cancelled".into());
            }
            let (guard, _) = self
                .cv
                .wait_timeout(state, Duration::from_millis(100))
                .unwrap_or_else(|e| e.into_inner());
            state = guard;
        }
    }

    fn wait_demand(&self, last_gen: u64) -> Option<(u64, u64, u64)> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if self.control.is_cancelled() || state.error.is_some() {
                return None;
            }
            if state.demand_gen != last_gen
                && !Self::covered(&state, state.demand_start, state.demand_end)
            {
                return Some((
                    state.demand_start,
                    state.demand_end,
                    state.demand_gen,
                ));
            }
            let (guard, _) = self
                .cv
                .wait_timeout(state, Duration::from_millis(200))
                .unwrap_or_else(|e| e.into_inner());
            state = guard;
        }
    }

    fn fail(&self, message: String) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.error.is_none() {
            state.error = Some(message);
        }
        self.cv.notify_all();
    }

    fn is_complete(&self) -> bool {
        self.control.complete.load(Ordering::Relaxed)
    }
}

fn mark_range(ranges: &mut Vec<(u64, u64)>, start: u64, end: u64) {
    if start >= end {
        return;
    }
    ranges.push((start, end));
    ranges.sort_unstable();
    let mut merged: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (a, b) in ranges.drain(..) {
        if let Some(last) = merged.last_mut()
            && a <= last.1
        {
            last.1 = last.1.max(b);
            continue;
        }
        merged.push((a, b));
    }
    *ranges = merged;
}

fn first_gap(ranges: &[(u64, u64)], start: u64, end: u64) -> Option<u64> {
    let mut at = start;
    for &(a, b) in ranges {
        if b <= at {
            continue;
        }
        if a > at {
            return Some(at);
        }
        if b >= end {
            return None;
        }
        at = b;
    }
    if at < end { Some(at) } else { None }
}

fn fetch_loop(
    store: Arc<RangeStore>,
    resolver: Arc<UrlResolver>,
    video_id: String,
    mut format: YtAudioFormat,
    mut file: File,
    cache: Arc<StreamCache>,
    file_name: String,
) {
    let mut last_gen = 0u64;
    let mut attempts = 0u32;
    let mut refreshes = 0u32;

    loop {
        let Some((demand_start, demand_end, generation)) = store.wait_demand(last_gen) else {
            return;
        };
        last_gen = generation;

        let total = {
            let state = store.state.lock().unwrap_or_else(|e| e.into_inner());
            state.total
        };
        let snapshot = {
            let state = store.state.lock().unwrap_or_else(|e| e.into_inner());
            state.coverage.clone()
        };
        let Some(gap_start) = first_gap(&snapshot, demand_start, total) else {
            continue;
        };
        if gap_start >= demand_end {
            continue;
        }
        let fetch_end = (gap_start + FETCH_CHUNK.max(READAHEAD.min(total)))
            .min(total)
            .max(demand_end.min(total));
        let result = run_fetch(
            &store,
            &resolver,
            &mut format,
            &mut file,
            gap_start,
            fetch_end,
        );

        match result {
            Ok(()) => {
                attempts = 0;
                if store.is_complete() {
                    cache.mark_complete(&video_id, &file_name, total);
                }
            }
            Err(FetchError::Cancelled) => return,
            Err(FetchError::Auth) if refreshes < MAX_REFRESH_ATTEMPTS => {
                refreshes += 1;
                match resolver.resolve_blocking(
            &video_id,
            (format.itag != 0).then_some(format.itag),
        ) {
                    Ok(next) => format = next,
                    Err(error) => {
                        store.fail(format!("the stream url expired: {error}"));
                        return;
                    }
                }
            }
            Err(FetchError::Auth) => {
                store.fail("the stream url expired and could not be renewed".into());
                return;
            }
            Err(error) => {
                attempts += 1;
                if attempts >= MAX_FETCH_ATTEMPTS {
                    store.fail(format!("stream download failed: {error}"));
                    return;
                }
                std::thread::sleep(Duration::from_millis(250 * attempts as u64));
            }
        }
    }
}

enum FetchError {
    Auth,
    Cancelled,
    Other(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Auth => f.write_str("the stream url was refused"),
            FetchError::Cancelled => f.write_str("cancelled"),
            FetchError::Other(message) => f.write_str(message),
        }
    }
}

fn run_fetch(
    store: &Arc<RangeStore>,
    resolver: &Arc<UrlResolver>,
    format: &mut YtAudioFormat,
    file: &mut File,
    start: u64,
    end: u64,
) -> Result<(), FetchError> {
    let http = &resolver.http;
    let runtime = resolver.runtime.clone();
    let control = store.control.clone();
    let store_task = store.clone();
    let url = format.url.clone();
    let agent = format.user_agent;
    let request = async move {
        let response = http
            .get(&url)
            .header(RANGE, format!("bytes={start}-{}", end - 1))
            .header(USER_AGENT, agent)
            .header(ACCEPT, "*/*")
            .send()
            .await
            .map_err(|e| FetchError::Other(format!("range request: {e}")))?;
        let status = response.status();
        if status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::GONE
            || status == reqwest::StatusCode::NOT_FOUND
        {
            return Err(FetchError::Auth);
        }
        if status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(FetchError::Other(format!("range request returned {status}")));
        }

        let mut response = response;
        let mut at = start;
        while at < end {
            if control.is_cancelled() {
                return Err(FetchError::Cancelled);
            }
            let chunk = tokio::time::timeout(FETCH_STALL, response.chunk())
                .await
                .map_err(|_| FetchError::Other("the stream stalled".into()))?
                .map_err(|e| FetchError::Other(format!("range body: {e}")))?;
            let Some(bytes) = chunk else {
                break;
            };
            if bytes.is_empty() {
                continue;
            }
            let take = bytes.len().min((end - at) as usize);
            file.seek(SeekFrom::Start(at))
                .map_err(|e| FetchError::Other(format!("cache seek: {e}")))?;
            file.write_all(&bytes[..take])
                .map_err(|e| FetchError::Other(format!("cache write: {e}")))?;
            at += take as u64;
            store_task.mark(at - take as u64, at);
            control.fetched.fetch_add(take as u64, Ordering::Relaxed);
            if at < end {
                continue;
            }
        }
        if at < end {
            return Err(FetchError::Other("the stream ended early".into()));
        }
        Ok(())
    };

    match runtime.block_on(async move {
        tokio::time::timeout(FETCH_STALL, request).await
    }) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(FetchError::Other("the range request timed out".into())),
    }
}

struct RangeReader {
    store: Arc<RangeStore>,
    file: File,
    pos: u64,
    total: u64,
}

impl Read for RangeReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.total || buf.is_empty() {
            return Ok(0);
        }
        let want = (buf.len() as u64).min(self.total - self.pos);
        self.store
            .ensure(self.pos, self.pos + want)
            .map_err(io::Error::other)?;
        self.file.seek(SeekFrom::Start(self.pos))?;
        self.file.read_exact(&mut buf[..want as usize])?;
        self.pos += want;
        Ok(want as usize)
    }
}

impl Seek for RangeReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let next = match pos {
            SeekFrom::Start(offset) => offset as i128,
            SeekFrom::End(offset) => self.total as i128 + offset as i128,
            SeekFrom::Current(offset) => self.pos as i128 + offset as i128,
        };
        if next < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "negative seek"));
        }
        self.pos = (next as u64).min(self.total);
        Ok(self.pos)
    }
}

impl MediaSource for RangeReader {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.total)
    }
}

struct BufferState {
    queue: VecDeque<f32>,
    capacity: usize,
    closed: bool,
    error: Option<String>,
    format: Option<(u32, u16)>,
}

struct StreamBuffer {
    state: Mutex<BufferState>,
    cv: Condvar,
}

impl StreamBuffer {
    fn new() -> Self {
        Self {
            state: Mutex::new(BufferState {
                queue: VecDeque::new(),
                capacity: (MAX_BUFFER_SECS * DEFAULT_RATE as f64 * DEFAULT_CHANNELS as f64) as usize,
                closed: false,
                error: None,
                format: None,
            }),
            cv: Condvar::new(),
        }
    }

    fn set_format(&self, rate: u32, channels: u16) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.format.is_none() {
            state.format = Some((rate.max(1), channels.max(1)));
            state.capacity =
                (MAX_BUFFER_SECS * rate as f64 * channels as f64).max(4096.0) as usize;
        }
        self.cv.notify_all();
    }

    fn push(&self, samples: &[f32], control: &SessionControl) -> bool {
        if samples.is_empty() {
            return true;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        while state.queue.len() + samples.len() > state.capacity {
            if control.is_cancelled() || state.closed {
                return false;
            }
            let (guard, _) = self
                .cv
                .wait_timeout(state, Duration::from_millis(150))
                .unwrap_or_else(|e| e.into_inner());
            state = guard;
        }
        if control.is_cancelled() || state.closed {
            return false;
        }
        state.queue.extend(samples.iter().copied());
        self.cv.notify_all();
        true
    }

    fn close(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.closed = true;
        self.cv.notify_all();
    }

    fn fail(&self, message: String) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.error.is_none() {
            state.error = Some(message);
        }
        state.closed = true;
        self.cv.notify_all();
    }

    fn pop(&self) -> Pop {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(sample) = state.queue.pop_front() {
            Pop::Sample(sample)
        } else if state.closed {
            Pop::Closed
        } else {
            Pop::Underrun
        }
    }

    fn buffered_secs(&self) -> f64 {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let (rate, channels) = state.format.unwrap_or((DEFAULT_RATE, DEFAULT_CHANNELS));
        state.queue.len() as f64 / (rate as f64 * channels as f64)
    }

    fn format(&self) -> Option<(u32, u16)> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .format
    }

    fn error(&self) -> Option<String> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .error
            .clone()
    }

    fn is_closed(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .closed
    }
}

enum Pop {
    Sample(f32),
    Underrun,
    Closed,
}

pub struct StreamSource {
    buffer: Arc<StreamBuffer>,
    channels: u16,
    sample_rate: u32,
    duration: Option<Duration>,
    skip: u64,
}

impl StreamSource {
    pub(crate) fn new(session: &StreamSession) -> Self {
        let (rate, channels) = session
            .buffer
            .format()
            .unwrap_or((DEFAULT_RATE, DEFAULT_CHANNELS));
        Self {
            buffer: session.buffer.clone(),
            channels,
            sample_rate: rate,
            duration: session
                .duration
                .filter(|d| d.is_finite() && *d > 0.0)
                .map(Duration::from_secs_f64),
            skip: 0,
        }
    }
}

impl Iterator for StreamSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        loop {
            if self.skip > 0 {
                match self.buffer.pop() {
                    Pop::Sample(_) => {
                        self.skip -= 1;
                        continue;
                    }
                    Pop::Underrun => {
                        self.skip += 1;
                        return Some(0.0);
                    }
                    Pop::Closed => return None,
                }
            }
            match self.buffer.pop() {
                Pop::Sample(sample) => return Some(sample),
                Pop::Underrun => {
                    self.skip += 1;
                    return Some(0.0);
                }
                Pop::Closed => return None,
            }
        }
    }
}

impl rodio::Source for StreamSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.duration
    }

    fn try_seek(&mut self, _pos: Duration) -> Result<(), rodio::source::SeekError> {
        Err(rodio::source::SeekError::NotSupported {
            underlying_source: "Vynl stream",
        })
    }
}

pub struct StreamSession {
    pub video_id: String,
    pub duration: Option<f64>,
    buffer: Arc<StreamBuffer>,
    control: Arc<SessionControl>,
    store: Arc<RangeStore>,
    cache: Arc<StreamCache>,
    file_name: String,
}

impl StreamSession {
    pub fn open(
        cache: Arc<StreamCache>,
        resolver: Arc<UrlResolver>,
        video_id: String,
        format: YtAudioFormat,
        start: f64,
        duration_hint: Option<f64>,
    ) -> Result<Arc<Self>, String> {
        let total = format
            .content_length
            .ok_or_else(|| "this stream has no known length".to_string())?;
        if total == 0 {
            return Err("this stream is empty".into());
        }
        let duration = format
            .duration
            .map(|d| d.as_secs_f64())
            .or(duration_hint)
            .filter(|d| d.is_finite() && *d > 0.0);
        let ext = ext_for(&format);
        let (path, file_name) = cache.begin(&video_id, ext, total)?;

        let write_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| format!("could not open the stream cache: {e}"))?;
        let read_file = File::open(&path)
            .map_err(|e| format!("could not open the stream cache: {e}"))?;

        let control = Arc::new(SessionControl::default());
        let buffer = Arc::new(StreamBuffer::new());
        let store = Arc::new(RangeStore::new(total, control.clone()));

        let fetch_store = store.clone();
        let fetch_resolver = resolver.clone();
        let fetch_cache = cache.clone();
        let fetch_video = video_id.clone();
        let fetch_name = file_name.clone();
        std::thread::Builder::new()
            .name("vynl-stream-fetch".into())
            .spawn(move || {
                fetch_loop(
                    fetch_store,
                    fetch_resolver,
                    fetch_video,
                    format,
                    write_file,
                    fetch_cache,
                    fetch_name,
                );
            })
            .map_err(|e| format!("could not start the stream downloader: {e}"))?;

        let decode_buffer = buffer.clone();
        let decode_control = control.clone();
        let decode_store = store.clone();
        let decode_video = video_id.clone();
        let decode_cache = cache.clone();
        let decode_name = file_name.clone();
        std::thread::Builder::new()
            .name("vynl-stream-decode".into())
            .spawn(move || {
                decode_loop(
                    decode_buffer,
                    decode_control,
                    decode_store,
                    read_file,
                    decode_video,
                    start,
                    ext,
                    decode_cache,
                    decode_name,
                    total,
                );
            })
            .map_err(|e| format!("could not start the stream decoder: {e}"))?;

        Ok(Arc::new(Self {
            video_id,
            duration,
            buffer,
            control,
            store,
            cache,
            file_name,
        }))
    }

    pub fn cancel(&self) {
        self.control.cancel();
        self.buffer.close();
    }

    pub fn buffered_secs(&self) -> f64 {
        self.buffer.buffered_secs()
    }

    pub fn is_ready(&self) -> bool {
        self.buffer.format().is_some()
    }

    pub fn is_finished(&self) -> bool {
        self.buffer.is_closed()
    }

    pub fn error(&self) -> Option<String> {
        self.buffer.error()
    }

    pub fn is_complete(&self) -> bool {
        self.store.is_complete()
    }
}

impl Drop for StreamSession {
    fn drop(&mut self) {
        self.cancel();
        let cache = self.cache.clone();
        let video_id = self.video_id.clone();
        let file_name = self.file_name.clone();
        let complete = self.store.is_complete();
        std::thread::spawn(move || cache.finish(&video_id, &file_name, complete));
    }
}

#[allow(clippy::too_many_arguments)]
fn decode_loop(
    buffer: Arc<StreamBuffer>,
    control: Arc<SessionControl>,
    store: Arc<RangeStore>,
    file: File,
    video_id: String,
    start: f64,
    ext: &'static str,
    cache: Arc<StreamCache>,
    file_name: String,
    total: u64,
) {
    let reader = RangeReader {
        store: store.clone(),
        file,
        pos: 0,
        total,
    };
    let source = MediaSourceStream::new(Box::new(reader), Default::default());
    let mut format_reader = match symphonia::default::get_probe().probe(
        &Hint::new().with_extension(ext),
        source,
        FormatOptions::default(),
        MetadataOptions::default(),
    ) {
        Ok(reader) => reader,
        Err(error) => {
            buffer.fail(format!("could not read this track: {error}"));
            return;
        }
    };

    let track = match format_reader
        .default_track(TrackType::Audio)
        .cloned()
    {
        Some(track) => track,
        None => {
            buffer.fail("this track has no audio".into());
            return;
        }
    };
    let params = match track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .cloned()
    {
        Some(params) => params,
        None => {
            buffer.fail("this track has no audio parameters".into());
            return;
        }
    };
    let sample_rate = params.sample_rate.unwrap_or(DEFAULT_RATE).max(1);
    let channels = params
        .channels
        .as_ref()
        .map(|channels| channels.count() as u16)
        .unwrap_or(DEFAULT_CHANNELS)
        .max(1);
    buffer.set_format(sample_rate, channels);

    let mut decoder = match audio::codec_registry()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
    {
        Ok(decoder) => decoder,
        Err(error) => {
            buffer.fail(format!("this audio format cannot be decoded: {error}"));
            return;
        }
    };

    let mut discard = 0usize;
    if start > 0.5
        && let Some(time) = Time::try_from_secs_f64(start)
    {
        match format_reader.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time,
                track_id: Some(track.id),
            },
        ) {
            Ok(seeked) => {
                let actual = track
                    .time_base
                    .map(|base| {
                        seeked.actual_ts.get() as f64 * base.numer.get() as f64
                            / base.denom.get() as f64
                    })
                    .unwrap_or(start);
                let skip = (start - actual).max(0.0);
                discard = (skip * sample_rate as f64 * channels as f64) as usize;
                decoder = match audio::codec_registry()
                    .make_audio_decoder(&params, &AudioDecoderOptions::default())
                {
                    Ok(decoder) => decoder,
                    Err(error) => {
                        buffer.fail(format!("this audio format cannot be decoded: {error}"));
                        return;
                    }
                };
            }
            Err(error) => {
                eprintln!("catalogue: seek in {video_id} failed: {error}");
            }
        }
    }

    let mut scratch: Vec<f32> = Vec::new();
    loop {
        if control.is_cancelled() {
            break;
        }
        let packet = match format_reader.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(error) => {
                buffer.fail(format!("stream read failed: {error}"));
                return;
            }
        };
        if packet.track_id != track.id {
            continue;
        }
        let audio = match decoder.decode(&packet) {
            Ok(audio) => audio,
            Err(error) => {
                eprintln!("catalogue: skipped undecodable packet for {video_id}: {error}");
                continue;
            }
        };
        let samples = audio.samples_interleaved();
        scratch.resize(samples, 0.0);
        audio.copy_to_slice_interleaved(&mut scratch[..samples]);
        let mut slice = &scratch[..samples];
        if discard > 0 {
            let trim = discard.min(slice.len());
            let trim = trim - trim % channels as usize;
            slice = &slice[trim..];
            discard -= trim;
        }
        if !buffer.push(slice, &control) {
            break;
        }
    }

    buffer.close();
    if store.is_complete() {
        cache.mark_complete(&video_id, &file_name, total);
    }
}

#[derive(Clone)]
pub enum PreparedAudio {
    Stream(Arc<StreamSession>),
    File(PathBuf),
}

#[derive(Clone)]
pub struct PreparedStream {
    pub video_id: String,
    pub duration: Option<f64>,
    pub start: f64,
    pub audio: PreparedAudio,
    pub cached: bool,
}

impl PreparedStream {
    pub fn session(&self) -> Option<&Arc<StreamSession>> {
        match &self.audio {
            PreparedAudio::Stream(session) => Some(session),
            PreparedAudio::File(_) => None,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match &self.audio {
            PreparedAudio::File(path) => Some(path),
            PreparedAudio::Stream(_) => None,
        }
    }
}

pub async fn prepare(
    cache: &Arc<StreamCache>,
    resolver: &Arc<UrlResolver>,
    video_id: &str,
    start: f64,
    duration_hint: Option<f64>,
) -> Result<PreparedStream, String> {
    let start = if start.is_finite() && start > 0.0 {
        start
    } else {
        0.0
    };

    if let Some(path) = cache.lookup_complete(video_id) {
        return Ok(PreparedStream {
            video_id: video_id.to_string(),
            duration: duration_hint,
            start,
            audio: PreparedAudio::File(path),
            cached: true,
        });
    }

    let format = resolver.resolve(video_id, None).await?;
    let session = StreamSession::open(
        cache.clone(),
        resolver.clone(),
        video_id.to_string(),
        format,
        start,
        duration_hint,
    )?;

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(error) = session.error() {
            return Err(error);
        }
        if session.is_ready() {
            if session.buffered_secs() >= 1.5 || session.is_finished() {
                break;
            }
            let duration = session.duration.unwrap_or(0.0);
            if duration > 0.0 && start + session.buffered_secs() >= duration - 1.0 {
                break;
            }
        }
        if std::time::Instant::now() >= deadline {
            if session.is_ready() && session.buffered_secs() > 0.0 {
                break;
            }
            session.cancel();
            return Err("the track took too long to start".into());
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    Ok(PreparedStream {
        video_id: video_id.to_string(),
        duration: session.duration.or(duration_hint),
        start,
        audio: PreparedAudio::Stream(session),
        cached: false,
    })
}

pub fn prepare_blocking(
    cache: &Arc<StreamCache>,
    resolver: &Arc<UrlResolver>,
    video_id: &str,
    start: f64,
    duration_hint: Option<f64>,
) -> Result<PreparedStream, String> {
    resolver
        .runtime
        .block_on(prepare(cache, resolver, video_id, start, duration_hint))
}

pub type PreparedSession = Weak<StreamSession>;
