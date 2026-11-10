use crate::commands::types::{LyricsKind, LyricsLookup, LyricsResult};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const FAIL_CACHE_MAX: usize = 500;
const DISK_CACHE_MAX: usize = 1000;
const REMOTE_CACHE_MAX: usize = 500;
const DISK_CACHE_FILE: &str = "lyrics-cache.json";
const NEGATIVE_CACHE_TTL_MS: u64 = 6 * 60 * 60 * 1000;
const LRC_TIMESTAMP_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[ \t]*\[[ \t]*\d{1,3}:\d{1,2}(?::\d{1,4})?(?:[.,]\d{1,4})?[ \t]*\]").unwrap()
});

static REMOTE_CACHE: OnceLock<tokio::sync::RwLock<HashMap<String, Option<LyricsResult>>>> =
    OnceLock::new();

fn remote_cache() -> &'static tokio::sync::RwLock<HashMap<String, Option<LyricsResult>>> {
    REMOTE_CACHE.get_or_init(|| tokio::sync::RwLock::new(HashMap::new()))
}

static FAIL_CACHE: OnceLock<tokio::sync::RwLock<HashMap<String, u64>>> = OnceLock::new();

fn fail_cache() -> &'static tokio::sync::RwLock<HashMap<String, u64>> {
    FAIL_CACHE.get_or_init(|| tokio::sync::RwLock::new(HashMap::new()))
}

static DISK_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

fn disk_mutex() -> &'static Mutex<()> {
    DISK_MUTEX.get_or_init(|| Mutex::new(()))
}

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("Vynl/1.0 (https://github.com/DevX32/Vynl)")
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

pub fn sidecar_path(file: &str, ext: &str) -> PathBuf {
    let p = Path::new(file);
    let parent = p.parent().unwrap_or(Path::new("."));
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
    parent.join("lyrics").join(format!("{}.{}", stem, ext))
}

pub fn legacy_sidecar_path(file: &str, ext: &str) -> PathBuf {
    let p = Path::new(file);
    let parent = p.parent().unwrap_or(Path::new("."));
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
    parent.join(format!("{}.{}", stem, ext))
}

pub fn is_ttml(text: &str) -> bool {
    let bytes = text.as_bytes();
    for i in 0..bytes.len().saturating_sub(3) {
        if bytes[i] != b'<' {
            continue;
        }
        let (b1, b2, b3) = (bytes[i + 1], bytes[i + 2], bytes[i + 3]);
        if !b1.eq_ignore_ascii_case(&b't') || !b2.eq_ignore_ascii_case(&b't') {
            continue;
        }
        if matches!(b3, b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/') {
            return true;
        }
        if b3.eq_ignore_ascii_case(&b'm') || b3.eq_ignore_ascii_case(&b'l') {
            return true;
        }
    }
    false
}

pub fn is_lrc(text: &str) -> bool {
    text.lines()
        .any(|line| LRC_TIMESTAMP_RE.is_match(line.trim_start()))
}

pub fn lyrics_kind(text: &str) -> LyricsKind {
    if is_ttml(text) {
        LyricsKind::Ttml
    } else if is_lrc(text) {
        LyricsKind::Lrc
    } else {
        LyricsKind::Txt
    }
}

pub fn local_lyrics(file: &str) -> Option<LyricsResult> {
    for kind in &["ttml", "lrc", "txt"] {
        for p in [sidecar_path(file, kind), legacy_sidecar_path(file, kind)] {
            match fs::read_to_string(&p) {
                Ok(text) if !text.trim().is_empty() => {
                    let lyrics_kind = match *kind {
                        "lrc" => crate::commands::types::LyricsKind::Lrc,
                        "ttml" => crate::commands::types::LyricsKind::Ttml,
                        _ => crate::commands::types::LyricsKind::Txt,
                    };
                    return Some(LyricsResult {
                        kind: lyrics_kind,
                        text,
                        source: crate::commands::types::LyricsSource::Local,
                        file: Some(p.to_string_lossy().to_string()),
                    });
                }
                _ => continue,
            }
        }
    }
    None
}

