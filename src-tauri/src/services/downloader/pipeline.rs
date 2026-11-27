use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{Mutex, Semaphore};

use crate::commands::types::*;
use crate::services::{audio, history, process as sysproc};

use super::finalize::{move_tagged_to_output, process_lyrics_lookup, record_failure, verify_track_duration};
use super::fs::final_output_path;
use super::media::{emit_track, fetch_cover_now, CoverMap};
use super::paths::{audio_format_ext, tmp_dir};
use super::process::resolve_tool;
use super::request::{base_args, find_downloaded_stream, is_safe_candidate_url, sanitize_key_component};
use super::search::search_and_rank_candidates;
use super::tagging::tag;
use super::{CANCELLED_DOWNLOAD, CHILDREN, DOWNLOAD_STALL_TIMEOUT, DOWNLOAD_WORKERS, ProcessResult, TEMP_KEY_GEN, describe_no_audio_format, last_error, progress_from_line, watch_for_stall};

pub(super) async fn download_track(
    track: &TrackMeta,
    candidates: &[SearchCandidate],
    picked: Option<usize>,
    settings: &Settings,
    user_data_dir: &Path,
    generation: u64,
    track_event: impl Fn(&TrackMeta, TrackStatus, f64, Option<&str>) + Send + Sync + 'static,
) -> ProcessResult {
    let ytdlp = match resolve_tool("yt-dlp", user_data_dir) {
        Some(p) => p,
        None => {
            return ProcessResult {
                ok: false,
                message: "yt-dlp is required to download.".into(),
                raw_path: None,
                used_url: None,
            };
        }
    };
    let tmp = tmp_dir(user_data_dir);
    let ext = audio_format_ext(&settings.format);
    let download_key = format!(
        "{}-{}",
        sanitize_key_component(&track.id),
        TEMP_KEY_GEN.fetch_add(1, Ordering::Relaxed)
    );
    let raw_path = tmp.join(format!("{}.{}", download_key, ext));
    let tagged_path = tmp.join(format!("{}.tagged.{}", download_key, ext));

    for f in [&raw_path, &tagged_path] {
        let _ = std::fs::remove_file(f);
    }

    let start = picked.unwrap_or(0);
    let attempts: Vec<&SearchCandidate> = candidates.iter().skip(start).collect();

    if attempts.is_empty() {
        return ProcessResult {
            ok: false,
            message: "No matching result found to download.".into(),
            raw_path: None,
            used_url: None,
        };
    }

    let mut failures: Vec<String> = Vec::new();
    let track_event = Arc::new(track_event);

    for cand in &attempts {
        if CANCELLED_DOWNLOAD.load(Ordering::Relaxed) {
            break;
        }

        let source_label = match cand.source {
            MatchSource::YouTubeMusic => "youtube music",
            MatchSource::YouTube => "youtube",
        };
        let msg = match &cand.channel {
            Some(ch) => format!("{} ┬╖ {}", source_label, ch),
            None => source_label.to_string(),
        };
        track_event(track, TrackStatus::Searching, 0.0, Some(&msg));

        let mut args = base_args(&settings.format);
        args.push("-o".into());
        args.push(
            tmp.join(format!("{}.%(ext)s", download_key))
                .to_string_lossy()
                .to_string(),
        );
        if !is_safe_candidate_url(&cand.url) {
            failures.push(format!("rejected unsafe source URL: {}", cand.url));
            continue;
        }
        args.push("--".into());
        args.push(cand.url.clone());

        let mut command = sysproc::hidden_tokio(Command::new(&ytdlp));
        command
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                failures.push(e.to_string());
                continue;
            }
        };

        let id = match child.id() {
            Some(id) if id > 0 => id,
            Some(id) => {
                log::warn!(
                    "[downloader] Warning: child.id() returned {id}, attempting alternative tracking"
                );
                let _ = child.kill().await;
                continue;
            }
            None => {
                let _ = child.kill().await;
                continue;
            }
        };
        {
            let mut children = CHILDREN.lock().await;
            children.push((id, generation));
        }

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let track_clone = track.clone();
        let event_fn = Arc::clone(&track_event);

        let last_activity = Arc::new(std::sync::Mutex::new(std::time::Instant::now()));
        let stalled = Arc::new(AtomicBool::new(false));
        let watch_done = Arc::new(AtomicBool::new(false));
        let _watchdog = tokio::spawn(watch_for_stall(
            id,
            Arc::clone(&last_activity),
            Arc::clone(&stalled),
            Arc::clone(&watch_done),
        ));

        let stderr_task = tokio::spawn(async move {
            let mut buf = String::new();
            if let Some(err) = stderr {
                let mut reader = BufReader::new(err);
                reader.read_to_string(&mut buf).await.ok();
            }
            buf
        });

        let mut stdout_task = None;
        if let Some(out) = stdout {
            let track_for_stdout = track_clone.clone();
            let event_for_stdout = Arc::clone(&event_fn);
            let last_activity = Arc::clone(&last_activity);
            stdout_task = Some(tokio::spawn(async move {
                let mut reader = BufReader::new(out);
                let mut last_emit = std::time::Instant::now();
                let mut last_pct: f64 = -1.0;
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                    let mut activity = last_activity.lock().unwrap_or_else(|e| e.into_inner());
                    *activity = std::time::Instant::now();
                    if let Some(pct) = progress_from_line(&line) {
                        let now = std::time::Instant::now();
                        if now.duration_since(last_emit).as_millis() > 150 && pct != last_pct {
                            last_emit = now;
                            last_pct = pct;
                            event_for_stdout(
                                &track_for_stdout,
                                TrackStatus::Downloading,
                                pct,
                                None,
                            );
                        }
                    }
                }
            }));
        }

        let code = child.wait().await.ok().and_then(|s| s.code());
        watch_done.store(true, Ordering::Relaxed);

        {
            let mut children = CHILDREN.lock().await;
            children.retain(|&(c, _)| c != id);
        }

        let exited_cleanly = code == Some(0);
        if stalled.load(Ordering::Relaxed) && !exited_cleanly {
            for f in [
                &raw_path,
                &tagged_path,
                &tmp.join(format!("{}.part", download_key)),
            ] {
                let _ = std::fs::remove_file(f);
            }
            if let Some(stream) = find_downloaded_stream(&tmp, &download_key) {
                let _ = std::fs::remove_file(stream);
            }
            let msg = format!(
                "stalled: no progress for {} seconds",
                DOWNLOAD_STALL_TIMEOUT.as_secs()
            );
            track_event(track, TrackStatus::Searching, 0.0, Some(&msg));
            failures.push(msg);
            if let Some(task) = stdout_task {
                task.abort();
            }
            stderr_task.abort();
            continue;
        }

        if let Some(task) = stdout_task {
            let _ = task.await;
        }
        let stderr_text = stderr_task.await.unwrap_or_default();

        let stream_path = find_downloaded_stream(&tmp, &download_key);
        let keep_native = crate::services::downloader::keep_native(&settings.format);

        if let Some(stream) = stream_path.clone()
            && code == Some(0)
        {
            let same_file = keep_native && stream == raw_path;
            if !same_file {
                let _ = std::fs::remove_file(&raw_path);
            }
            track_event(
                track,
                TrackStatus::Processing,
                100.0,
                Some(if keep_native {
                    "packaging original audioΓÇª"
                } else {
                    "encoding audioΓÇª"
                }),
            );

            let bitrate = settings.bitrate.filter(|b| *b > 0).map(|b| b as u32);
            let encoded = if keep_native {
                if same_file {
                    Ok(())
                } else {
                    let remux_src = stream.clone();
                    let remux_dest = raw_path.clone();
                    let remux = tokio::task::spawn_blocking(move || {
                        crate::services::opusmux::remux(&remux_src, &remux_dest).map(|_| ())
                    })
                    .await
                    .map_err(|e| format!("Remux task failed: {e}"))
                    .and_then(|r| r);
                    match remux {
                        Ok(()) => Ok(()),
                        Err(remux_error) => {
                            log::error!(
                                "opusmux refused the source ({remux_error}); transcoding to mp3 instead"
                            );
                            let fallback_src = stream.clone();
                            let fallback_dest = raw_path.clone();
                            tokio::task::spawn_blocking(move || {
                                audio::transcode_to_mp3(
                                    &fallback_src,
                                    &fallback_dest,
                                    bitrate,
                                    Some(&CANCELLED_DOWNLOAD),
                                )
                            })
                            .await
                            .map_err(|e| format!("Encoding task failed: {e}"))
                            .and_then(|r| r)
                        }
                    }
                }
            } else {
                let transcode_src = stream.clone();
                let transcode_dest = raw_path.clone();
                tokio::task::spawn_blocking(move || {
                    audio::transcode_to_mp3(
                        &transcode_src,
                        &transcode_dest,
                        bitrate,
                        Some(&CANCELLED_DOWNLOAD),
                    )
                })
                .await
                .map_err(|e| format!("Encoding task failed: {e}"))
                .and_then(|r| r)
            };

            if !same_file {
                let _ = std::fs::remove_file(&stream);
            }

            match encoded {
                Ok(()) => {
                    track_event(
                        track,
                        TrackStatus::Processing,
                        100.0,
                        Some("verifying lengthΓÇª"),
                    );
                    if let Some(err_msg) = verify_track_duration(&raw_path, track).await {
                        let _ = std::fs::remove_file(&raw_path);
                        failures.push(err_msg);
                        continue;
                    }
                    return ProcessResult {
                        ok: true,
                        message: String::new(),
                        raw_path: Some(raw_path.to_string_lossy().to_string()),
                        used_url: Some(cand.url.clone()),
                    };
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&raw_path);
                    if CANCELLED_DOWNLOAD.load(Ordering::Relaxed) {
                        break;
                    }
                    failures.push(e);
                    continue;
                }
            }
        }

        if let Some(stream) = stream_path {
            let _ = std::fs::remove_file(&stream);
        }
        for f in [
            &raw_path,
            &tagged_path,
            &tmp.join(format!("{}.part", download_key)),
        ] {
            let _ = std::fs::remove_file(f);
        }
        failures.push(
            describe_no_audio_format(&stderr_text).unwrap_or_else(|| last_error(&stderr_text)),
        );
    }

    ProcessResult {
        ok: false,
        message: failures
            .last()
            .cloned()
            .unwrap_or_else(|| "Unknown error".into()),
        raw_path: None,
        used_url: None,
    }
}

