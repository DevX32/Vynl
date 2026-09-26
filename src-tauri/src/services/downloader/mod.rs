use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use regex::Regex;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{Mutex, Semaphore};

use crate::commands::types::*;
use crate::services::{catalog, history, lyrics, process as sysproc, tools, util};

mod matching;
mod paths;
mod process;
mod search;

use matching::filter_and_rank_candidates;
use paths::tmp_dir;
pub use paths::{audio_format_ext, render_pattern, sanitize};
pub use process::cancel_download;
pub use process::resolve_tool;
use process::{
    fmt_dur, last_error, progress_from_line, watch_for_stall, CANCELLED_DOWNLOAD, CHILDREN,
};
pub use search::find_matches;
use search::{common_yt_args, search_and_rank_candidates};

const VERIFY_TOLERANCE: f64 = 0.10;

static PROGRESS_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"\[download\]\s+(\d+(?:\.\d+)?)%").unwrap());

static PAREN_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"(?i)\s*[\(\[].*?[\)\]]\s*").unwrap());

static FEAT_RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r"(?i)\s*[-–—]\s*(feat\.|ft\.|featuring)\s+.*$").unwrap()
});

static MULTI_SPACE_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"\s{2,}").unwrap());

static SHORT_LINK_RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r"https?://(spotify\.link|spotify\.app\.link)/").unwrap()
});

static LRC_TIME_RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r"(?m)^\[\d{1,2}:\d{2}(?:[.:]\d{1,3})?\]\s*").unwrap()
});

static LAST_ERROR_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"(?i)^ERROR:\s*(.+)$").unwrap());

static WHITESPACE_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"\s+").unwrap());

static TRAILING_DOT_SPACE_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"[. ]+$").unwrap());

static NON_ALNUM_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"[^a-z0-9]+").unwrap());

const UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

static BUSY: AtomicBool = AtomicBool::new(false);
static GENERATION: AtomicU64 = AtomicU64::new(0);

pub(super) fn clean_title(s: &str) -> String {
    let no_parens = PAREN_RE.replace_all(s, " ");
    let no_feat = FEAT_RE.replace(&no_parens, "");
    let cleaned = MULTI_SPACE_RE.replace_all(&no_feat, " ");
    cleaned.trim().to_string()
}

async fn follow_short_link(raw: &str) -> Option<catalog::CatalogLink> {
    let trimmed = raw.trim();
    if !SHORT_LINK_RE.is_match(trimmed) {
        return None;
    }

    if let Ok(parsed) = reqwest::Url::parse(trimmed) {
        match parsed.scheme() {
            "http" | "https" => {}
            _ => return None,
        }
        if let Some(host) = parsed.host_str() {
            let lower = host.to_lowercase();
            if lower == "localhost"
                || lower == "127.0.0.1"
                || lower == "::1"
                || lower.ends_with(".local")
                || lower.starts_with("10.")
                || lower.starts_with("192.168.")
                || lower.starts_with("172.")
            {
                return None;
            }
        }
    } else {
        return None;
    }

    let client = reqwest::Client::builder()
        .user_agent(UA)
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .ok()?;

    let resp = client.get(trimmed).send().await.ok()?;
    let location = resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if location.is_empty() {
        return None;
    }

    catalog::parse_link(location)
}

pub async fn resolve_link(raw_url: &str) -> Result<Collection, String> {
    let link = catalog::parse_link(raw_url)
        .or(async { follow_short_link(raw_url).await }.await)
        .ok_or_else(|| "That does not look like a track, album, or playlist link.".to_string())?;
    catalog::fetch_collection(&link).await
}

const DOWNLOAD_WORKERS: usize = 4;

const DOWNLOAD_STALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);

const SEARCH_BUDGET: std::time::Duration = std::time::Duration::from_secs(120);

const COVER_PREFETCH_WORKERS: usize = 8;

type CoverMap = Arc<std::sync::Mutex<HashMap<String, Option<PathBuf>>>>;

