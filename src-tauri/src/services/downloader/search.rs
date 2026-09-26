use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::{Mutex, Semaphore};

use crate::commands::types::*;

use super::process::run_yt_json;
use super::resolve_tool;
use super::{clean_title, filter_and_rank_candidates};

pub(super) static CANCELLED_FIND: AtomicBool = AtomicBool::new(false);

pub(super) fn build_query(track: &TrackMeta) -> String {
    let title = clean_title(&track.title);
    let mut parts = vec![title.clone(), track.artist.clone()];
    if !track.album.is_empty() && track.album != track.title && track.album != track.artist {
        let album = clean_title(&track.album);
        if !album.is_empty() && album != title {
            parts.push(album);
        }
    }
    parts.join(" ")
}

pub(super) fn build_query_simple(track: &TrackMeta, suffix: &str) -> String {
    let title = clean_title(&track.title);
    format!("{} {}{}", track.artist, title, suffix)
}

const YTM_SEARCH_COUNT: usize = 12;

const YT_SEARCH_COUNT: usize = 12;

const MIN_SEARCH_CANDIDATES: usize = 5;

const MATCH_CACHE_FILE: &str = "match-cache.json";

const SEARCH_PREFIX: &[(MatchSource, &str)] = &[
    (MatchSource::YouTubeMusic, "ytmsearch"),
    (MatchSource::YouTube, "ytsearch"),
];

const JS_RUNTIMES: &str = "deno,node,quickjs,bun";

pub(super) fn common_yt_args() -> Vec<String> {
    vec![
        "--no-warnings".into(),
        "--color".into(),
        "never".into(),
        "--socket-timeout".into(),
        "15".into(),
        "--js-runtimes".into(),
        JS_RUNTIMES.into(),
    ]
}

fn search_args(search_term: &str) -> Vec<String> {
    let mut args = vec![
        "--flat-playlist".into(),
        "--dump-single-json".into(),
        "--quiet".into(),
    ];
    args.extend(common_yt_args());
    args.push(search_term.into());
    args
}

fn search_prefix(source: &MatchSource) -> &str {
    SEARCH_PREFIX
        .iter()
        .find(|(s, _)| s == source)
        .map(|(_, p)| *p)
        .unwrap_or("ytsearch")
}

fn parse_search_entries(data: &Value, source: &MatchSource, out: &mut Vec<SearchCandidate>) {
    let Some(entries) = data.get("entries").and_then(|e| e.as_array()) else {
        return;
    };

    for entry in entries {
        let Some(url) = entry.get("url").and_then(|v| v.as_str()) else {
            continue;
        };
        let title = entry
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if title.is_empty() {
            continue;
        }

        out.push(SearchCandidate {
            url: url.to_string(),
            title,
            duration: entry
                .get("duration")
                .and_then(|v| v.as_f64())
                .map(|d| d.round()),
            channel: entry
                .get("channel")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            source: source.clone(),
            view_count: entry.get("view_count").and_then(|v| v.as_i64()),
            channel_verified: entry.get("channel_is_verified").and_then(|v| v.as_bool()),
            upload_date: entry
                .get("upload_date")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        });
    }
}

fn search_source(
    ytdlp: &str,
    source: &MatchSource,
    total: usize,
    queries: &[&str],
    deadline: &std::time::Instant,
) -> Vec<SearchCandidate> {
    let mut out: Vec<SearchCandidate> = Vec::new();
    let per_query = total.div_ceil(queries.len().max(1));

    for q in queries {
        if out.len() >= total {
            break;
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        let search_term = format!("{}{}:{}", search_prefix(source), per_query, q);
        let args = search_args(&search_term);

        if let Some(data) = run_yt_json(ytdlp, &args, remaining) {
            parse_search_entries(&data, source, &mut out);
        }
    }

    out
}

fn dedup_candidates(mut candidates: Vec<SearchCandidate>) -> Vec<SearchCandidate> {
    let mut seen: HashSet<String> = HashSet::new();
    candidates.retain(|c| seen.insert(c.url.clone()));
    candidates
}

pub(super) fn search_candidates(
    ytdlp: &str,
    query: &str,
    query_alt: &str,
    query_lyrics: &str,
) -> Vec<SearchCandidate> {
    let queries = [query, query_alt, query_lyrics];
    let deadline = std::time::Instant::now() + super::SEARCH_BUDGET;

    let music = dedup_candidates(search_source(
        ytdlp,
        &MatchSource::YouTubeMusic,
        YTM_SEARCH_COUNT,
        &queries,
        &deadline,
    ));
    if music.len() >= MIN_SEARCH_CANDIDATES {
        return music;
    }

    let mut all = music;
    all.extend(search_source(
        ytdlp,
        &MatchSource::YouTube,
        YT_SEARCH_COUNT,
        &queries,
        &deadline,
    ));
    dedup_candidates(all)
}

fn match_cache_path(user_data_dir: &Path) -> PathBuf {
    user_data_dir.join(MATCH_CACHE_FILE)
}

struct MatchCacheState {
    path: Option<PathBuf>,
    entries: HashMap<String, Vec<SearchCandidate>>,
    dirty: bool,
    last_saved: Option<std::time::Instant>,
}

static MATCH_CACHE: once_cell::sync::Lazy<std::sync::Mutex<MatchCacheState>> =
    once_cell::sync::Lazy::new(|| {
        std::sync::Mutex::new(MatchCacheState {
            path: None,
            entries: HashMap::new(),
            dirty: false,
            last_saved: None,
        })
    });

fn with_match_cache<R>(user_data_dir: &Path, f: impl FnOnce(&mut MatchCacheState) -> R) -> R {
    let mut guard = MATCH_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let path = match_cache_path(user_data_dir);
    if guard.path.as_ref() != Some(&path) {
        guard.entries = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        guard.path = Some(path);
        guard.dirty = false;
        guard.last_saved = None;
    }
    f(&mut guard)
}

const MATCH_CACHE_SAVE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

fn match_cache_save(state: &mut MatchCacheState, force: bool) {
    if !state.dirty {
        return;
    }
    let due = force
        || state
            .last_saved
            .map(|t| t.elapsed() >= MATCH_CACHE_SAVE_INTERVAL)
            .unwrap_or(true);
    if !due {
        return;
    }
    let Some(path) = state.path.clone() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec(&state.entries) {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, bytes).is_ok() && std::fs::rename(&tmp, &path).is_ok() {
            state.dirty = false;
            state.last_saved = Some(std::time::Instant::now());
        }
    }
}

