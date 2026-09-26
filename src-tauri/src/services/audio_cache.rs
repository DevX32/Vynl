use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

static HTTP: once_cell::sync::Lazy<reqwest::Client> = once_cell::sync::Lazy::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default()
});

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const CHUNK_STALL_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("vynl")
        .join("session-audio")
}

fn safe_name(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let digest = hex::encode(hasher.finalize());
    let ext = Path::new(key)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("audio");
    format!("{digest}.{ext}")
}

pub async fn cache_remote_audio(url: &str, key: &str) -> Result<String, String> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("cache dir: {e}"))?;

    let dest = dir.join(safe_name(key));
    if dest.exists() {
        if let Ok(meta) = dest.metadata() {
            if meta.len() > 0 {
                return Ok(dest.to_string_lossy().into_owned());
            }
        }
    }

    let tmp = dest.with_extension("partial");
    let mut resp = tokio::time::timeout(RESPONSE_TIMEOUT, HTTP.get(url).send())
        .await
        .map_err(|_| format!("download timed out after {}s", RESPONSE_TIMEOUT.as_secs()))?
        .map_err(|e| format!("download failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("download HTTP {}", resp.status()));
    }

    let mut file = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| format!("write cache: {e}"))?;
    let streamed = stream_body(&mut resp, &mut file).await;
    drop(file);

    if let Err(e) = streamed {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(e);
    }

    let _ = std::fs::remove_file(&dest);
    std::fs::rename(&tmp, &dest).map_err(|e| format!("rename cache: {e}"))?;

    Ok(dest.to_string_lossy().into_owned())
}

async fn stream_body(
    resp: &mut reqwest::Response,
    file: &mut tokio::fs::File,
) -> Result<(), String> {
    loop {
        match tokio::time::timeout(CHUNK_STALL_TIMEOUT, resp.chunk()).await {
            Ok(Ok(Some(chunk))) => {
                file.write_all(&chunk)
                    .await
                    .map_err(|e| format!("write cache: {e}"))?;
            }
            Ok(Ok(None)) => break,
            Ok(Err(e)) => return Err(format!("download body: {e}")),
            Err(_) => {
                return Err(format!(
                    "download stalled: no data for {}s",
                    CHUNK_STALL_TIMEOUT.as_secs()
                ))
            }
        }
    }
    file.flush().await.map_err(|e| format!("write cache: {e}"))
}

pub fn clear_cache() -> Result<(), String> {
    let dir = cache_dir();
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("clear cache: {e}"))?;
    }
    Ok(())
}
