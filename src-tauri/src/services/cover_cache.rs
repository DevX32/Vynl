use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::CONTENT_TYPE;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const CHUNK_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_BYTES: u64 = 15 * 1024 * 1024;

static HTTP: once_cell::sync::Lazy<reqwest::Client> = once_cell::sync::Lazy::new(|| {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(RESPONSE_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_default()
});

fn cache_dir(user_data: &Path) -> PathBuf {
    user_data.join("catalogue").join("covers")
}

pub fn normalize_cover_url(url: &str) -> std::borrow::Cow<'_, str> {
    fn digits(url: &str, start: usize) -> (usize, usize) {
        let bytes = url.as_bytes();
        let mut end = start;
        while bytes.get(end).is_some_and(|b| b.is_ascii_digit()) {
            end += 1;
        }
        (start, end)
    }

    let url = url.trim();
    let Some(start) = url.find("=w") else {
        return std::borrow::Cow::Borrowed(url);
    };

    let (w_start, w_end) = digits(url, start + 2);
    let Ok(width) = url[w_start..w_end].parse::<u64>() else {
        return std::borrow::Cow::Borrowed(url);
    };
    if w_end == w_start || !url[w_end..].starts_with("-h") {
        return std::borrow::Cow::Borrowed(url);
    }

    let (h_start, h_end) = digits(url, w_end + 2);
    let Ok(height) = url[h_start..h_end].parse::<u64>() else {
        return std::borrow::Cow::Borrowed(url);
    };
    if h_end == h_start {
        return std::borrow::Cow::Borrowed(url);
    }

    let mut params: Vec<&str> = Vec::new();
    for part in url[h_end..].split('-') {
        if !part.is_empty() && !params.contains(&part) {
            params.push(part);
        }
    }

    let new_width = width.max(512);
    let new_height = height.max(512);
    if new_width == width
        && new_height == height
        && params.len() == url[h_end..].matches('-').count()
    {
        return std::borrow::Cow::Borrowed(url);
    }

    let mut out = String::with_capacity(url.len() + 16);
    out.push_str(&url[..start]);
    out.push_str(&format!("=w{new_width}-h{new_height}"));
    for param in params {
        out.push('-');
        out.push_str(param);
    }
    std::borrow::Cow::Owned(out)
}

fn ext_for(content_type: &str) -> &'static str {
    if content_type.starts_with("image/png") {
        "png"
    } else if content_type.starts_with("image/webp") {
        "webp"
    } else if content_type.starts_with("image/gif") {
        "gif"
    } else if content_type.starts_with("image/avif") {
        "avif"
    } else {
        "jpg"
    }
}

fn existing(dir: &Path, digest: &str) -> Option<PathBuf> {
    ["jpg", "png", "webp", "gif", "avif"]
        .iter()
        .map(|ext| dir.join(format!("{digest}.{ext}")))
        .find(|path| path.is_file())
}

fn ytimg_fallback(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://i.ytimg.com/vi/")
        .or_else(|| url.strip_prefix("http://i.ytimg.com/vi/"))?;
    let (id, _) = rest.split_once('/')?;
    if id.is_empty() {
        return None;
    }
    Some(format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg"))
}

async fn fetch(url: &str) -> Result<reqwest::Response, String> {
    crate::services::util::follow_public_https(&HTTP, url, 5).await
}

pub async fn cache_cover(user_data: &Path, url: &str) -> Result<String, String> {
    let url = normalize_cover_url(url);
    let url = url.as_ref();
    if !url.starts_with("https://") {
        return Err("cover URL must use https".into());
    }
    if let Ok(parsed) = reqwest::Url::parse(url)
        && let Some(host) = parsed.host_str()
        && !crate::services::rpc::is_public_host(host)
    {
        return Err("cover URL points at a private host".into());
    }

    let dir = cache_dir(user_data);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cover cache dir: {e}"))?;

    let digest = hex::encode(Sha256::digest(url.as_bytes()));
    if let Some(path) = existing(&dir, &digest) {
        return Ok(path.to_string_lossy().into_owned());
    }

    let mut response = fetch(url).await?;

    if !response.status().is_success()
        && let Some(fallback) = ytimg_fallback(url)
        && let Ok(retry) = fetch(&fallback).await
        && retry.status().is_success()
    {
        response = retry;
    }

    if !response.status().is_success() {
        return Err(format!("cover download HTTP {}", response.status()));
    }

    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();

    if !content_type.starts_with("image/") {
        return Err(format!("unexpected cover content type: {content_type}"));
    }

    let ext = ext_for(&content_type);
    let dest = dir.join(format!("{digest}.{ext}"));
    if dest.is_file() {
        return Ok(dest.to_string_lossy().into_owned());
    }

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let tmp = dir.join(format!("{digest}.{unique}.tmp"));
    let mut response = response;
    let mut file = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| format!("cover cache write: {e}"))?;

    let mut written = 0u64;
    loop {
        match tokio::time::timeout(CHUNK_TIMEOUT, response.chunk()).await {
            Ok(Ok(Some(chunk))) => {
                written += chunk.len() as u64;
                if written > MAX_BYTES {
                    drop(file);
                    let _ = tokio::fs::remove_file(&tmp).await;
                    return Err("cover is too large".into());
                }
                file.write_all(&chunk)
                    .await
                    .map_err(|e| format!("cover cache write: {e}"))?;
            }
            Ok(Ok(None)) => break,
            Ok(Err(e)) => {
                drop(file);
                let _ = tokio::fs::remove_file(&tmp).await;
                return Err(format!("cover download body: {e}"));
            }
            Err(_) => {
                drop(file);
                let _ = tokio::fs::remove_file(&tmp).await;
                return Err("cover download stalled".into());
            }
        }
    }

    file.flush()
        .await
        .map_err(|e| format!("cover cache write: {e}"))?;
    drop(file);

    if let Err(error) = std::fs::rename(&tmp, &dest) {
        let _ = std::fs::remove_file(&tmp);
        if dest.is_file() {
            return Ok(dest.to_string_lossy().into_owned());
        }
        return Err(format!("cover cache rename: {error}"));
    }

    Ok(dest.to_string_lossy().into_owned())
}
