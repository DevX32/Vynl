use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

const DISK_CACHE_FILE: &str = "artist-cache.json";
const MEM_CACHE_MAX: usize = 200;
const DISK_CACHE_MAX: usize = 500;

const MAX_ARTIST_NAME_LENGTH: usize = 200;
const VALID_ARTIST_NAME_RE: &str = r"^[a-zA-Z0-9\s\-_.,&'()]+$";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArtistInfo {
    pub name: String,
    pub disambiguation: Option<String>,
    pub country: Option<String>,
    pub begin_area: Option<String>,
    pub life_span: Option<ArtistLifeSpan>,
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub bio: Option<String>,
    pub artist_type: Option<String>,
    pub score: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArtistLifeSpan {
    pub ended: bool,
    pub begin: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MbArtist {
    name: String,
    #[serde(default)]
    disambiguation: Option<String>,
    #[serde(default)]
    country: Option<String>,
    #[serde(default)]
    begin_area: Option<MbArea>,
    #[serde(default)]
    life_span: Option<MbLifeSpan>,
    #[serde(default, rename = "type")]
    artist_type: Option<String>,
    #[serde(default, rename = "tags")]
    mb_tags: Option<Vec<MbTag>>,
    #[serde(default, rename = "genres")]
    mb_genres: Option<Vec<MbTag>>,
    #[serde(default)]
    score: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct MbArea {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MbLifeSpan {
    ended: Option<bool>,
    begin: Option<String>,
    end: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MbTag {
    name: String,
    count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct MbSearchResponse {
    artists: Option<Vec<MbArtist>>,
}

#[derive(Debug, Deserialize)]
struct WikipediaExtract {
    extract: Option<String>,
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("Vynl/0.1 (https://github.com/DevX32/Vynl)")
            .timeout(std::time::Duration::from_secs(8))
            .pool_max_idle_per_host(4)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

fn cache_key(name: &str) -> String {
    name.trim().to_lowercase()
}

fn mem_cache() -> &'static Mutex<HashMap<String, ArtistInfo>> {
    static CACHE: OnceLock<Mutex<HashMap<String, ArtistInfo>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn disk_cache_path(user_data_dir: &Path) -> PathBuf {
    user_data_dir.join(DISK_CACHE_FILE)
}

fn load_disk_cache(user_data_dir: &Path) -> HashMap<String, ArtistInfo> {
    let path = disk_cache_path(user_data_dir);
    let Ok(data) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

fn save_disk_cache(user_data_dir: &Path, cache: &HashMap<String, ArtistInfo>) {
    let path = disk_cache_path(user_data_dir);
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = fs::write(path, json);
    }
}

fn get_cached(key: &str, user_data_dir: &Path) -> Option<ArtistInfo> {
    if let Ok(guard) = mem_cache().lock() {
        if let Some(hit) = guard.get(key) {
            return Some(hit.clone());
        }
    }
    let disk = load_disk_cache(user_data_dir);
    if let Some(hit) = disk.get(key) {
        if let Ok(mut guard) = mem_cache().lock() {
            guard.insert(key.to_string(), hit.clone());
        }
        return Some(hit.clone());
    }
    None
}

fn put_cached(key: &str, info: &ArtistInfo, user_data_dir: &Path) {
    if let Ok(mut guard) = mem_cache().lock() {
        if guard.len() >= MEM_CACHE_MAX {
            if let Some(k) = guard.keys().next().cloned() {
                guard.remove(&k);
            }
        }
        guard.insert(key.to_string(), info.clone());
    }

    let mut disk = load_disk_cache(user_data_dir);
    if disk.len() >= DISK_CACHE_MAX && !disk.contains_key(key) {
        if let Some(k) = disk.keys().next().cloned() {
            disk.remove(&k);
        }
    }
    disk.insert(key.to_string(), info.clone());
    save_disk_cache(user_data_dir, &disk);
}

fn ranked_names(tags: Option<Vec<MbTag>>, limit: usize) -> Vec<String> {
    let mut items = tags.unwrap_or_default();
    items.sort_by_key(|item| std::cmp::Reverse(item.count.unwrap_or(0)));
    items
        .into_iter()
        .filter(|t| t.count.unwrap_or(0) > 0)
        .take(limit)
        .map(|t| t.name)
        .collect()
}

fn artist_from_mb(mb: MbArtist, bio: Option<String>) -> ArtistInfo {
    let genres = ranked_names(mb.mb_genres, 10);
    let mut tags = ranked_names(mb.mb_tags, 10);
    tags.retain(|t| !genres.contains(t));

    ArtistInfo {
        name: mb.name,
        disambiguation: mb.disambiguation,
        country: mb.country,
        begin_area: mb.begin_area.map(|a| a.name),
        life_span: mb.life_span.map(|ls| ArtistLifeSpan {
            ended: ls.ended.unwrap_or(false),
            begin: ls.begin,
            end: ls.end,
        }),
        genres,
        tags,
        bio,
        artist_type: mb.artist_type,
        score: mb.score,
    }
}

async fn fetch_musicbrainz(
    client: &reqwest::Client,
    artist_name: &str,
) -> Result<MbArtist, String> {
    let search_url = format!(
        "https://musicbrainz.org/ws/2/artist/?query={}&fmt=json&limit=1",
        urlencoding::encode(artist_name)
    );

    let resp = client
        .get(&search_url)
        .send()
        .await
        .map_err(|e| format!("MusicBrainz request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("MusicBrainz returned status {status}"));
    }

    let search: MbSearchResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse MusicBrainz response: {e}"))?;

    search
        .artists
        .and_then(|a| a.into_iter().next())
        .ok_or_else(|| format!("No artist found for \"{artist_name}\""))
}

async fn wikipedia_summary(client: &reqwest::Client, title: &str) -> Option<String> {
    let url = format!(
        "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
        urlencoding::encode(title)
    );
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let summary = resp.json::<WikipediaExtract>().await.ok()?;
    let extract = summary.extract.filter(|s| !s.trim().is_empty())?;
    Some(extract)
}

async fn fetch_wikipedia_bio(client: &reqwest::Client, artist_name: &str) -> Option<String> {
    let musician = format!("{artist_name} (musician)");
    let band = format!("{artist_name} (band)");

    let (direct, as_musician, as_band) = tokio::join!(
        wikipedia_summary(client, artist_name),
        wikipedia_summary(client, &musician),
        wikipedia_summary(client, &band),
    );

    if let Some(bio) = direct.or(as_musician).or(as_band) {
        return Some(bio);
    }

    let api_url = format!(
        "https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&format=json&srlimit=1",
        urlencoding::encode(&format!("{artist_name} musician"))
    );

    let resp = client.get(&api_url).send().await.ok()?;
    let json = resp.json::<serde_json::Value>().await.ok()?;
    let title = json["query"]["search"][0]["title"].as_str()?;
    wikipedia_summary(client, title).await
}

async fn fetch_artist_info_impl(
    artist_name: String,
    user_data_dir: &Path,
) -> Result<ArtistInfo, String> {
    if artist_name.len() > MAX_ARTIST_NAME_LENGTH {
        return Err(format!(
            "Artist name too long (max {} characters)",
            MAX_ARTIST_NAME_LENGTH
        ));
    }

    if artist_name.trim().is_empty() {
        return Err("Artist name cannot be empty".to_string());
    }

    let re = regex::Regex::new(VALID_ARTIST_NAME_RE).map_err(|e| format!("Invalid regex: {e}"))?;
    if !re.is_match(&artist_name) {
        return Err("Artist name contains invalid characters".to_string());
    }

    let key = cache_key(&artist_name);
    if let Some(cached) = get_cached(&key, user_data_dir) {
        return Ok(cached);
    }

    let client = http_client();

    let (mb_result, wiki_bio) = tokio::join!(
        fetch_musicbrainz(client, &artist_name),
        fetch_wikipedia_bio(client, &artist_name),
    );

    let mb = mb_result?;
    let mut bio = wiki_bio;

    if bio.is_none() {
        let canonical = mb.name.as_str();
        if !canonical.eq_ignore_ascii_case(artist_name.trim()) {
            bio = fetch_wikipedia_bio(client, canonical).await;
        }
    }

    let info = artist_from_mb(mb, bio);
    put_cached(&key, &info, user_data_dir);
    let canon_key = cache_key(&info.name);
    if canon_key != key {
        put_cached(&canon_key, &info, user_data_dir);
    }

    Ok(info)
}

#[tauri::command]
pub async fn fetch_artist_info(artist: String, app: AppHandle) -> Result<ArtistInfo, String> {
    let user_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    fetch_artist_info_impl(artist, &user_data).await
}