pub fn remove_stale_sidecars(file: &str, ext: &str) {
    for other in ["ttml", "lrc", "txt"] {
        if other == ext {
            continue;
        }
        let _ = fs::remove_file(sidecar_path(file, other));
        let _ = fs::remove_file(legacy_sidecar_path(file, other));
    }
    let _ = fs::remove_file(legacy_sidecar_path(file, ext));
}

pub fn save_manual_lyrics(file: &str, kind: LyricsKind, text: &str) -> Result<(), String> {
    let ext = match kind {
        LyricsKind::Lrc => "lrc",
        LyricsKind::Ttml => "ttml",
        LyricsKind::Txt => "txt",
    };

    let path = sidecar_path(file, ext);
    let Some(parent) = path.parent() else {
        return Err("Could not resolve lyrics directory".into());
    };
    fs::create_dir_all(parent).map_err(|e| format!("Could not create lyrics directory: {e}"))?;

    let temp = path.with_extension(format!("{ext}.{}.tmp", std::process::id()));
    fs::write(&temp, text).map_err(|e| format!("Could not save lyrics: {e}"))?;
    if let Err(err) = fs::rename(&temp, &path) {
        let _ = fs::remove_file(&temp);
        return Err(format!("Could not save lyrics: {err}"));
    }

    remove_stale_sidecars(file, ext);

    Ok(())
}

pub fn embed_lyrics(file: &str, text: &str) -> Result<(), String> {
    super::audio::embed_lyrics(Path::new(file), text)
}

fn cache_key(lookup: &LyricsLookup) -> String {
    format!("v4:{}", serde_json::to_string(lookup).unwrap_or_default())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DiskCacheEntry {
    key: String,
    result: Option<LyricsResult>,
    accessed_at: u64,
}

fn disk_cache_path(user_data_dir: &Path) -> PathBuf {
    user_data_dir.join(DISK_CACHE_FILE)
}

fn load_disk_cache(user_data_dir: &Path) -> (HashMap<String, DiskCacheEntry>, Option<SystemTime>) {
    let _lock = disk_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let path = disk_cache_path(user_data_dir);
    let mtime = fs::metadata(&path).and_then(|m| m.modified()).ok();
    let entries: Vec<DiskCacheEntry> = fs::read_to_string(&path)
        .ok()
        .and_then(|d| serde_json::from_str(&d).ok())
        .unwrap_or_default();
    let map = entries.into_iter().map(|e| (e.key.clone(), e)).collect();
    (map, mtime)
}

fn disk_cache_mtime(user_data_dir: &Path) -> Option<SystemTime> {
    let _lock = disk_mutex().lock().unwrap_or_else(|e| e.into_inner());
    fs::metadata(disk_cache_path(user_data_dir))
        .and_then(|m| m.modified())
        .ok()
}

fn save_disk_cache(user_data_dir: &Path, cache: &HashMap<String, DiskCacheEntry>) {
    let _lock = disk_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let mut entries: Vec<&DiskCacheEntry> = cache.values().collect();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.accessed_at));
    entries.truncate(DISK_CACHE_MAX);

    let path = disk_cache_path(user_data_dir);
    if let Ok(json) = serde_json::to_string(&entries) {
        let _ = fs::write(&path, json);
    }
}