fn final_output_path(track: &TrackMeta, settings: &Settings) -> PathBuf {
    let base_name = sanitize(&render_pattern(&settings.filename_pattern, track));
    PathBuf::from(&settings.output_dir).join(format!(
        "{}.{}",
        base_name,
        audio_format_ext(&settings.format)
    ))
}

pub fn get_done_track_ids(
    collection: &Collection,
    user_data_dir: &Path,
    settings: Option<&Settings>,
) -> Vec<String> {
    let key = history::collection_key(&collection.kind, &collection.id);
    let history_ids: Vec<String> = history::collection_done(user_data_dir, &key);
    let history_set: HashSet<String> = history_ids.iter().cloned().collect();
    let mut done: HashSet<String> = history_set.clone();

    if let Some(s) = settings {
        if !s.output_dir.is_empty() {
            let output = Path::new(&s.output_dir);
            if output.exists() {
                for track in &collection.tracks {
                    let path = final_output_path(track, s);
                    if path.exists() {
                        done.insert(track.id.clone());
                    }
                }
            }
            done.retain(|id| {
                if history_set.contains(id) {
                    if let Some(track) = collection.tracks.iter().find(|t| &t.id == id) {
                        let path = final_output_path(track, s);
                        return path.exists();
                    }
                }
                true
            });
        }
    }

    let mut out: Vec<String> = done.into_iter().collect();
    out.sort();
    out
}

async fn probe_duration(ffprobe: &str, file: &str) -> Option<f64> {
    {
        use lofty::prelude::*;
        use lofty::probe::Probe;
        if let Ok(tf) = Probe::open(file).and_then(|p| p.read()) {
            let dur = tf.properties().duration();
            let secs = dur.as_secs_f64();
            if secs > 0.0 && secs.is_finite() {
                return Some(secs.round());
            }
        }
    }
    let args: Vec<String> = vec![
        "-v".into(),
        "error".into(),
        "-show_entries".into(),
        "format=duration".into(),
        "-of".into(),
        "default=noprint_wrappers=1:nokey=1".into(),
        file.into(),
    ];

    let mut command = sysproc::hidden_tokio(Command::new(ffprobe));
    command
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let output = command.output().await.ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    trimmed
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(|v| v.round())
}

async fn fetch_cover(url: &str, dest: &str) -> bool {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return false;
    }
    let client = match reqwest::Client::builder()
        .user_agent(UA)
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(_) => return false,
    };

    if !resp.status().is_success() {
        return false;
    }

    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(_) => return false,
    };

    tokio::fs::write(dest, &bytes).await.is_ok()
}

fn try_tag_with_lofty(
    audio_path: &str,
    track: &TrackMeta,
    _format: &AudioFormat,
    cover: Option<&str>,
    lyrics: Option<&str>,
) -> Result<(), String> {
    use lofty::picture::{MimeType, Picture, PictureType};
    use lofty::prelude::*;
    use lofty::probe::Probe;
    use lofty::tag::{ItemKey, TagType};
    use std::fs;

    let path = Path::new(audio_path);
    let mut tagged_file = Probe::open(path)
        .map_err(|e| format!("lofty probe failed: {e}"))?
        .read()
        .map_err(|e| format!("lofty read failed: {e}"))?;

    let tag_type = TagType::Id3v2;
    let tag = if let Some(primary_type) = tagged_file.primary_tag().map(|t| t.tag_type()) {
        if primary_type != tag_type {
            if tagged_file.tag(tag_type).is_none() {
                tagged_file.insert_tag(lofty::tag::Tag::new(tag_type));
            }
            tagged_file.tag_mut(tag_type).unwrap()
        } else {
            tagged_file.primary_tag_mut().unwrap()
        }
    } else {
        let new_tag = lofty::tag::Tag::new(tag_type);
        tagged_file.insert_tag(new_tag);
        tagged_file.primary_tag_mut().unwrap()
    };

    tag.set_title(track.title.clone());
    tag.set_artist(track.artist.clone());
    tag.set_album(track.album.clone());

    if let Some(year) = track.year {
        tag.insert_text(ItemKey::Year, year.to_string());
        tag.insert_text(ItemKey::RecordingDate, year.to_string());
    }
    if let Some(num) = track.track_number {
        tag.set_track(num as u32);
    }
    if let Some(l) = lyrics {
        if !l.trim().is_empty() {
            tag.insert_text(ItemKey::Lyrics, l.to_string());
            tag.insert_text(ItemKey::UnsyncLyrics, l.to_string());
        }
    }
    if let Some(cover_path) = cover {
        if let Ok(bytes) = fs::read(cover_path) {
            if !bytes.is_empty() {
                let mime = if cover_path.to_lowercase().ends_with(".png") {
                    MimeType::Png
                } else {
                    MimeType::Jpeg
                };
                let pic = Picture::unchecked(bytes)
                    .mime_type(mime)
                    .pic_type(PictureType::CoverFront)
                    .build();
                tag.remove_picture_type(PictureType::CoverFront);
                tag.push_picture(pic);
            }
        }
    }

    tagged_file
        .save_to_path(path, Default::default())
        .map_err(|e| format!("lofty save failed: {e}"))?;
    Ok(())
}