pub(super) enum TrackOutcome {
    Done,
    Skipped,
    Cancelled,
    Failed {
        message: String,
        used_url: Option<String>,
    },
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn process_track(
    idx: usize,
    collection: &Collection,
    settings: &Settings,
    user_data_dir: &Path,
    tmp: &Path,
    opts_matches: Option<&HashMap<String, Vec<SearchCandidate>>>,
    opts_picks: Option<&HashMap<String, i64>>,
    skip_ids: &HashSet<String>,
    ytdlp: &str,
    app: &tauri::AppHandle,
    prefetched_covers: &CoverMap,
    history_lookup: &history::HistoryLookup,
    generation: u64,
) -> TrackOutcome {
    let track = &collection.tracks[idx];

    if !skip_ids.is_empty() && skip_ids.contains(&track.id) {
        emit_track(
            app,
            track,
            TrackStatus::Skipped,
            0.0,
            Some("already synced"),
        );
        return TrackOutcome::Skipped;
    }

    let final_path = final_output_path(track, settings);

    if final_path.exists() && !settings.overwrite {
        emit_track(app, track, TrackStatus::Skipped, 0.0, None);
        return TrackOutcome::Skipped;
    }

    if !settings.overwrite && history_lookup.has_downloaded_same_song(&track.title, &track.artist) {
        emit_track(
            app,
            track,
            TrackStatus::Skipped,
            0.0,
            Some("already downloaded"),
        );
        return TrackOutcome::Skipped;
    }

    let has_cached_match = opts_matches
        .map(|m| m.contains_key(&track.id))
        .unwrap_or(false);

    if !has_cached_match {
        emit_track(
            app,
            track,
            TrackStatus::Searching,
            0.0,
            Some("finding matchesΓÇª"),
        );
    }

    let picked = opts_picks
        .and_then(|p| p.get(&track.id))
        .map(|&i| i as usize);

    let candidates =
        search_and_rank_candidates(ytdlp, track, opts_matches, picked, user_data_dir).await;

    let app_for_dl = app.clone();
    let result = download_track(
        track,
        &candidates,
        picked,
        settings,
        user_data_dir,
        generation,
        move |t, s, p, m| emit_track(&app_for_dl, t, s, p, m),
    )
    .await;

    if CANCELLED_DOWNLOAD.load(Ordering::Relaxed) {
        emit_track(app, track, TrackStatus::Cancelled, 0.0, None);
        return TrackOutcome::Cancelled;
    }

    if !result.ok {
        return TrackOutcome::Failed {
            message: result.message,
            used_url: result.used_url,
        };
    }

    emit_track(app, track, TrackStatus::Processing, 100.0, None);

    let raw_path = match result.raw_path {
        Some(path) => PathBuf::from(path),
        None => {
            return TrackOutcome::Failed {
                message: "download finished but no file was produced".into(),
                used_url: result.used_url,
            };
        }
    };
    let used_url = result.used_url.clone();

    let cover_path = {
        let prefetched = prefetched_covers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&track.id)
            .cloned();
        match prefetched {
            Some(cover) => cover,
            None => fetch_cover_now(track, collection, tmp).await,
        }
    };

