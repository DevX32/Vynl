use std::sync::Arc;

use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use super::internal;
use crate::commands::types::*;

const EMBED_BASE: &str = "https://open.spotify.com/embed";
const TRACK_PAGE_BASE: &str = "https://open.spotify.com/track";

const RETRY_DELAY_MS: u64 = 1200;
const MAX_ATTEMPTS: u32 = 2;
const CONCURRENT_WORKERS: usize = 10;

static PATH_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"open\.spotify\.com/(track|album|playlist)/([A-Za-z0-9]+)").unwrap());
static URI_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^spotify:(track|album|playlist):([A-Za-z0-9]+)$").unwrap());
static NEXT_DATA_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<script id="__NEXT_DATA__" type="application/json">(.*?)</script>"#).unwrap()
});

#[derive(Debug, Clone)]
pub struct CatalogLink {
    pub kind: crate::commands::types::CollectionKind,
    pub id: String,
}

fn build_client() -> Result<Client, String> {
    internal::build_client(internal::UA, internal::TIMEOUT_SECS)
}

pub fn parse_link(raw: &str) -> Option<CatalogLink> {
    let url: String = raw
        .trim()
        .chars()
        .take_while(|&c| c != '?' && c != '#')
        .collect();

    if let Some(caps) = PATH_RE.captures(&url) {
        let kind_str = caps.get(1)?.as_str();
        let kind = match kind_str {
            "track" => crate::commands::types::CollectionKind::Track,
            "album" => crate::commands::types::CollectionKind::Album,
            "playlist" => crate::commands::types::CollectionKind::Playlist,
            _ => return None,
        };
        let id = caps.get(2)?.as_str().to_string();
        return Some(CatalogLink { kind, id });
    }

    if let Some(caps) = URI_RE.captures(raw.trim()) {
        let kind_str = caps.get(1)?.as_str();
        let kind = match kind_str {
            "track" => crate::commands::types::CollectionKind::Track,
            "album" => crate::commands::types::CollectionKind::Album,
            "playlist" => crate::commands::types::CollectionKind::Playlist,
            _ => return None,
        };
        let id = caps.get(2)?.as_str().to_string();
        return Some(CatalogLink { kind, id });
    }

    None
}

#[derive(Debug)]
enum EmbedResult {
    Entity(Value),
    NotFound,
    NoData,
}

async fn fetch_embed(client: &Client, url: &str) -> Result<EmbedResult, String> {
    fetch_embed_inner(client, url, 0).await
}

async fn fetch_embed_inner(client: &Client, url: &str, depth: u32) -> Result<EmbedResult, String> {
    if depth > 10 {
        return Err("Too many redirects".to_string());
    }

    let resp = client
        .get(url)
        .header("Accept", "text/html")
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();

    if status.is_redirection() {
        let location = resp
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| "Redirect with no Location header".to_string())?;

        let base: reqwest::Url = url.parse().map_err(|e| format!("URL parse error: {e}"))?;
        let redirect_url: reqwest::Url = base
            .join(location)
            .map_err(|e| format!("Invalid redirect URL: {e}"))?;

        return Box::pin(fetch_embed_inner(client, redirect_url.as_str(), depth + 1)).await;
    }

    if !status.is_success() {
        return Err(format!("Remote service responded with HTTP {status}"));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read body: {e}"))?;

    extract_embed_entity(&body)
}

async fn fetch_entity(client: &Client, url: &str) -> Result<EmbedResult, String> {
    let mut last_result: Option<EmbedResult> = None;
    let mut last_error: Option<String> = None;

    for _ in 0..MAX_ATTEMPTS {
        match fetch_embed(client, url).await {
            Ok(result) => match &result {
                EmbedResult::Entity(_) | EmbedResult::NotFound => return Ok(result),
                EmbedResult::NoData => {
                    last_result = Some(result);
                    sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                }
            },
            Err(e) => {
                last_error = Some(e);
                sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
            }
        }
    }

    if let Some(e) = last_error {
        return Err(e);
    }
    Ok(last_result.unwrap_or(EmbedResult::NoData))
}