async fn tag(
    ffmpeg: &str,
    audio_in: &str,
    audio_out: &str,
    track: &TrackMeta,
    format: &AudioFormat,
    cover: Option<&str>,
    lyrics: Option<&str>,
) -> Result<(), String> {
    if audio_in != audio_out {
        if let Err(e) = tokio::fs::copy(audio_in, audio_out).await {
            eprintln!("lofty copy failed: {e}, falling back to ffmpeg");
        } else {
            let out = audio_out.to_string();
            let track_c = track.clone();
            let fmt_c = format.clone();
            let cover_c = cover.map(|s| s.to_string());
            let lyrics_c = lyrics.map(|s| s.to_string());
            let lofty_res = tokio::task::spawn_blocking(move || {
                try_tag_with_lofty(
                    &out,
                    &track_c,
                    &fmt_c,
                    cover_c.as_deref(),
                    lyrics_c.as_deref(),
                )
            })
            .await
            .map_err(|e| format!("lofty join failed: {e}"))
            .and_then(|r| r);

            match lofty_res {
                Ok(()) => return Ok(()),
                Err(e) => {
                    eprintln!("lofty tagging failed: {e}, falling back to ffmpeg");
                    let _ = tokio::fs::remove_file(audio_out).await;
                }
            }
        }
    }

    let use_cover = cover.is_some();
    let mut args: Vec<String> = vec!["-y".into(), "-i".into(), audio_in.into()];

    if let Some(c) = cover {
        if use_cover {
            args.push("-i".into());
            args.push(c.into());
        }
    }

    args.push("-map".into());
    args.push("0:a:0".into());

    if use_cover {
        args.push("-map".into());
        args.push("1:v:0".into());
        args.push("-c:v:0".into());
        args.push("mjpeg".into());
        args.push("-disposition:v:0".into());
        args.push("attached_pic".into());
    }

    args.push("-c:a".into());
    args.push("copy".into());

    if *format == AudioFormat::Mp3 {
        args.push("-id3v2_version".into());
        args.push("3".into());
    }

    let year_str = track.year.map(|y| y.to_string()).unwrap_or_default();
    let track_str = track
        .track_number
        .map(|n| n.to_string())
        .unwrap_or_default();

    let mut metadata: Vec<(&str, &str)> = vec![
        ("title", track.title.as_str()),
        ("artist", track.artist.as_str()),
        ("album", track.album.as_str()),
        ("album_artist", track.artist.as_str()),
        ("date", &year_str),
    ];
    if !track_str.is_empty() {
        metadata.push(("track", &track_str));
    }
    if let Some(l) = lyrics {
        metadata.push(("lyrics", l));
    }

    for (k, v) in &metadata {
        if !v.is_empty() {
            args.push("-metadata".into());
            args.push(format!("{}={}", k, v));
        }
    }

    args.push(audio_out.into());

    let mut command = sysproc::hidden_tokio(Command::new(ffmpeg));
    command
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = command
        .output()
        .await
        .map_err(|e| format!("Failed to run ffmpeg: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg tagging failed: {}", last_error(&stderr)));
    }

    Ok(())
}

