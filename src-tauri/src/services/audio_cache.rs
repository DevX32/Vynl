use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;

static HTTP: once_cell::sync::Lazy<reqwest::Client> = once_cell::sync::Lazy::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_default()
});

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const CHUNK_STALL_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_BYTES: usize = 512 * 1024 * 1024;
const MAX_CACHE_FILES: usize = 300;
const MAX_CACHE_BYTES: u64 = 512 * 1024 * 1024;

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
    if !url.starts_with("https://") {
        return Err("audio URL must use https".into());
    }
    if let Ok(parsed) = reqwest::Url::parse(url)
        && let Some(host) = parsed.host_str()
        && !crate::services::rpc::is_public_host(host)
    {
        return Err("audio URL points at a private host".into());
    }
    let dir = cache_dir();
    let dir = tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&dir).map_err(|e| format!("cache dir: {e}"))?;
        prune_cache_dir(&dir);
        Ok::<PathBuf, String>(dir)
    })
    .await
    .map_err(|e| format!("cache setup: {e}"))??;

    let dest = dir.join(safe_name(key));
    if dest.exists()
        && let Ok(meta) = dest.metadata()
        && meta.len() > 0
    {
        return Ok(dest.to_string_lossy().into_owned());
    }

    static TMP_COUNTER: once_cell::sync::Lazy<std::sync::atomic::AtomicU64> =
        once_cell::sync::Lazy::new(|| std::sync::atomic::AtomicU64::new(0));
    let unique = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = dir.join(format!(
        "{}.partial.{}.{}",
        safe_name(key),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
        unique
    ));
    let mut resp = tokio::time::timeout(
        RESPONSE_TIMEOUT,
        crate::services::util::follow_public_https(&HTTP, url, 5),
    )
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

    match std::fs::rename(&tmp, &dest) {
        Ok(()) => {}
        Err(rename_error) => {
            if dest.is_file() {
                let _ = std::fs::remove_file(&dest);
                std::fs::rename(&tmp, &dest).map_err(|e| format!("rename cache: {e}"))?;
            } else {
                let _ = std::fs::remove_file(&tmp);
                return Err(format!("rename cache: {rename_error}"));
            }
        }
    }

    Ok(dest.to_string_lossy().into_owned())
}

async fn stream_body(
    resp: &mut reqwest::Response,
    file: &mut tokio::fs::File,
) -> Result<(), String> {
    let mut written: usize = 0;
    loop {
        match tokio::time::timeout(CHUNK_STALL_TIMEOUT, resp.chunk()).await {
            Ok(Ok(Some(chunk))) => {
                written += chunk.len();
                if written > MAX_BYTES {
                    return Err("download exceeded the size limit".into());
                }
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
                ));
            }
        }
    }
    file.flush().await.map_err(|e| format!("write cache: {e}"))
}

fn prune_cache_dir(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    let mut files: Vec<(PathBuf, u64, u64)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.contains(".partial."))
        {
            continue;
        }
        let Ok(meta) = path.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        files.push((path, mtime, meta.len()));
    }

    files.sort_by_key(|(_, mtime, _)| *mtime);

    let total_bytes: u64 = files.iter().map(|(_, _, len)| *len).sum();
    let mut excess_files = files.len().saturating_sub(MAX_CACHE_FILES);
    let mut excess_bytes = total_bytes.saturating_sub(MAX_CACHE_BYTES);

    for (path, _, len) in files {
        if excess_files == 0 && excess_bytes == 0 {
            break;
        }
        let _ = std::fs::remove_file(&path);
        excess_files = excess_files.saturating_sub(1);
        excess_bytes = excess_bytes.saturating_sub(len);
    }
}

pub fn clear_cache() -> Result<(), String> {
    let dir = cache_dir();
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("clear cache: {e}"))?;
    }
    Ok(())
}