    let (lyrics_text, lyrics_sidecar) = process_lyrics_lookup(track, user_data_dir).await;

    let mut tagged_file = raw_path.clone();
    tagged_file.set_extension(format!("tagged.{}", audio_format_ext(&settings.format)));

    {
        let cover_str = cover_path.as_ref().map(|p| p.to_string_lossy().to_string());
        let cover_arg = cover_str.as_deref();
        match tag(
            &raw_path.to_string_lossy(),
            &tagged_file.to_string_lossy(),
            track,
            cover_arg,
            lyrics_text.as_deref(),
        )
        .await
        {
            Ok(()) => {}
            Err(_) if CANCELLED_DOWNLOAD.load(Ordering::Relaxed) => {
                if let Some(cp) = &cover_path {
                    let _ = std::fs::remove_file(cp);
                }
                emit_track(app, track, TrackStatus::Cancelled, 0.0, None);
                return TrackOutcome::Cancelled;
            }
            Err(_) => {
                emit_track(
                    app,
                    track,
                    TrackStatus::Processing,
                    100.0,
                    Some("tagging failed, saving untaggedΓÇª"),
                );
                tagged_file = raw_path.clone();
            }
        }
    }

    if let Some(cp) = &cover_path {
        let _ = std::fs::remove_file(cp);
    }