fn image_sources(value: &Value) -> Option<Vec<&Value>> {
    match value {
        Value::Array(arr) if !arr.is_empty() => Some(arr.iter().collect()),
        Value::Object(obj) if obj.contains_key("url") => Some(vec![value]),
        _ => None,
    }
}

fn cover_image_sources(entity: &Value) -> Option<Vec<&Value>> {
    if let Some(sources) = entity
        .get("coverArt")
        .and_then(|c| c.get("sources"))
        .and_then(image_sources)
    {
        return Some(sources);
    }

    if let Some(sources) = entity
        .get("visualIdentity")
        .and_then(|vi| vi.get("image"))
        .and_then(image_sources)
    {
        return Some(sources);
    }

    entity.get("images").and_then(image_sources)
}

fn pick_cover(entity: &Value) -> Option<String> {
    let sources = cover_image_sources(entity)?;

    let mut sorted = sources;
    sorted.sort_by(|a, b| {
        let ha = a
            .get("height")
            .and_then(|v| v.as_i64())
            .or_else(|| a.get("maxHeight").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        let hb = b
            .get("height")
            .and_then(|v| v.as_i64())
            .or_else(|| b.get("maxHeight").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        hb.cmp(&ha)
    });

    sorted
        .first()
        .and_then(|s| s.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()))
}

fn year_of(entity: &Value) -> Option<i64> {
    if let Some(rd) = entity.get("releaseDate") {
        if let Some(iso) = rd.get("isoString").and_then(|v| v.as_str()) {
            if iso.len() >= 4 {
                if let Ok(y) = iso[..4].parse::<i64>() {
                    if y > 0 {
                        return Some(y);
                    }
                }
            }
        }
        if let Some(y) = rd.get("year").and_then(|v| v.as_i64()) {
            if y > 0 {
                return Some(y);
            }
        }
        if let Some(y) = rd.get("year").and_then(|v| v.as_str()) {
            if let Ok(y) = y.parse::<i64>() {
                if y > 0 {
                    return Some(y);
                }
            }
        }
    }

    if let Some(rd) = entity.get("releaseDate").and_then(|v| v.as_str()) {
        if rd.len() >= 4 {
            if let Ok(y) = rd[..4].parse::<i64>() {
                if y > 0 {
                    return Some(y);
                }
            }
        }
    }

    None
}

fn artists_of(entity: &Value) -> String {
    if let Some(Value::Array(artists)) = entity.get("artists") {
        let names: Vec<&str> = artists
            .iter()
            .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
            .collect();
        if !names.is_empty() {
            return names.join(", ");
        }
    }
    entity
        .get("subtitle")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn track_id(item: &Value, index: usize) -> String {
    item.get("uri")
        .and_then(|v| v.as_str())
        .and_then(|uri| uri.rsplit(':').next())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("t{}", index))
}

fn to_track(
    item: &Value,
    index: usize,
    fallback_album: &str,
    fallback_year: Option<i64>,
) -> TrackMeta {
    let duration_ms = item.get("duration").and_then(|v| v.as_f64());

    let id = track_id(item, index);
    let title = item
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("Unknown Track {}", index + 1));
    let artist = {
        let a = artists_of(item);
        if a.is_empty() {
            "Unknown Artist".to_string()
        } else {
            a
        }
    };
    let duration = duration_ms
        .filter(|d| *d > 0.0)
        .map(|d| (d / 1000.0).round());
    let url = item
        .get("uri")
        .and_then(|v| v.as_str())
        .map(|_| {
            if !id.is_empty() {
                format!("{TRACK_PAGE_BASE}/{id}")
            } else {
                String::new()
            }
        })
        .unwrap_or_default();
    let track_number = Some((index + 1) as i64);

    TrackMeta {
        id,
        title,
        artist,
        album: fallback_album.to_string(),
        year: fallback_year,
        duration,
        cover: None,
        track_number,
        url,
    }
}

async fn track_cover(
    client: &Client,
    cache: &Arc<Mutex<std::collections::HashMap<String, Option<String>>>>,
    id: &str,
) -> Option<String> {
    {
        let cache = cache.lock().await;
        if let Some(cached) = cache.get(id) {
            return cached.clone();
        }
    }

    let url = format!("{EMBED_BASE}/track/{id}");
    let cover = match fetch_entity(client, &url).await {
        Ok(EmbedResult::Entity(entity)) => pick_cover(&entity),
        _ => None,
    };

    let mut cache = cache.lock().await;
    cache.insert(id.to_string(), cover.clone());
    cover
}