struct ProcessResult {
    ok: bool,
    message: String,
    raw_path: Option<String>,
    used_url: Option<String>,
}

fn base_args(ffmpeg_dir: &str, format: &AudioFormat, bitrate: Option<i64>) -> Vec<String> {
    let ext = audio_format_ext(format);
    let quality = bitrate
        .map(|b| format!("{}K", b))
        .unwrap_or_else(|| "0".into());

    let mut args = vec![
        "--no-playlist".into(),
        "--format".into(),
        "bestaudio/best".into(),
        "--extract-audio".into(),
        "--audio-format".into(),
        ext.into(),
        "--audio-quality".into(),
        quality,
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
    args.push("--ffmpeg-location".into());
    args.push(ffmpeg_dir.into());

    if cfg!(target_os = "windows") {
        args.push("--windows-filenames".into());
    }

    args
}

async fn download_track(
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
    let ffmpeg = match resolve_tool("ffmpeg", user_data_dir) {
        Some(p) => p,
        None => {
            return ProcessResult {
                ok: false,
                message: "ffmpeg is required to download.".into(),
                raw_path: None,
                used_url: None,
            };
        }
    };

    let ffmpeg_dir = Path::new(&ffmpeg)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let tmp = tmp_dir(user_data_dir);
    let ext = audio_format_ext(&settings.format);
    let raw_path = tmp.join(format!("{}.{}", track.id, ext));
    let tagged_path = tmp.join(format!("{}.tagged.{}", track.id, ext));

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

        let source_label = "youtube";
        let msg = match &cand.channel {
            Some(ch) => format!("{} · {}", source_label, ch),
            None => source_label.to_string(),
        };
        track_event(track, TrackStatus::Searching, 0.0, Some(&msg));

        let mut args = base_args(&ffmpeg_dir, &settings.format, settings.bitrate);
        args.push("-o".into());
        args.push(
            tmp.join(format!("{}.%(ext)s", track.id))
                .to_string_lossy()
                .to_string(),
        );
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
                eprintln!("[downloader] Warning: child.id() returned {id}, attempting alternative tracking");
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

        if let Some(out) = stdout {
            let track_for_stdout = track_clone.clone();
            let event_for_stdout = Arc::clone(&event_fn);
            let last_activity = Arc::clone(&last_activity);
            tokio::spawn(async move {
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
                    if let Ok(mut activity) = last_activity.lock() {
                        *activity = std::time::Instant::now();
                    }
                    if line.contains("[ExtractAudio]") {
                        event_for_stdout(&track_for_stdout, TrackStatus::Processing, 100.0, None);
                        continue;
                    }
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
            });
        }

        let code = child.wait().await.ok().and_then(|s| s.code());
        watch_done.store(true, Ordering::Relaxed);

        {
            let mut children = CHILDREN.lock().await;
            children.retain(|&(c, _)| c != id);
        }

        let succeeded = raw_path.exists() && code == Some(0);
        if stalled.load(Ordering::Relaxed) && !succeeded {
            for f in [
                &raw_path,
                &tagged_path,
                &tmp.join(format!("{}.part", track.id)),
            ] {
                let _ = std::fs::remove_file(f);
            }
            let msg = format!(
                "stalled: no progress for {} seconds",
                DOWNLOAD_STALL_TIMEOUT.as_secs()
            );
            track_event(track, TrackStatus::Searching, 0.0, Some(&msg));
            failures.push(msg);
            continue;
        }

        let stderr_text = stderr_task.await.unwrap_or_default();

        if raw_path.exists() && code == Some(0) {
            if let Some(ffprobe) = tools::get_ffprobe_path(user_data_dir) {
                track_event(
                    track,
                    TrackStatus::Processing,
                    100.0,
                    Some("verifying length…"),
                );
                if let Some(err_msg) = verify_track_duration(&ffprobe, &raw_path, track).await {
                    for f in [
                        &raw_path,
                        &tagged_path,
                        &tmp.join(format!("{}.part", track.id)),
                    ] {
                        let _ = std::fs::remove_file(f);
                    }
                    failures.push(err_msg);
                    continue;
                }
            }
            return ProcessResult {
                ok: true,
                message: String::new(),
                raw_path: Some(raw_path.to_string_lossy().to_string()),
                used_url: Some(cand.url.clone()),
            };
        }

        for f in [
            &raw_path,
            &tagged_path,
            &tmp.join(format!("{}.part", track.id)),
        ] {
            let _ = std::fs::remove_file(f);
        }
        failures.push(last_error(&stderr_text));
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

fn emit_track(
    app: &tauri::AppHandle,
    t: &TrackMeta,
    status: TrackStatus,
    percent: f64,
    message: Option<&str>,
) {
    let payload = TrackProgress {
        track_id: t.id.clone(),
        status,
        percent,
        message: message.map(|m| m.to_string()),
    };
    let event = DownloadEvent::Track { payload };
    let _ = app.emit("vynl:dl", &event);
}

fn start_cover_prefetch(
    collection: &Collection,
    tmp: &Path,
) -> (CoverMap, tokio::task::JoinHandle<()>) {
    let covers: CoverMap = Arc::new(std::sync::Mutex::new(HashMap::new()));
    let semaphore = Arc::new(Semaphore::new(COVER_PREFETCH_WORKERS));
    let mut handles = Vec::new();

    for track in &collection.tracks {
        let cover_url = track.cover.as_ref().or(collection.cover.as_ref()).cloned();
        let track_id = track.id.clone();
        let tmp = tmp.to_path_buf();
        let covers = Arc::clone(&covers);
        let semaphore = Arc::clone(&semaphore);

        if let Some(url) = cover_url {
            handles.push(tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(_) => return,
                };
                let cp = tmp.join(format!("cover-{}.jpg", track_id));
                let _ = std::fs::remove_file(&cp);
                let success = fetch_cover(&url, &cp.to_string_lossy()).await;
                let mut map = covers.lock().unwrap_or_else(|e| e.into_inner());
                if success {
                    map.insert(track_id, Some(cp));
                } else {
                    let _ = std::fs::remove_file(&cp);
                    map.insert(track_id, None);
                }
            }));
        } else {
            let mut map = covers.lock().unwrap_or_else(|e| e.into_inner());
            map.insert(track_id, None);
        }
    }

    let joiner = tokio::spawn(async move {
        for handle in handles {
            let _ = handle.await;
        }
    });

    (covers, joiner)
}