    if CANCELLED_DOWNLOAD.load(Ordering::Relaxed) {
        let _ = std::fs::remove_file(&tagged_file);
        let _ = std::fs::remove_file(&raw_path);
        emit_track(app, track, TrackStatus::Cancelled, 0.0, None);
        return TrackOutcome::Cancelled;
    }

    match move_tagged_to_output(
        &tagged_file,
        &final_path,
        track,
        lyrics_sidecar
            .as_ref()
            .map(|(ext, text)| (*ext, text.as_str())),
        user_data_dir,
        collection,
        used_url.as_deref(),
        settings.overwrite,
    )
    .await
    {
        Ok(()) => {
            emit_track(app, track, TrackStatus::Done, 100.0, None);
            TrackOutcome::Done
        }
        Err(e) => TrackOutcome::Failed {
            message: e,
            used_url,
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_pass(
    queue: Arc<Mutex<VecDeque<usize>>>,
    collection: &Collection,
    settings: &Settings,
    user_data_dir: &Path,
    tmp: &Path,
    opts_matches: Option<&HashMap<String, Vec<SearchCandidate>>>,
    opts_picks: Option<&HashMap<String, i64>>,
    skip_ids: &HashSet<String>,
    ytdlp: &str,
    app_handle: &tauri::AppHandle,
    prefetched_covers: &CoverMap,
    summary: &Arc<Mutex<DownloadSummary>>,
    generation: u64,
    is_final_attempt: bool,
    retry_out: &Arc<Mutex<Vec<usize>>>,
) {
    let semaphore = Arc::new(Semaphore::new(DOWNLOAD_WORKERS));
    let history_lookup = Arc::new(history::HistoryLookup::load(user_data_dir));
    let mut handles = Vec::new();

    for _ in 0..DOWNLOAD_WORKERS {
        let queue = Arc::clone(&queue);
        let semaphore = Arc::clone(&semaphore);
        let summary = Arc::clone(summary);
        let retry_out = Arc::clone(retry_out);
        let collection = collection.clone();
        let settings = settings.clone();
        let user_data_dir = user_data_dir.to_path_buf();
        let tmp = tmp.to_path_buf();
        let opts_matches = opts_matches.cloned();
        let opts_picks = opts_picks.cloned();
        let skip_ids = skip_ids.clone();
        let ytdlp = ytdlp.to_string();
        let app = app_handle.clone();
        let prefetched_covers = Arc::clone(prefetched_covers);
        let history_lookup = Arc::clone(&history_lookup);

        handles.push(tokio::spawn(async move {
            let _permit = match semaphore.acquire().await {
                Ok(p) => p,
                Err(_) => return,
            };
            loop {
                if CANCELLED_DOWNLOAD.load(Ordering::Relaxed) {
                    break;
                }
                let idx = {
                    let mut q = queue.lock().await;
                    q.pop_front()
                };
                let idx = match idx {
                    Some(i) => i,
                    None => break,
                };

                let outcome = process_track(
                    idx,
                    &collection,
                    &settings,
                    &user_data_dir,
                    &tmp,
                    opts_matches.as_ref(),
                    opts_picks.as_ref(),
                    &skip_ids,
                    &ytdlp,
                    &app,
                    &prefetched_covers,
                    &history_lookup,
                    generation,
                )
                .await;

                let track = &collection.tracks[idx];
                match outcome {
                    TrackOutcome::Done => {
                        summary.lock().await.done += 1;
                    }
                    TrackOutcome::Skipped => {
                        summary.lock().await.skipped += 1;
                    }
                    TrackOutcome::Cancelled => {
                        summary.lock().await.cancelled = true;
                    }
                    TrackOutcome::Failed { message, used_url } => {
                        if is_final_attempt {
                            summary.lock().await.failed += 1;
                            emit_track(&app, track, TrackStatus::Error, 0.0, Some(&message));
                            let final_path = final_output_path(track, &settings);
                            record_failure(
                                &user_data_dir,
                                track,
                                &final_path,
                                used_url.as_deref(),
                                &message,
                            );
                        } else {
                            emit_track(&app, track, TrackStatus::Queued, 0.0, Some("retryingΓÇª"));
                            retry_out.lock().await.push(idx);
                        }
                    }
                }
                {
                    let s = summary.lock().await;
                    let _ = app.emit("vynl:dl", DownloadEvent::Summary { payload: s.clone() });
                }
            }
        }));
    }

    for h in handles {
        let _ = h.await;
    }
}
