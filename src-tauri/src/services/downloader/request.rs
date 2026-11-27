use std::path::{Path, PathBuf};

use crate::commands::types::AudioFormat;
use super::{keep_native, STREAM_EXTS, UA, search::common_yt_args};

pub(super) fn sanitize_key_component(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect()
}


pub(super) fn is_safe_candidate_url(url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty() || trimmed.starts_with('-') {
        return false;
    }
    if !trimmed.starts_with("https://") {
        return false;
    }
    let Ok(parsed) = reqwest::Url::parse(trimmed) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    crate::services::rpc::is_public_host(host)
        && crate::services::util::host_resolves_publicly(host)
}


pub(super) fn base_args(format: &AudioFormat) -> Vec<String> {
    let audio_format = if keep_native(format) {
        "bestaudio[ext=webm]/bestaudio[ext=m4a]/bestaudio".into()
    } else {
        "bestaudio[ext=webm]/bestaudio[ext=m4a]/bestaudio/best".into()
    };
    let mut args = vec![
        "--no-playlist".into(),
        "--format".into(),
        audio_format,
        "--newline".into(),
    ];

    args.extend(common_yt_args());

    args.push("--retries".into());
    args.push("10".into());
    args.push("--fragment-retries".into());
    args.push("10".into());
    args.push("--concurrent-fragments".into());
    args.push("4".into());
    args.push("--user-agent".into());
    args.push(UA.into());

    if cfg!(target_os = "windows") {
        args.push("--windows-filenames".into());
    }

    args
}


pub(super) fn find_downloaded_stream(tmp: &Path, track_id: &str) -> Option<PathBuf> {
    let prefix = format!("{track_id}.");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(tmp).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(ext) = name.strip_prefix(&prefix) else {
            continue;
        };
        if !STREAM_EXTS.contains(&ext.to_ascii_lowercase().as_str()) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() || meta.len() == 0 {
            continue;
        }
        let modified = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
        if best.as_ref().is_none_or(|(t, _)| modified >= *t) {
            best = Some((modified, entry.path()));
        }
    }
    best.map(|(_, path)| path)
}