async fn fetch_cover_now(
    track: &TrackMeta,
    collection: &Collection,
    tmp: &Path,
) -> Option<PathBuf> {
    let url = track.cover.as_ref().or(collection.cover.as_ref())?;
    let cp = tmp.join(format!("cover-late-{}.jpg", track.id));
    if fetch_cover(url, &cp.to_string_lossy()).await {
        Some(cp)
    } else {
        let _ = std::fs::remove_file(&cp);
        None
    }
}

enum TrackOutcome {
    Done,
    Skipped,
    Cancelled,
    Failed {
        message: String,
        used_url: Option<String>,
    },
}

#[allow(clippy::too_many_arguments)]
async fn process_track(
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
    gen: u64,
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

    let base_name = sanitize(&render_pattern(&settings.filename_pattern, track));
    let final_path = PathBuf::from(&settings.output_dir).join(format!(
        "{}.{}",
        base_name,
        audio_format_ext(&settings.format)
    ));

    if final_path.exists() && !settings.overwrite {
        emit_track(app, track, TrackStatus::Skipped, 0.0, None);
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
            Some("finding matches…"),
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
        gen,
        move |t, s, p, m| emit_track(&app_for_dl, t, s, p, m),
    )
    .await;

    if CANCELLED_DOWNLOAD.load(Ordering::Relaxed) {
        emit_track(app, track, TrackStatus::Cancelled, 0.0, None);
        return TrackOutcome::Cancelled;
    }

    if !result.ok || result.raw_path.is_none() {
        return TrackOutcome::Failed {
            message: result.message,
            used_url: result.used_url,
        };
    }

    emit_track(app, track, TrackStatus::Processing, 100.0, None);

    let raw_path = PathBuf::from(result.raw_path.as_ref().unwrap());
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

    let (lyrics_text, lyrics_lrc_text) =
        process_lyrics_lookup(track, &settings.format, user_data_dir).await;

    let ffmpeg = resolve_tool("ffmpeg", user_data_dir).unwrap_or_default();
    let mut tagged_file = tmp.join(format!(
        "{}.tagged.{}",
        track.id,
        audio_format_ext(&settings.format)
    ));

    {
        let cover_str = cover_path.as_ref().map(|p| p.to_string_lossy().to_string());
        let cover_arg = cover_str.as_deref();
        match tag(
            &ffmpeg,
            &raw_path.to_string_lossy(),
            &tagged_file.to_string_lossy(),
            track,
            &settings.format,
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
                    Some("tagging failed, saving untagged…"),
                );
                tagged_file = raw_path.clone();
            }
        }
    }

    if let Some(cp) = &cover_path {
        let _ = std::fs::remove_file(cp);
    }

    match move_tagged_to_output(
        &tagged_file,
        &final_path,
        track,
        lyrics_lrc_text.as_deref(),
        user_data_dir,
        collection,
        used_url.as_deref(),
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
async fn run_pass(
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
    gen: u64,
    is_final_attempt: bool,
    retry_out: &Arc<Mutex<Vec<usize>>>,
) {
    let semaphore = Arc::new(Semaphore::new(DOWNLOAD_WORKERS));
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
                    gen,
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
                            emit_track(&app, track, TrackStatus::Queued, 0.0, Some("retrying…"));
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

pub async fn download_collection(
    collection: Collection,
    settings: Settings,
    user_data_dir: &Path,
    app_handle: tauri::AppHandle,
    opts: DownloadOpts,
) -> Result<(), String> {
    if BUSY.swap(true, Ordering::SeqCst) {
        return Err("A download is already running.".into());
    }

    struct BusyGuard;
    impl Drop for BusyGuard {
        fn drop(&mut self) {
            BUSY.store(false, Ordering::SeqCst);
        }
    }
    let _guard = BusyGuard;

    download_collection_inner(collection, settings, user_data_dir, &app_handle, opts).await
}

async fn download_collection_inner(
    collection: Collection,
    settings: Settings,
    user_data_dir: &Path,
    app_handle: &tauri::AppHandle,
    opts: DownloadOpts,
) -> Result<(), String> {
    CANCELLED_DOWNLOAD.store(false, Ordering::SeqCst);
    let gen = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;

    let tmp = tmp_dir(user_data_dir);

    cleanup_tmp_dir(&tmp);

    let summary = Arc::new(Mutex::new(DownloadSummary {
        total: 0,
        done: 0,
        skipped: 0,
        failed: 0,
        cancelled: false,
    }));

    let (deduped_tracks, dup_skipped) = deduplicate_tracks(&collection.tracks, app_handle);
    {
        let mut s = summary.lock().await;
        s.skipped += dup_skipped as i64;
        s.total = deduped_tracks.len() as i64 + s.skipped;
    }
    let collection = Collection {
        tracks: deduped_tracks,
        ..collection
    };

    let ytdlp = resolve_tool("yt-dlp", user_data_dir)
        .ok_or_else(|| "yt-dlp and ffmpeg are required to download.".to_string())?;
    let _ffmpeg = resolve_tool("ffmpeg", user_data_dir)
        .ok_or_else(|| "yt-dlp and ffmpeg are required to download.".to_string())?;

    let (prefetched_covers, covers_task) = start_cover_prefetch(&collection, &tmp);

    let retry_indices: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));

    let skip_ids_set: HashSet<String> = opts
        .skip_ids
        .as_ref()
        .map(|ids| ids.iter().cloned().collect())
        .unwrap_or_default();

    let first_queue: VecDeque<usize> = (0..collection.tracks.len()).collect();
    let first_queue = Arc::new(Mutex::new(first_queue));

    run_pass(
        first_queue,
        &collection,
        &settings,
        user_data_dir,
        &tmp,
        opts.matches.as_ref(),
        opts.picks.as_ref(),
        &skip_ids_set,
        &ytdlp,
        app_handle,
        &prefetched_covers,
        &summary,
        gen,
        false,
        &retry_indices,
    )
    .await;

    if !CANCELLED_DOWNLOAD.load(Ordering::Relaxed) {
        let retries: VecDeque<usize> = {
            let mut r = retry_indices.lock().await;
            std::mem::take(&mut *r).into_iter().collect()
        };
        if !retries.is_empty() {
            let retry_queue = Arc::new(Mutex::new(retries));
            run_pass(
                retry_queue,
                &collection,
                &settings,
                user_data_dir,
                &tmp,
                opts.matches.as_ref(),
                opts.picks.as_ref(),
                &skip_ids_set,
                &ytdlp,
                app_handle,
                &prefetched_covers,
                &summary,
                gen,
                true,
                &retry_indices,
            )
            .await;
        }
    }

    let _ = covers_task.await;

    cleanup_tmp_dir(&tmp);

    if CANCELLED_DOWNLOAD.load(Ordering::SeqCst) {
        let mut s = summary.lock().await;
        s.cancelled = true;
    }

    emit_final_summary(app_handle, &summary).await;

    let _ = app_handle.emit("vynl:dl", DownloadEvent::Finished);
    Ok(())
}