fn match_cache_get(user_data_dir: &Path, id: &str) -> Option<Vec<SearchCandidate>> {
    with_match_cache(user_data_dir, |s| {
        s.entries.get(id).filter(|v| !v.is_empty()).cloned()
    })
}

fn match_cache_put(user_data_dir: &Path, id: &str, candidates: Vec<SearchCandidate>) -> bool {
    if candidates.is_empty() {
        return false;
    }
    with_match_cache(user_data_dir, |s| {
        if s.entries.contains_key(id) {
            return false;
        }
        s.entries.insert(id.to_string(), candidates);
        s.dirty = true;
        match_cache_save(s, false);
        true
    })
}

pub async fn find_matches(
    collection: &Collection,
    user_data_dir: &Path,
) -> Result<HashMap<String, Vec<SearchCandidate>>, String> {
    CANCELLED_FIND.store(false, Ordering::SeqCst);
    let ytdlp = resolve_tool("yt-dlp", user_data_dir)
        .ok_or_else(|| "yt-dlp is required to search for matches.".to_string())?;

    let result: Arc<Mutex<HashMap<String, Vec<SearchCandidate>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let index = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(4));
    let data_dir = user_data_dir.to_path_buf();

    let mut handles = Vec::new();
    for _ in 0..4 {
        let ytdlp = ytdlp.clone();
        let collection = collection.clone();
        let result = Arc::clone(&result);
        let index = Arc::clone(&index);
        let semaphore = Arc::clone(&semaphore);
        let data_dir = data_dir.clone();

        handles.push(tokio::spawn(async move {
            let _permit = match semaphore.acquire().await {
                Ok(p) => p,
                Err(_) => return,
            };
            loop {
                if CANCELLED_FIND.load(Ordering::Relaxed) {
                    break;
                }
                let i = index.fetch_add(1, Ordering::Relaxed);
                if i >= collection.tracks.len() {
                    break;
                }
                let track = &collection.tracks[i];
                let cached = match_cache_get(&data_dir, &track.id);
                let candidates = match cached {
                    Some(c) => c,
                    None => {
                        let query = build_query(track);
                        let query_alt = build_query_simple(track, "");
                        let query_lyrics = build_query_simple(track, " lyrics");
                        let ytdlp_clone = ytdlp.clone();
                        let found = tokio::task::spawn_blocking(move || {
                            search_candidates(&ytdlp_clone, &query, &query_alt, &query_lyrics)
                        })
                        .await
                        .unwrap_or_default();
                        match_cache_put(&data_dir, &track.id, found.clone());
                        found
                    }
                };

                let mut res = result.lock().await;
                res.insert(track.id.clone(), candidates);
            }
        }));
    }

    for handle in handles {
        handle.await.map_err(|e| e.to_string())?;
    }

    let final_result = match Arc::try_unwrap(result) {
        Ok(mutex) => mutex.into_inner(),
        Err(arc) => arc.lock().await.clone(),
    };

    with_match_cache(user_data_dir, |s| match_cache_save(s, true));

    Ok(final_result)
}

pub(super) async fn search_and_rank_candidates(
    ytdlp: &str,
    track: &TrackMeta,
    precomputed: Option<&HashMap<String, Vec<SearchCandidate>>>,
    picked: Option<usize>,
    user_data_dir: &Path,
) -> Vec<SearchCandidate> {
    if let Some(cands) = precomputed.and_then(|m| m.get(&track.id)) {
        if !cands.is_empty() {
            return match picked {
                Some(_) => cands.clone(),
                None => filter_and_rank_candidates(cands.clone(), track),
            };
        }
    }
    let raw = match match_cache_get(user_data_dir, &track.id) {
        Some(cached) => cached,
        None => {
            let ytdlp_c = ytdlp.to_string();
            let query = build_query(track);
            let query_alt = build_query_simple(track, "");
            let query_lyrics = build_query_simple(track, " lyrics");
            let found = tokio::task::spawn_blocking(move || {
                search_candidates(&ytdlp_c, &query, &query_alt, &query_lyrics)
            })
            .await
            .unwrap_or_default();
            match_cache_put(user_data_dir, &track.id, found.clone());
            found
        }
    };
    filter_and_rank_candidates(raw, track)
}