pub async fn fetch_lyrics_with_cache(
    lookup: &LyricsLookup,
    user_data_dir: &Path,
) -> Option<LyricsResult> {
    let key = cache_key(lookup);

    {
        let cache = remote_cache().read().await;
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }
    }

    {
        let mut fails = fail_cache().write().await;
        if let Some(at) = fails.get(&key) {
            if now_millis().saturating_sub(*at) < NEGATIVE_CACHE_TTL_MS {
                return None;
            }
            fails.remove(&key);
        }
    }

    let now = now_millis();
    let (mut disk, disk_loaded_mtime) = load_disk_cache(user_data_dir);
    if let Some(entry) = disk.get(&key).cloned() {
        if let Some(hit) = entry.result {
            if let Some(current) = disk.get_mut(&key) {
                current.accessed_at = now;
            }
            save_disk_cache(user_data_dir, &disk);
            let mut mem_cache = remote_cache().write().await;
            mem_cache.insert(key, Some(hit.clone()));
            return Some(hit);
        }
        if now.saturating_sub(entry.accessed_at) < NEGATIVE_CACHE_TTL_MS {
            return None;
        }
        disk.remove(&key);
        save_disk_cache(user_data_dir, &disk);
    }

    let result = do_fetch(lookup).await;

    {
        let mut cache = remote_cache().write().await;
        if let Some(hit) = &result {
            cache.insert(key.clone(), Some(hit.clone()));
            if cache.len() > REMOTE_CACHE_MAX {
                let keys_to_remove: Vec<String> = cache
                    .keys()
                    .take(cache.len() - REMOTE_CACHE_MAX)
                    .cloned()
                    .collect();
                for k in keys_to_remove {
                    cache.remove(&k);
                }
            }
        } else {
            let mut fails = fail_cache().write().await;
            if fails.len() >= FAIL_CACHE_MAX {
                fails.clear();
            }
            fails.insert(key.clone(), now_millis());
        }
    }

    if disk_cache_mtime(user_data_dir) != disk_loaded_mtime {
        disk = load_disk_cache(user_data_dir).0;
    }
    disk.insert(
        key.clone(),
        DiskCacheEntry {
            key,
            result: result.clone(),
            accessed_at: now_millis(),
        },
    );
    save_disk_cache(user_data_dir, &disk);

    result
}

fn path_cache_key(path: &str) -> String {
    format!("path:{path}")
}

pub fn save_path_lyrics(user_data_dir: &Path, path: &str, text: &str, kind: LyricsKind) {
    let key = path_cache_key(path);
    let now = now_millis();
    let (mut disk, _) = load_disk_cache(user_data_dir);
    disk.insert(
        key.clone(),
        DiskCacheEntry {
            key,
            result: Some(LyricsResult {
                kind,
                text: text.to_string(),
                source: crate::commands::types::LyricsSource::Local,
                file: Some(path.to_string()),
            }),
            accessed_at: now,
        },
    );
    save_disk_cache(user_data_dir, &disk);
}

pub fn path_lyrics(user_data_dir: &Path, path: &str) -> Option<LyricsResult> {
    let key = path_cache_key(path);
    let (mut disk, _) = load_disk_cache(user_data_dir);
    let entry = disk.get_mut(&key)?;
    entry.accessed_at = now_millis();
    let result = entry.result.clone();
    save_disk_cache(user_data_dir, &disk);
    result
}

const LRCLIB_BASE: &str = "https://lrclib.net";
const LRCRED_BASE: &str = "https://lrc.red";

fn is_synced_kind(kind: &crate::commands::types::LyricsKind) -> bool {
    matches!(
        kind,
        crate::commands::types::LyricsKind::Lrc | crate::commands::types::LyricsKind::Ttml
    )
}

async fn do_fetch(lookup: &LyricsLookup) -> Option<LyricsResult> {
    let lrclib = do_fetch_lrclib(lookup).await;
    if lrclib.as_ref().is_some_and(|hit| is_synced_kind(&hit.kind)) {
        return lrclib;
    }
    if let Some(hit) = do_fetch_lrcred(lookup).await {
        return Some(hit);
    }
    let better = do_fetch_betterlyrics(lookup).await;
    if better.as_ref().is_some_and(|hit| is_synced_kind(&hit.kind)) {
        return better;
    }
    lrclib.or(better)
}

const BETTERLYRICS_BASE: &str = "https://lyrics-api.boidu.dev";

fn parse_betterlyrics(data: &serde_json::Value) -> Option<LyricsResult> {
    use crate::commands::types::LyricsKind;
    for (key, kind) in [
        ("ttml", LyricsKind::Ttml),
        ("syncedLyrics", LyricsKind::Lrc),
        ("lrc", LyricsKind::Lrc),
        ("lyrics", LyricsKind::Txt),
    ] {
        let Some(text) = data.get(key).and_then(|v| v.as_str()) else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        return Some(LyricsResult {
            kind,
            text: text.trim().to_string(),
            source: crate::commands::types::LyricsSource::Remote,
            file: None,
        });
    }
    None
}