fn normalize(s: &str) -> String {
    let replaced = s.to_lowercase().replace('&', " and ");
    let collapsed = NON_ALNUM_RE.replace_all(&replaced, " ");
    WHITESPACE_RE
        .replace_all(&collapsed, " ")
        .trim()
        .to_string()
}

fn deduplicate_tracks(
    tracks: &[TrackMeta],
    app_handle: &tauri::AppHandle,
) -> (Vec<TrackMeta>, usize) {
    let mut seen_keys: HashSet<String> = HashSet::new();
    let mut deduped: Vec<TrackMeta> = Vec::new();
    let mut skipped = 0;
    for track in tracks {
        let key = format!(
            "{}||{}||{}",
            normalize(&track.title),
            normalize(&track.artist),
            track.track_number.map_or("".to_string(), |n| n.to_string())
        );
        if seen_keys.insert(key) {
            deduped.push(track.clone());
        } else {
            emit_track(
                app_handle,
                track,
                TrackStatus::Skipped,
                0.0,
                Some("duplicate"),
            );
            skipped += 1;
        }
    }
    (deduped, skipped)
}

fn cleanup_tmp_dir(tmp: &Path) {
    if !tmp.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(tmp) {
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                let _ = std::fs::remove_file(&path);
            } else if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let _ = std::fs::remove_dir_all(&path);
            }
        }
    }
}

