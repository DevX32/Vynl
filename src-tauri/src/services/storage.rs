use std::path::Path;

use serde::Serialize;

pub fn clear_caches(user_data: &Path) -> u64 {
    let mut freed = 0u64;

    freed += clear_dir(&user_data.join("catalogue").join("covers"));
    freed += clear_dir(&crate::services::audio_cache::cache_dir());
    freed += crate::services::catalogue::clear_stream_cache();

    for name in ["lyrics-cache.json", "match-cache.json"] {
        let path = user_data.join(name);
        if let Ok(meta) = std::fs::metadata(&path) {
            freed += meta.len();
            let _ = std::fs::remove_file(&path);
        }
    }

    freed
}

fn clear_dir(dir: &Path) -> u64 {
    let mut freed = 0u64;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return freed;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        if std::fs::remove_file(&path).is_ok() {
            freed += size;
        }
    }

    freed
}

const AUDIO_EXTS: &[&str] = &["mp3", "m4a", "opus", "flac", "wav", "ogg", "aac"];

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadsStats {
    pub files: u64,
    pub bytes: u64,
}

pub fn downloads_stats(root: &Path) -> DownloadsStats {
    let mut files = 0u64;
    let mut bytes = 0u64;
    walk_audio(root, &mut files, &mut bytes, 0);
    DownloadsStats { files, bytes }
}

fn walk_audio(dir: &Path, files: &mut u64, bytes: &mut u64, depth: u32) {
    if depth > 8 {
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            walk_audio(&path, files, bytes, depth + 1);
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase);
        if !ext.as_deref().is_some_and(|ext| AUDIO_EXTS.contains(&ext)) {
            continue;
        }

        *files += 1;
        *bytes += entry.metadata().map(|meta| meta.len()).unwrap_or(0);
    }
}
