use crate::commands::types::{LyricsLookup, LyricsResult};
use crate::services::process;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const FAIL_CACHE_MAX: usize = 500;
const DISK_CACHE_MAX: usize = 1000;
const REMOTE_CACHE_MAX: usize = 500;
const DISK_CACHE_FILE: &str = "lyrics-cache.json";

static REMOTE_CACHE: OnceLock<tokio::sync::RwLock<HashMap<String, Option<LyricsResult>>>> =
    OnceLock::new();

fn remote_cache() -> &'static tokio::sync::RwLock<HashMap<String, Option<LyricsResult>>> {
    REMOTE_CACHE.get_or_init(|| tokio::sync::RwLock::new(HashMap::new()))
}

static FAIL_CACHE: OnceLock<tokio::sync::RwLock<HashMap<String, ()>>> = OnceLock::new();

fn fail_cache() -> &'static tokio::sync::RwLock<HashMap<String, ()>> {
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
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

pub fn sidecar_path(file: &str, ext: &str) -> PathBuf {
    let p = Path::new(file);
    let parent = p.parent().unwrap_or(Path::new("."));
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
    parent.join(format!("{}.{}", stem, ext))
}

pub fn local_lyrics(file: &str) -> Option<LyricsResult> {
    for kind in &["lrc", "txt"] {
        let p = sidecar_path(file, kind);
        match fs::read_to_string(&p) {
            Ok(text) if !text.trim().is_empty() => {
                let lyrics_kind = match *kind {
                    "lrc" => crate::commands::types::LyricsKind::Lrc,
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
    None
}

fn try_embed_lyrics_lofty(file: &str, text: &str) -> Result<(), String> {
    use lofty::prelude::*;
    use lofty::probe::Probe;
    use lofty::tag::ItemKey;

    let ext = Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext == "wav" {
        return Err("wav cannot store lyrics".into());
    }

    let mut tagged_file = Probe::open(file)
        .map_err(|e| format!("probe failed: {e}"))?
        .read()
        .map_err(|e| format!("read failed: {e}"))?;
    let tag_type = match ext.as_str() {
        "mp3" => lofty::tag::TagType::Id3v2,
        "m4a" | "mp4" | "aac" => lofty::tag::TagType::Mp4Ilst,
        "flac" => lofty::tag::TagType::VorbisComments,
        "opus" | "ogg" => lofty::tag::TagType::VorbisComments,
        _ => tagged_file
            .primary_tag()
            .map(|t| t.tag_type())
            .unwrap_or(lofty::tag::TagType::Id3v2),
    };

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

    tag.insert_text(ItemKey::Lyrics, text.to_string());
    tag.insert_text(ItemKey::UnsyncLyrics, text.to_string());
    tagged_file
        .save_to_path(file, Default::default())
        .map_err(|e| format!("lofty save failed: {e}"))?;
    Ok(())
}

pub fn embed_lyrics(file: &str, text: &str, ffmpeg_path: Option<&str>) -> Result<(), String> {
    let ext = Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if ext.eq_ignore_ascii_case("wav") {
        return Err("wav cannot store lyrics tags.".into());
    }
    if try_embed_lyrics_lofty(file, text).is_ok() {
        return Ok(());
    }

    let ffmpeg = ffmpeg_path.ok_or("ffmpeg is required to embed lyrics.")?;

    let tmp = {
        let ext = Path::new(file)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let rand_suffix: String = {
            use std::collections::hash_map::RandomState;
            use std::hash::{BuildHasher, Hasher};
            let hash = RandomState::new().build_hasher();
            let mut h = hash;
            h.write_u64(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64,
            );
            format!("{:08x}", (h.finish() >> 16) as u32)
        };
        format!("{}.{}.lyr.tmp{}", file, rand_suffix, ext)
    };
    let metadata_val = text.replace('\\', "\\\\").replace('"', "\\\"");
    let mut command = process::hidden_std(Command::new(ffmpeg));
    command
        .args([
            "-hide_banner",
            "-y",
            "-v",
            "error",
            "-i",
            file,
            "-c",
            "copy",
        ])
        .args(["-metadata", &format!("lyrics={}", metadata_val)])
        .arg(&tmp)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let result = command.output();

    match result {
        Ok(output) if output.status.success() => {
            fs::rename(&tmp, file).map_err(|e| format!("Failed to rename temp file: {e}"))?;
            Ok(())
        }
        Ok(output) => {
            fs::remove_file(&tmp).ok();
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("ffmpeg failed: {}", stderr.trim()))
        }
        Err(e) => {
            fs::remove_file(&tmp).ok();
            Err(format!("Failed to run ffmpeg: {e}"))
        }
    }
}

fn cache_key(lookup: &LyricsLookup) -> String {
    serde_json::to_string(lookup).unwrap_or_default()
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

fn load_disk_cache(user_data_dir: &Path) -> HashMap<String, Option<LyricsResult>> {
    let _lock = disk_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let path = disk_cache_path(user_data_dir);
    let data = match fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return HashMap::new(),
    };
    let entries: Vec<DiskCacheEntry> = serde_json::from_str(&data).unwrap_or_default();
    entries.into_iter().map(|e| (e.key, e.result)).collect()
}

fn save_disk_cache(user_data_dir: &Path, cache: &HashMap<String, Option<LyricsResult>>) {
    let _lock = disk_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let mut entries: Vec<DiskCacheEntry> = cache
        .iter()
        .map(|(k, v)| DiskCacheEntry {
            key: k.clone(),
            result: v.clone(),
            accessed_at: now_millis(),
        })
        .collect();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.accessed_at));
    entries.truncate(DISK_CACHE_MAX);

    let path = disk_cache_path(user_data_dir);
    if let Ok(json) = serde_json::to_string_pretty(&entries) {
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
        let fails = fail_cache().read().await;
        if fails.contains_key(&key) {
            return None;
        }
    }

    {
        let disk = load_disk_cache(user_data_dir);
        if let Some(cached) = disk.get(&key) {
            let mut mem_cache = remote_cache().write().await;
            mem_cache.insert(key.clone(), cached.clone());
            return cached.clone();
        }
    }

    let result = do_fetch(lookup).await;

    {
        let mut cache = remote_cache().write().await;
        if result.is_some() {
            cache.insert(key.clone(), result.clone());
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
            fails.insert(key.clone(), ());
        }
    }

    {
        let mut disk = load_disk_cache(user_data_dir);
        disk.insert(key, result.clone());
        save_disk_cache(user_data_dir, &disk);
    }

    result
}

async fn do_fetch(lookup: &LyricsLookup) -> Option<LyricsResult> {
    let lrclib = do_fetch_lrclib(lookup).await;
    if is_synced(&lrclib) {
        return lrclib;
    }

    let ovh = do_fetch_lyrics_ovh(lookup).await;
    if is_synced(&ovh) {
        return ovh;
    }

    lrclib.or(ovh)
}

fn is_synced(result: &Option<LyricsResult>) -> bool {
    matches!(
        result,
        Some(r) if matches!(r.kind, crate::commands::types::LyricsKind::Lrc)
    )
}

fn score_lyrics_match(title: &str, artist: &str, lookup: &LyricsLookup) -> i32 {
    let title_lower = title.to_lowercase();
    let artist_lower = artist.to_lowercase();
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

    title_match + artist_match
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

    match client.get(&url).send().await {
        Ok(resp) if resp.status().as_u16() == 200 => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(r) = parse_lrclib_response(Some(&data)) {
                    return Some(r);
                }
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
            let mut best: Option<LyricsResult> = None;
            let mut best_score: i32 = -1;
            for item in &items {
                let title = item["trackName"].as_str().unwrap_or("");
                let artist = item["artistName"].as_str().unwrap_or("");
                let score = score_lyrics_match(title, artist, lookup);
                if score > best_score {
                    if let Some(r) = parse_lrclib_response(Some(item)) {
                        best_score = score;
                        best = Some(r);
                    }
                }
            }
            best
        }
        _ => None,
    }
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

async fn do_fetch_lyrics_ovh(lookup: &LyricsLookup) -> Option<LyricsResult> {
    let client = http_client();
    let artist = urlencoding::encode(&lookup.artist);
    let title = urlencoding::encode(&lookup.title);
    let url = format!("https://api.lyrics.ovh/v1/{}/{}", artist, title);

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
            Ok(data) => {
                if let Some(text) = data["lyrics"].as_str() {
                    let text = text.trim().to_string();
                    if !text.is_empty() {
                        let is_lrc = text.contains("[00:") || text.contains("[01:");
                        return Some(LyricsResult {
                            kind: if is_lrc {
                                crate::commands::types::LyricsKind::Lrc
                            } else {
                                crate::commands::types::LyricsKind::Txt
                            },
                            text,
                            source: crate::commands::types::LyricsSource::Remote,
                            file: None,
                        });
                    }
                }
                None
            }
            Err(_) => None,
        },
        _ => None,
    }
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

    let url = format!("https://lrclib.net/api/search?{}", params.join("&"));
    let client = http_client();

    match client.get(&url).send().await {
        Ok(resp) if resp.status().as_u16() == 200 => resp
            .json::<Vec<crate::commands::types::LrcSearchResult>>()
            .await
            .unwrap_or_default(),
        _ => vec![],
    }
}
