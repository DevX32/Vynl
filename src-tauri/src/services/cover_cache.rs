use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::{ACCEPT, CONTENT_TYPE, USER_AGENT};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const CHUNK_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_BYTES: u64 = 15 * 1024 * 1024;
const USER_AGENT_VALUE: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

static HTTP: once_cell::sync::Lazy<reqwest::Client> = once_cell::sync::Lazy::new(|| {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(RESPONSE_TIMEOUT)
        .build()
        .unwrap_or_default()
});

fn cache_dir(user_data: &Path) -> PathBuf {
    user_data.join("catalogue").join("covers")
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
    HTTP.get(url)
        .header(USER_AGENT, USER_AGENT_VALUE)
        .header(
            ACCEPT,
            "image/avif,image/webp,image/apng,image/*,*/*;q=0.8",
        )
        .send()
        .await
        .map_err(|e| format!("cover download failed: {e}"))
}

pub async fn cache_cover(user_data: &Path, url: &str) -> Result<String, String> {
    let url = url.trim();
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("unsupported cover URL".into());
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