async fn emit_final_summary(app_handle: &tauri::AppHandle, summary: &Arc<Mutex<DownloadSummary>>) {
    let s = summary.lock().await;
    let _ = app_handle.emit("vynl:dl", DownloadEvent::Summary { payload: s.clone() });

    if !s.cancelled && !CANCELLED_DOWNLOAD.load(Ordering::SeqCst) && (s.done > 0 || s.failed > 0) {
        let _ = app_handle.emit("vynl:toast", &*s);
    }
}

async fn verify_track_duration(
    ffprobe: &str,
    raw_path: &Path,
    track: &TrackMeta,
) -> Option<String> {
    let expected = track.duration?;
    if expected <= 0.0 {
        return None;
    }
    let real = probe_duration(ffprobe, &raw_path.to_string_lossy()).await?;
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

fn record_failure(
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

async fn process_lyrics_lookup(
    track: &TrackMeta,
    _format: &AudioFormat,
    user_data_dir: &Path,
) -> (Option<String>, Option<String>) {
    let lookup = LyricsLookup {
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        duration: track.duration.unwrap_or(0.0),
    };
    let fetch_result = lyrics::fetch_lyrics_with_cache(&lookup, user_data_dir).await;
    match fetch_result {
        Some(lr)
            if lr.kind == crate::commands::types::LyricsKind::Lrc && !lr.text.trim().is_empty() =>
        {
            let lrc_text = lr.text.clone();
            let plain = LRC_TIME_RE.replace_all(&lr.text, "").trim().to_string();
            let embed = if plain.is_empty() { None } else { Some(plain) };
            (embed, Some(lrc_text))
        }
        Some(lr) if !lr.text.trim().is_empty() => (Some(lr.text), None),
        _ => (None, None),
    }
}

async fn move_tagged_to_output(
    tagged_file: &Path,
    final_path: &Path,
    track: &TrackMeta,
    lyrics_lrc_text: Option<&str>,
    user_data_dir: &Path,
    collection: &Collection,
    used_url: Option<&str>,
) -> Result<(), String> {
    tokio::fs::create_dir_all(final_path.parent().unwrap_or(Path::new(".")))
        .await
        .map_err(|e| e.to_string())?;

    if final_path.exists() {
        let _ = std::fs::remove_file(final_path);
    }

    move_file(tagged_file, final_path)?;

    if let Some(lrc) = lyrics_lrc_text {
        if !lrc.trim().is_empty() {
            let sidecar = lyrics::sidecar_path(&final_path.to_string_lossy(), "lrc");
            let _ = std::fs::write(&sidecar, lrc);
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

fn move_file(from: &Path, to: &Path) -> Result<(), String> {
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            let tmp = to.with_extension(format!(
                "{}.tmp",
                to.extension().and_then(|e| e.to_str()).unwrap_or("")
            ));
            std::fs::copy(from, &tmp).map_err(|e| e.to_string())?;
            match std::fs::rename(&tmp, to) {
                Ok(()) => {
                    let _ = std::fs::remove_file(from);
                    Ok(())
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&tmp);
                    Err(e.to_string())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::clean_title;

    const MODULE_SOURCES: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("paths.rs", include_str!("paths.rs")),
        ("matching.rs", include_str!("matching.rs")),
    ];

    #[test]
    fn clean_title_strips_feat_suffix_after_any_dash_separator() {
        let en = "\u{2013}";
        let em = "\u{2014}";

        assert_eq!(clean_title("Ghost Town - feat. Someone"), "Ghost Town");
        assert_eq!(
            clean_title(&format!("Ghost Town {} feat. Someone", en)),
            "Ghost Town"
        );
        assert_eq!(
            clean_title(&format!("Ghost Town {} feat. Someone", em)),
            "Ghost Town"
        );
        assert_eq!(clean_title("Ghost Town (feat. Someone)"), "Ghost Town");
        assert_eq!(clean_title("Plain Title"), "Plain Title");
    }

    #[test]
    fn source_file_has_no_double_encoded_text() {
        for (name, src) in MODULE_SOURCES {
            let chars: Vec<char> = src.chars().collect();

            for (i, w) in chars.windows(2).enumerate() {
                let (a, b) = (w[0] as u32, w[1] as u32);
                let latin1_lead = (0xC0..=0xFF).contains(&a);
                let c1_control = (0x80..=0x9F).contains(&b);

                assert!(
                    !(latin1_lead && b >= 0x80) && !c1_control,
                    "double-encoded UTF-8 in {name} at char offset {}: {:?}{:?}",
                    i,
                    a,
                    b
                );
            }
        }
    }
}
