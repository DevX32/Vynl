use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use ytmusic::{AudioFormat as YtAudioFormat, Client, YtMusic};

use tokio::io::{AsyncSeekExt, AsyncWriteExt};

use reqwest::header::{ACCEPT, CONTENT_RANGE, RANGE, USER_AGENT};

use super::cache::StreamCache;
use super::store::{FETCH_CHUNK, FETCH_STALL, FetchError, MAX_REFRESH_ATTEMPTS};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

pub(super) fn ext_for(format: &YtAudioFormat) -> &'static str {
    let mime = format.mime.to_ascii_lowercase();
    if mime.contains("webm") {
        "webm"
    } else if mime.contains("mp4")
        || mime.contains("m4a")
        || format.itag == 140
        || format.itag == 139
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
    let begun = cache.begin(video_id, ext, total, format.itag, false)?;
    let (path, file_name) = (begun.path, begun.file_name);

    let mut file = match tokio::fs::OpenOptions::new().write(true).open(&path).await {
        Ok(file) => file,
        Err(error) => {
            let _ = std::fs::remove_file(&path);
            cache.finish(video_id, &file_name, false);
            return Err(format!("could not open the stream cache: {error}"));
        }
    };

    let mut at = 0u64;
    let mut refreshes = 0u32;
    while at < total {
        let end = (at + FETCH_CHUNK).min(total);
        match download_range(resolver, &format, &mut file, at, end).await {
            Ok(()) => at = end,
            Err(FetchError::Auth) if refreshes < MAX_REFRESH_ATTEMPTS => {
                refreshes += 1;
                let itag = (format.itag != 0).then_some(format.itag);
                match resolver.resolve(video_id, itag).await {
                    Ok(next) => {
                        if next.itag != format.itag || next.content_length != Some(total) {
                            let _ = std::fs::remove_file(&path);
                            cache.finish(video_id, &file_name, false);
                            return Err("stream format changed while downloading".into());
                        }
                        format = next;
                    }
                    Err(error) => {
                        let _ = std::fs::remove_file(&path);
                        cache.finish(video_id, &file_name, false);
                        return Err(format!("stream download failed: {error}"));
                    }
                }
            }
            Err(error) => {
                let _ = std::fs::remove_file(&path);
                cache.finish(video_id, &file_name, false);
                return Err(format!("stream download failed: {error}"));
            }
        }
    }

    cache.mark_complete(video_id, &file_name, total);
    cache.finish(video_id, &file_name, true);
    match cache.lookup_complete(video_id) {
        Some(canonical) => Ok(canonical),
        None => Ok(path),
    }
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
    pub formats: Mutex<HashMap<String, (YtAudioFormat, Instant)>>,
}

const FORMAT_TTL: Duration = Duration::from_secs(20 * 60);
const MAX_FORMAT_CACHE: usize = 256;

impl UrlResolver {
    pub(super) fn cached_format(&self, video_id: &str) -> Option<YtAudioFormat> {
        let mut formats = self.formats.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let fresh = formats.get(video_id).is_some_and(|(_, at)| {
            now.checked_duration_since(*at)
                .is_some_and(|age| age < FORMAT_TTL)
        });
        if fresh {
            return formats.get(video_id).map(|(format, _)| format.clone());
        }
        formats.remove(video_id);
        None
    }

    pub(super) fn remember_format(&self, video_id: &str, format: &YtAudioFormat) {
        let mut formats = self.formats.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        formats.retain(|_, (_, at)| {
            now.checked_duration_since(*at)
                .is_some_and(|age| age < FORMAT_TTL)
        });
        if formats.len() >= MAX_FORMAT_CACHE
            && !formats.contains_key(video_id)
            && let Some(oldest) = formats
                .iter()
                .min_by_key(|(_, (_, at))| *at)
                .map(|(id, _)| id.clone())
        {
            formats.remove(&oldest);
        }
        formats.insert(video_id.to_string(), (format.clone(), now));
    }

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
                self.remember_format(video_id, &format);
                return Ok(format);
            }
            let format = self.ytmusic.best_audio(video_id).await.map_err(|e| {
                format!(
                    "stream url refresh failed: {}",
                    crate::services::catalogue::sanitize_error(&e.to_string())
                )
            })?;
            if format.itag != itag {
                return Err("YouTube changed the stream format for this track".into());
            }
            self.remember_format(video_id, &format);
            return Ok(format);
        }

        if let Some(format) = self.cached_format(video_id) {
            return Ok(format);
        }

        let mut webm = webm_format(&self.ytmusic, video_id).await;
        if webm.is_err() {
            tokio::time::sleep(Duration::from_millis(400)).await;
            webm = webm_format(&self.ytmusic, video_id).await;
        }
        if let Ok(format) = &webm
            && url_serves(&self.http, format).await
        {
            self.remember_format(video_id, format);
            return Ok(format.clone());
        }

        let mut best = self.ytmusic.best_audio(video_id).await;
        if best.is_err() {
            tokio::time::sleep(Duration::from_millis(400)).await;
            best = self.ytmusic.best_audio(video_id).await;
        }
        match best {
            Ok(format) if url_serves(&self.http, &format).await => {
                self.remember_format(video_id, &format);
                return Ok(format);
            }
            Ok(_) => {}
            Err(error) => {
                if webm.is_ok() {
                    log::error!("catalogue: webm stream unusable for {video_id}: {error:#}");
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
        let mut command =
            crate::services::process::hidden_tokio(tokio::process::Command::new(&ytdlp));
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
            .map_err(|e| {
                format!(
                    "yt-dlp failed: {}",
                    crate::services::catalogue::sanitize_error(&e.to_string())
                )
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let line = stderr
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("yt-dlp could not resolve this track");
            return Err(crate::services::catalogue::sanitize_error(line.trim()));
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
            .and_then(|d| Duration::try_from_secs_f64(d).ok());

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
        .map_err(|e| {
            format!(
                "player response failed: {}",
                crate::services::catalogue::sanitize_error(&e.to_string())
            )
        })?;
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

pub(super) fn parse_u64(value: Option<&Value>) -> Option<u64> {
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