async fn enrich_covers(client: &Client, tracks: &mut [TrackMeta]) {
    let missing_ids: Vec<String> = tracks
        .iter()
        .filter(|t| t.cover.is_none())
        .map(|t| t.id.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    if missing_ids.is_empty() {
        return;
    }

    let cache: Arc<Mutex<std::collections::HashMap<String, Option<String>>>> =
        Arc::new(Mutex::new(std::collections::HashMap::new()));
    let semaphore = Arc::new(tokio::sync::Semaphore::new(CONCURRENT_WORKERS));
    let mut handles = Vec::new();

    for id in &missing_ids {
        let client = client.clone();
        let cache = Arc::clone(&cache);
        let semaphore = Arc::clone(&semaphore);
        let id = id.clone();
        handles.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            let cover = track_cover(&client, &cache, &id).await;
            (id, cover)
        }));
    }

    let mut results: Vec<(String, Option<String>)> = Vec::new();
    for h in handles {
        if let Ok((id, cover)) = h.await {
            results.push((id, cover));
        }
    }

    for (id, cover) in results {
        if let Some(c) = cover {
            for t in tracks.iter_mut() {
                if t.id == id {
                    t.cover = Some(c.clone());
                }
            }
        }
    }
}

fn extract_embed_entity(body: &str) -> Result<EmbedResult, String> {
    let json_str = match NEXT_DATA_RE.captures(body) {
        Some(caps) => caps.get(1).unwrap().as_str(),
        None => return Ok(EmbedResult::NoData),
    };

    let parsed: Value =
        serde_json::from_str(json_str).map_err(|_| "Could not read the link data.".to_string())?;

    let page = parsed
        .get("props")
        .and_then(|p| p.get("pageProps"))
        .and_then(|pp| pp.as_object());

    if let Some(page) = page {
        if let Some(status_val) = page.get("status").and_then(|s| s.as_i64()) {
            if status_val == 500 || status_val == 404 {
                return Ok(EmbedResult::NotFound);
            }
        }
    }

    let entity = parsed
        .get("props")
        .and_then(|p| p.get("pageProps"))
        .and_then(|pp| pp.get("state"))
        .and_then(|s| s.get("data"))
        .and_then(|d| d.get("entity"))
        .cloned();

    match entity {
        Some(e) => Ok(EmbedResult::Entity(e)),
        None => Ok(EmbedResult::NotFound),
    }
}

fn parse_track_entity(
    entity: &Value,
    link: &CatalogLink,
    cover: &Option<String>,
    year: Option<i64>,
) -> Result<Collection, String> {
    let item = entity
        .get("trackList")
        .and_then(|tl| tl.as_array())
        .and_then(|arr| arr.first())
        .cloned()
        .unwrap_or_else(|| {
            let mut m = serde_json::Map::new();
            if let Some(uri) = entity.get("uri") {
                m.insert("uri".to_string(), uri.clone());
            }
            if let Some(t) = entity.get("title") {
                m.insert("title".to_string(), t.clone());
            }
            if let Some(d) = entity.get("duration") {
                m.insert("duration".to_string(), d.clone());
            }
            if let Some(id) = entity.get("id") {
                m.insert("id".to_string(), id.clone());
            }
            Value::Object(m)
        });

    let mut track = to_track(&item, 0, "", year);
    let entity_artists = artists_of(entity);
    if !entity_artists.is_empty() {
        track.artist = entity_artists;
    }
    track.cover = cover.clone();

    let track_id = entity
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| link.id.clone());

    Ok(Collection {
        kind: crate::commands::types::CollectionKind::Track,
        id: track_id,
        title: track.title.clone(),
        owner: None,
        cover: cover.clone(),
        tracks: vec![track],
    })
}