async fn do_fetch_betterlyrics(lookup: &LyricsLookup) -> Option<LyricsResult> {
    if lookup.duration <= 0.0 {
        return None;
    }
    let client = http_client();
    for with_album in [true, false] {
        if !with_album && lookup.album.is_empty() {
            break;
        }
        let mut params = vec![
            ("s", lookup.title.clone()),
            ("a", lookup.artist.clone()),
            ("d", format!("{}", lookup.duration.round() as i64)),
        ];
        if with_album && !lookup.album.is_empty() {
            params.push(("al", lookup.album.clone()));
        }
        let query = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let url = format!("{BETTERLYRICS_BASE}/getLyrics?{query}");
        let resp = match client.get(url).send().await {
            Ok(resp) if resp.status().as_u16() == 200 => resp,
            _ => continue,
        };
        let Ok(json) = resp.json::<serde_json::Value>().await else {
            continue;
        };
        if let Some(result) = parse_betterlyrics(&json) {
            return Some(result);
        }
    }
    None
}

const DURATION_EXACT_SECS: f64 = 2.0;
const DURATION_TOLERANCE_SECS: f64 = 5.0;
const DURATION_HARD_REJECT_SECS: f64 = 8.0;

struct Hit {
    title: String,
    artist: String,
    duration: Option<f64>,
}

fn lrclib_hit(data: &serde_json::Value) -> Hit {
    Hit {
        title: data["trackName"].as_str().unwrap_or("").to_string(),
        artist: data["artistName"].as_str().unwrap_or("").to_string(),
        duration: item_duration(data),
    }
}

fn lrcred_hit(data: &serde_json::Value) -> Hit {
    Hit {
        title: data["title"].as_str().unwrap_or("").to_string(),
        artist: data["artist"].as_str().unwrap_or("").to_string(),
        duration: item_duration(data),
    }
}

fn item_duration(data: &serde_json::Value) -> Option<f64> {
    data.get("duration")
        .and_then(|v| v.as_f64())
        .filter(|d| d.is_finite() && *d > 0.0)
}

fn duration_acceptable(data: &serde_json::Value, lookup: &LyricsLookup) -> bool {
    if lookup.duration <= 0.0 {
        return true;
    }
    match item_duration(data) {
        Some(candidate) => (candidate - lookup.duration).abs() <= DURATION_HARD_REJECT_SECS,
        None => true,
    }
}

fn score_hit(hit: &Hit, lookup: &LyricsLookup) -> i32 {
    let title_lower = hit.title.to_lowercase();
    let artist_lower = hit.artist.to_lowercase();
    let lookup_title = lookup.title.to_lowercase();
    let lookup_artist = lookup.artist.to_lowercase();

    let title_match = if title_lower == lookup_title {
        10
    } else if title_lower.contains(&lookup_title) || lookup_title.contains(&title_lower) {
        5
    } else {
        0
    };

    let artist_match = if artist_lower == lookup_artist {
        10
    } else if artist_lower.contains(&lookup_artist) || lookup_artist.contains(&artist_lower) {
        5
    } else {
        0
    };

    let duration_match = match (hit.duration, lookup.duration > 0.0) {
        (Some(candidate), true) => {
            let diff = (candidate - lookup.duration).abs();
            if diff <= DURATION_EXACT_SECS {
                10
            } else if diff <= DURATION_TOLERANCE_SECS {
                5
            } else {
                -20
            }
        }
        _ => 0,
    };

    title_match + artist_match + duration_match
}

