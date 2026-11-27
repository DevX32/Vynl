use std::path::Path;
use std::sync::Arc;

use tauri::Emitter;
use tokio::sync::Mutex;

use crate::services::{history, lyrics, util};

use super::*;

pub(super) async fn emit_final_summary(app_handle: &tauri::AppHandle, summary: &Arc<Mutex<DownloadSummary>>) {
    let s = summary.lock().await;
    let _ = app_handle.emit("vynl:dl", DownloadEvent::Summary { payload: s.clone() });

    if !s.cancelled && !CANCELLED_DOWNLOAD.load(Ordering::SeqCst) && (s.done > 0 || s.failed > 0) {
        let _ = app_handle.emit("vynl:toast", &*s);
    }
}

pub(super) async fn verify_track_duration(raw_path: &Path, track: &TrackMeta) -> Option<String> {
    let expected = track.duration?;
    if expected <= 0.0 {
        return None;
    }
    let real = probe_duration(raw_path).await?;
    if (real - expected).abs() / expected > VERIFY_TOLERANCE {
        Some(format!(
            "wrong length: got {}, expected {}",
            fmt_dur(real),
            fmt_dur(expected)
        ))
    } else {
        None
    }
}

pub(super) fn record_failure(
    user_data_dir: &Path,
    track: &TrackMeta,
    final_path: &Path,
    used_url: Option<&str>,
    message: &str,
) {
    let _ = history::record_download(
        user_data_dir,
        &HistoryEntry {
            ts: util::now_ms(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            file: final_path.to_string_lossy().to_string(),
            url: used_url.unwrap_or_default().to_string(),
            ok: false,
            message: Some(message.to_string()),
        },
    );
}

pub(super) async fn process_lyrics_lookup(
    track: &TrackMeta,
    user_data_dir: &Path,
) -> (Option<String>, Option<(&'static str, String)>) {
    let lookup = LyricsLookup {
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        duration: track.duration.unwrap_or(0.0),
    };
    let fetch_result = lyrics::fetch_lyrics_with_cache(&lookup, user_data_dir, false).await;
    match fetch_result {
        Some(lr) if !lr.text.trim().is_empty() => {
            let kind = lr.kind;
            let text = lr.text;
            match kind {
                crate::commands::types::LyricsKind::Lrc => {
                    let plain = LRC_META_RE
                        .replace_all(&LRC_TIME_RE.replace_all(&text, ""), "")
                        .trim()
                        .to_string();
                    let embed = if plain.is_empty() { None } else { Some(plain) };
                    (embed, Some(("lrc", text)))
                }
                crate::commands::types::LyricsKind::Ttml => (None, Some(("ttml", text))),
                crate::commands::types::LyricsKind::Txt => (Some(text), None),
            }
        }
        _ => (None, None),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn move_tagged_to_output(
    tagged_file: &Path,
    final_path: &Path,
    track: &TrackMeta,
    lyrics_sidecar: Option<(&str, &str)>,
    user_data_dir: &Path,
    collection: &Collection,
    used_url: Option<&str>,
    overwrite: bool,
) -> Result<(), String> {
    tokio::fs::create_dir_all(final_path.parent().unwrap_or(Path::new(".")))
        .await
        .map_err(|e| e.to_string())?;

    let backup = if final_path.exists() {
        if overwrite {
            let backup = final_path.with_extension(format!(
                "{}.overwrite-backup-{}",
                final_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or(""),
                std::process::id()
            ));
            match std::fs::rename(final_path, &backup) {
                Ok(()) => Some(backup),
                Err(error) => {
                    let _ = std::fs::remove_file(tagged_file);
                    return Err(format!("could not move the existing file aside: {error}"));
                }
            }
        } else {
            let _ = std::fs::remove_file(tagged_file);
            return Err("track already exists".to_string());
        }
    } else {
        None
    };

    if let Err(error) = move_file(tagged_file, final_path) {
        if let Some(backup) = &backup {
            let _ = std::fs::rename(backup, final_path);
        }
        return Err(error);
    }
    if let Some(backup) = backup {
        let _ = std::fs::remove_file(backup);
    }

    if let Some((ext, text)) = lyrics_sidecar
        && !text.trim().is_empty()
    {
        let canonical = final_path.to_string_lossy();
        let sidecar = lyrics::sidecar_path(&canonical, ext);
        if let Some(parent) = sidecar.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            log::warn!("lyrics sidecar mkdir failed for {parent:?}: {e}");
        }
        match std::fs::write(&sidecar, text) {
            Ok(()) => lyrics::remove_stale_sidecars(&canonical, ext),
            Err(e) => log::warn!("lyrics sidecar write failed for {sidecar:?}: {e}"),
        }
    }

    let key = history::collection_key(&collection.kind, &collection.id);
    let _ = history::mark_collection_done(user_data_dir, &key, &track.id);
    let _ = history::record_download(
        user_data_dir,
        &HistoryEntry {
            ts: util::now_ms(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            file: final_path.to_string_lossy().to_string(),
            url: used_url.unwrap_or_default().to_string(),
            ok: true,
            message: None,
        },
    );
    Ok(())
}