fn parse_collection_entity(
    entity: &Value,
    link: &CatalogLink,
    title: &str,
    cover: &Option<String>,
    year: Option<i64>,
) -> Result<Collection, String> {
    let track_list = entity
        .get("trackList")
        .and_then(|tl| tl.as_array())
        .filter(|arr| !arr.is_empty())
        .ok_or_else(|| "This link is empty, private, or unavailable.".to_string())?;

    let fallback_album = if link.kind == crate::commands::types::CollectionKind::Album {
        title.to_string()
    } else {
        String::new()
    };

    let album_artist = if link.kind == crate::commands::types::CollectionKind::Album {
        artists_of(entity)
    } else {
        String::new()
    };

    let tracks: Vec<TrackMeta> = track_list
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let mut track = to_track(item, i, &fallback_album, year);
            if (track.artist.is_empty() || track.artist == "Unknown Artist")
                && !album_artist.is_empty()
            {
                track.artist = album_artist.clone();
            }
            if link.kind == crate::commands::types::CollectionKind::Album {
                track.cover = cover.clone();
            }
            track
        })
        .collect();

    let collection_id = entity
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| link.id.clone());

    let owner = if link.kind == crate::commands::types::CollectionKind::Playlist {
        entity
            .get("subtitle")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        None
    };

    Ok(Collection {
        kind: link.kind.clone(),
        id: collection_id,
        title: title.to_string(),
        owner,
        cover: cover.clone(),
        tracks,
    })
}

fn collection_kind_str(kind: &CollectionKind) -> &'static str {
    match kind {
        crate::commands::types::CollectionKind::Track => "track",
        crate::commands::types::CollectionKind::Album => "album",
        crate::commands::types::CollectionKind::Playlist => "playlist",
    }
}

async fn try_internal_collection(
    link: &CatalogLink,
    result: Result<Collection, String>,
) -> Option<Result<Collection, String>> {
    match result {
        Ok(mut collection) => {
            let client = match build_client() {
                Ok(c) => c,
                Err(e) => return Some(Err(e)),
            };
            if collection.tracks.iter().any(|t| t.cover.is_none()) {
                enrich_covers(&client, &mut collection.tracks).await;
            }
            Some(Ok(collection))
        }
        Err(e) => {
            eprintln!(
                "[Vynl] Internal API failed for {}, falling back to embed: {e}",
                collection_kind_str(&link.kind)
            );
            None
        }
    }
}

pub async fn fetch_collection(link: &CatalogLink) -> Result<Collection, String> {
    let internal = match link.kind {
        crate::commands::types::CollectionKind::Playlist => {
            try_internal_collection(link, internal::fetch_all_playlist_tracks(&link.id).await).await
        }
        crate::commands::types::CollectionKind::Album => {
            try_internal_collection(link, internal::fetch_all_album_tracks(&link.id).await).await
        }
        _ => None,
    };
    if let Some(result) = internal {
        return result;
    }

    let client = build_client()?;
    let kind_str = collection_kind_str(&link.kind);
    let url = format!("{EMBED_BASE}/{}/{}/", kind_str, link.id);

    let result = fetch_entity(&client, &url).await?;

    let entity = match result {
        EmbedResult::Entity(e) => e,
        EmbedResult::NotFound => {
            return Err(format!(
                "This link is empty, private, or unavailable{}.",
                if link.id.len() != 22 {
                    " (the ID does not look valid)"
                } else {
                    ""
                }
            ));
        }
        EmbedResult::NoData => {
            return Err(format!(
                "No data returned for this {}. It may be blocked in your region — try a different link.",
                kind_str
            ));
        }
    };

    let title = entity
        .get("name")
        .and_then(|v| v.as_str())
        .or_else(|| entity.get("title").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            if link.kind == crate::commands::types::CollectionKind::Track {
                "Track".to_string()
            } else {
                kind_str.to_string()
            }
        });
    let cover = pick_cover(&entity);
    let year = year_of(&entity);

    if link.kind == crate::commands::types::CollectionKind::Track {
        return parse_track_entity(&entity, link, &cover, year);
    }

    let mut collection = parse_collection_entity(&entity, link, &title, &cover, year)?;

    if link.kind == crate::commands::types::CollectionKind::Playlist {
        enrich_covers(&client, &mut collection.tracks).await;
    }

    Ok(collection)
}