async fn do_fetch_lrclib(lookup: &LyricsLookup) -> Option<LyricsResult> {
    let mut params = vec![
        ("track_name", lookup.title.clone()),
        ("artist_name", lookup.artist.clone()),
        ("album_name", lookup.album.clone()),
    ];
    if lookup.duration > 0.0 {
        params.push(("duration", format!("{}", lookup.duration.round() as i64)));
    }

    let query: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let url = format!("https://lrclib.net/api/get?{}", query);
    let client = http_client();

    let mut fallback: Option<LyricsResult> = None;
    match client.get(&url).send().await {
        Ok(resp) if resp.status().as_u16() == 200 => {
            if let Ok(data) = resp.json::<serde_json::Value>().await
                && duration_acceptable(&data, lookup)
                && let Some(r) = parse_lrclib_response(Some(&data))
            {
                if is_synced_kind(&r.kind) {
                    return Some(r);
                }
                fallback = Some(r);
            }
        }
        _ => {}
    }

    let mut search_params: Vec<String> = Vec::new();
    if !lookup.title.is_empty() {
        search_params.push(format!("track_name={}", urlencoding::encode(&lookup.title)));
    }
    if !lookup.artist.is_empty() {
        search_params.push(format!(
            "artist_name={}",
            urlencoding::encode(&lookup.artist)
        ));
    }
    if search_params.is_empty() {
        return None;
    }

    let search_url = format!("https://lrclib.net/api/search?{}", search_params.join("&"));
    match client.get(&search_url).send().await {
        Ok(resp) if resp.status().as_u16() == 200 => {
            let items: Vec<serde_json::Value> = resp.json().await.unwrap_or_default();
            let mut best_synced: Option<(i32, LyricsResult)> = None;
            let mut best_plain: Option<(i32, LyricsResult)> = None;
            for item in &items {
                if !duration_acceptable(item, lookup) {
                    continue;
                }
                let Some(result) = parse_lrclib_response(Some(item)) else {
                    continue;
                };
                let score = score_hit(&lrclib_hit(item), lookup);
                let slot = if is_synced_kind(&result.kind) {
                    &mut best_synced
                } else {
                    &mut best_plain
                };
                if slot.as_ref().is_none_or(|(best, _)| score > *best) {
                    *slot = Some((score, result));
                }
            }
            if let Some((_, hit)) = best_synced {
                return Some(hit);
            }
            if let Some((_, hit)) = best_plain {
                fallback.get_or_insert(hit);
            }
            fallback
        }
        _ => fallback,
    }
}

fn search_result_key(result: &crate::commands::types::LrcSearchResult) -> (String, String) {
    (
        result.track_name.to_lowercase(),
        result.artist_name.to_lowercase(),
    )
}

fn push_search_result(
    results: &mut Vec<crate::commands::types::LrcSearchResult>,
    seen: &mut HashMap<(String, String), bool>,
    result: crate::commands::types::LrcSearchResult,
) {
    let key = search_result_key(&result);
    let synced = result.synced_lyrics.is_some();
    match seen.get(&key) {
        Some(true) => return,
        Some(false) if !synced => return,
        _ => {}
    }
    seen.insert(key, synced);
    results.push(result);
}

async fn do_fetch_lrcred(lookup: &LyricsLookup) -> Option<LyricsResult> {
    let query = format!("{} {}", lookup.artist, lookup.title);
    let search_url = format!(
        "{LRCRED_BASE}/search.json?q={}",
        urlencoding::encode(&query)
    );

    let hits: serde_json::Value = match http_client().get(&search_url).send().await {
        Ok(resp) if resp.status().as_u16() == 200 => resp.json().await.ok()?,
        _ => return None,
    };

    let items = hits.get("hits")?.as_array()?;
    let mut best: Option<(i32, String)> = None;
    for item in items {
        if !duration_acceptable(item, lookup) {
            continue;
        }
        let Some(isrc) = item["isrc"].as_str() else {
            continue;
        };
        let score = score_hit(&lrcred_hit(item), lookup);
        if best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, isrc.to_string()));
        }
    }

    let (_, isrc) = best?;
    fetch_lrcred_isrc(&isrc).await
}

async fn fetch_lrcred_isrc(isrc: &str) -> Option<LyricsResult> {
    for ext in ["ttml", "lrc"] {
        let url = format!("{LRCRED_BASE}/s/{}.{ext}", urlencoding::encode(isrc));
        let resp = match http_client().get(url).send().await {
            Ok(resp) if resp.status().as_u16() == 200 => resp,
            _ => continue,
        };
        let Ok(text) = resp.text().await else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        let kind = if ext == "ttml" {
            crate::commands::types::LyricsKind::Ttml
        } else {
            crate::commands::types::LyricsKind::Lrc
        };
        return Some(LyricsResult {
            kind,
            text: text.trim().to_string(),
            source: crate::commands::types::LyricsSource::Remote,
            file: None,
        });
    }
    None
}

fn parse_lrclib_response(data: Option<&serde_json::Value>) -> Option<LyricsResult> {
    let data = data?;
    let synced = data["syncedLyrics"]
        .as_str()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let plain = data["plainLyrics"]
        .as_str()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    if let Some(t) = synced {
        return Some(LyricsResult {
            kind: crate::commands::types::LyricsKind::Lrc,
            text: t,
            source: crate::commands::types::LyricsSource::Remote,
            file: None,
        });
    }
    plain.map(|t| LyricsResult {
        kind: crate::commands::types::LyricsKind::Txt,
        text: t,
        source: crate::commands::types::LyricsSource::Remote,
        file: None,
    })
}

pub async fn search_lyrics(
    query: &str,
    track_name: Option<&str>,
    artist_name: Option<&str>,
) -> Vec<crate::commands::types::LrcSearchResult> {
    let mut params: Vec<String> = Vec::new();
    if let Some(t) = track_name {
        params.push(format!("track_name={}", urlencoding::encode(t)));
    }
    if let Some(a) = artist_name {
        params.push(format!("artist_name={}", urlencoding::encode(a)));
    }
    if !query.is_empty() {
        params.push(format!("q={}", urlencoding::encode(query)));
    }
    if params.is_empty() {
        return vec![];
    }

    let url = format!("{LRCLIB_BASE}/api/search?{}", params.join("&"));
    let client = http_client();

    let mut results = match client.get(&url).send().await {
        Ok(resp) if resp.status().as_u16() == 200 => {
            match resp
                .json::<Vec<crate::commands::types::LrcSearchResult>>()
                .await
            {
                Ok(results) => results,
                Err(err) => {
                    eprintln!("LRCLIB search response could not be parsed: {err}");
                    Vec::new()
                }
            }
        }
        _ => vec![],
    };

    let mut seen: HashMap<(String, String), bool> = HashMap::new();
    for r in results.iter() {
        let entry = seen.entry(search_result_key(r)).or_insert(false);
        *entry |= r.synced_lyrics.is_some();
    }

    for r in search_lrcred(query, track_name, artist_name).await {
        push_search_result(&mut results, &mut seen, r);
    }
    results
}

const LRCRED_SEARCH_LIMIT: usize = 10;

async fn search_lrcred(
    query: &str,
    track_name: Option<&str>,
    artist_name: Option<&str>,
) -> Vec<crate::commands::types::LrcSearchResult> {
    let mut q = String::new();
    if let Some(t) = track_name.filter(|s| !s.is_empty()) {
        q.push_str(t);
    }
    if let Some(a) = artist_name.filter(|s| !s.is_empty()) {
        if !q.is_empty() {
            q.push(' ');
        }
        q.push_str(a);
    }
    if q.trim().is_empty() {
        if query.is_empty() {
            return vec![];
        }
        q = query.to_string();
    }

    let url = format!(
        "{LRCRED_BASE}/search.json?q={}",
        urlencoding::encode(q.trim())
    );
    let hits: serde_json::Value = match http_client().get(&url).send().await {
        Ok(resp) if resp.status().as_u16() == 200 => match resp.json().await {
            Ok(v) => v,
            Err(_) => return vec![],
        },
        _ => return vec![],
    };

    let Some(items) = hits.get("hits").and_then(|h| h.as_array()) else {
        return vec![];
    };

    let top: Vec<&serde_json::Value> = items.iter().take(LRCRED_SEARCH_LIMIT).collect::<Vec<_>>();

    let mut fetched = futures_util::future::join_all(top.iter().map(|item| async move {
        let isrc = item["isrc"].as_str()?;
        for ext in ["ttml", "lrc"] {
            let url = format!("{LRCRED_BASE}/s/{}.{ext}", urlencoding::encode(isrc));
            let resp = match http_client().get(&url).send().await {
                Ok(resp) if resp.status().as_u16() == 200 => resp,
                _ => continue,
            };
            let text = match resp.text().await {
                Ok(text) if !text.trim().is_empty() => text,
                _ => continue,
            };
            return Some((text, isrc.to_string(), ext == "ttml"));
        }
        None
    }))
    .await;

    let mut out = Vec::new();
    for (item, pair) in top.iter().zip(fetched.drain(..)) {
        let Some((text, isrc, is_ttml)) = pair else {
            continue;
        };
        let synced = is_ttml || is_lrc(&text);
        out.push(crate::commands::types::LrcSearchResult {
            id: isrc,
            track_name: item["title"].as_str().unwrap_or("").to_string(),
            artist_name: item["artist"].as_str().unwrap_or("").to_string(),
            album_name: item["album"].as_str().unwrap_or("").to_string(),
            duration: item_duration(item).unwrap_or(0.0),
            instrumental: false,
            plain_lyrics: if synced { None } else { Some(text.clone()) },
            synced_lyrics: if synced { Some(text) } else { None },
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{is_lrc, lyrics_kind, push_search_result, search_result_key};
    use crate::commands::types::{LrcSearchResult, LyricsKind};
    use std::collections::HashMap;

    fn search_result(track: &str, artist: &str, synced: bool) -> LrcSearchResult {
        LrcSearchResult {
            id: format!("{track}-{artist}"),
            track_name: track.to_string(),
            artist_name: artist.to_string(),
            album_name: String::new(),
            duration: 0.0,
            instrumental: false,
            plain_lyrics: if synced {
                None
            } else {
                Some("plain".to_string())
            },
            synced_lyrics: if synced {
                Some("[00:01.00]hi".to_string())
            } else {
                None
            },
        }
    }

    fn seed(results: &[LrcSearchResult]) -> HashMap<(String, String), bool> {
        let mut seen = HashMap::new();
        for result in results {
            seen.insert(search_result_key(result), result.synced_lyrics.is_some());
        }
        seen
    }

    #[test]
    fn detects_timestamped_lrc() {
        assert!(is_lrc("[00:12.34]hello"));
        assert!(is_lrc("[00:12:34]hello"));
        assert!(is_lrc("intro\n[1:02.500]line"));
    }

    #[test]
    fn rejects_metadata_and_inline_brackets() {
        assert!(!is_lrc("[ar:Artist]\n[ti:Title]\n[al:Album]"));
        assert!(!is_lrc("plain text [00:12]"));
        assert!(!is_lrc(""));
    }

    #[test]
    fn classifies_lyrics_text() {
        assert_eq!(
            lyrics_kind(r#"<tt xmlns="http://www.w3.org/ns/ttml"><body/></tt>"#),
            LyricsKind::Ttml
        );
        assert_eq!(lyrics_kind("[00:12.34]hello"), LyricsKind::Lrc);
        assert_eq!(lyrics_kind("plain words"), LyricsKind::Txt);
    }

    #[test]
    fn keeps_synced_search_hit_when_plain_duplicate_exists() {
        let mut results = vec![search_result("Song", "Artist", false)];
        let mut seen = seed(&results);

        push_search_result(
            &mut results,
            &mut seen,
            search_result("song", "artist", true),
        );

        assert_eq!(results.len(), 2);
        assert!(results[1].synced_lyrics.is_some());
    }

    #[test]
    fn drops_search_hit_when_synced_duplicate_exists() {
        let mut results = vec![search_result("Song", "Artist", true)];
        let mut seen = seed(&results);

        push_search_result(
            &mut results,
            &mut seen,
            search_result("song", "artist", true),
        );

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn drops_plain_search_hit_when_plain_duplicate_exists() {
        let mut results = vec![search_result("Song", "Artist", false)];
        let mut seen = seed(&results);

        push_search_result(
            &mut results,
            &mut seen,
            search_result("song", "artist", false),
        );

        assert_eq!(results.len(), 1);
    }
}
