use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_json::{json, Value};
use sha1::Sha1;

use crate::commands::types::*;

type HmacSha1 = Hmac<Sha1>;

const SECRETS_URL: &str =
    "https://code.thetadev.de/ThetaDev/spotify-secrets/raw/branch/main/secrets/secretDict.json";

const FALLBACK_TOTP_VERSION: u32 = 18;
const FALLBACK_TOTP_SECRET: &[u8] = &[
    70, 60, 33, 57, 92, 120, 90, 33, 32, 62, 62, 55, 126, 93, 66, 35, 108, 68,
];

const FETCH_PLAYLIST_HASH: &str =
    "bb67e0af06e8d6f52b531f97468ee4acd44cd0f82b988e15c2ea47b1148efc77";
const GET_ALBUM_HASH: &str = "b9bfabef66ed756e5e13f68a942deb60bd4125ec1f1be8cc42769dc0259b4b10";

const PAGE_SIZE: u32 = 50;
const GQL_PAGE_CONCURRENCY: usize = 3;
pub const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
pub const TIMEOUT_SECS: u64 = 20;

struct CachedSession {
    session: Session,
    created_at: Instant,
}

struct CachedSecret {
    version: u32,
    key_bytes: Vec<u8>,
    fetched_at: Instant,
}

static CACHED_SESSION: Lazy<Mutex<Option<CachedSession>>> = Lazy::new(|| Mutex::new(None));
static CACHED_SECRET: Lazy<Mutex<Option<CachedSecret>>> = Lazy::new(|| Mutex::new(None));
const SESSION_MAX_AGE: Duration = Duration::from_secs(300);
const SECRET_MAX_AGE: Duration = Duration::from_secs(3600);

pub fn build_client(ua: &str, timeout_secs: u64) -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent(ua)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))
}

fn transform_secret(secret_bytes: &[u8]) -> Vec<u8> {
    let transformed: Vec<u8> = secret_bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ (((i as u8) % 33) + 9))
        .collect();
    let joined: String = transformed.iter().map(|b| b.to_string()).collect();
    let hex_str = hex::encode(joined.as_bytes());
    hex::decode(&hex_str).unwrap_or_default()
}

async fn fetch_latest_totp_secret(client: &Client) -> (u32, Vec<u8>) {
    {
        let cache = CACHED_SECRET.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < SECRET_MAX_AGE && !cached.key_bytes.is_empty() {
                return (cached.version, cached.key_bytes.clone());
            }
        }
    }

    if let Ok(resp) = client.get(SECRETS_URL).send().await {
        if resp.status().is_success() {
            if let Ok(secrets) = resp.json::<HashMap<String, Vec<u8>>>().await {
                if let Some((ver_str, secret)) = secrets
                    .iter()
                    .max_by_key(|(k, _)| k.parse::<u32>().unwrap_or(0))
                {
                    if let Ok(ver) = ver_str.parse::<u32>() {
                        let key_bytes = transform_secret(secret);
                        if !key_bytes.is_empty() {
                            let mut cache = CACHED_SECRET.lock().unwrap_or_else(|e| e.into_inner());
                            *cache = Some(CachedSecret {
                                version: ver,
                                key_bytes: key_bytes.clone(),
                                fetched_at: Instant::now(),
                            });
                            eprintln!("[Vynl] Fetched TOTP secret v{ver} from community endpoint");
                            return (ver, key_bytes);
                        }
                    }
                }
            }
        }
        eprintln!("[Vynl] Failed to fetch TOTP secret from community endpoint, using fallback");
    } else {
        eprintln!("[Vynl] Could not reach TOTP secrets endpoint, using fallback");
    }

    (
        FALLBACK_TOTP_VERSION,
        transform_secret(FALLBACK_TOTP_SECRET),
    )
}

fn generate_totp_from_key(key_bytes: &[u8], offset_secs: i64) -> Result<String, String> {
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Time error: {e}"))?;
    let secs = epoch.as_secs() as i64 + offset_secs;
    let counter = ((secs / 30) as u64).to_be_bytes();

    let mut mac = HmacSha1::new_from_slice(key_bytes).map_err(|e| format!("HMAC error: {e}"))?;
    mac.update(&counter);
    let result = mac.finalize().into_bytes();

    let off = (result[19] & 0x0F) as usize;
    let code_bytes = &result[off..off + 4];
    let code = ((code_bytes[0] as u32) << 24)
        | ((code_bytes[1] as u32) << 16)
        | ((code_bytes[2] as u32) << 8)
        | (code_bytes[3] as u32);
    let otp = format!("{:06}", code % 1_000_000);

    Ok(otp)
}

#[derive(Clone)]
pub struct Session {
    pub access_token: String,
    pub client_token: String,
    pub client_id: String,
    pub client_version: String,
    pub device_id: String,
}

pub async fn init_session(client: &Client) -> Result<Session, String> {
    {
        let cache = CACHED_SESSION.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref cached) = *cache {
            if cached.created_at.elapsed() < SESSION_MAX_AGE {
                return Ok(cached.session.clone());
            }
        }
    }

    let (access_token, client_id, cookies) = get_access_token(client).await?;

    let device_id = cookies
        .get("sp_t")
        .cloned()
        .unwrap_or_else(|| "vynl-device".to_string());

    let session_page = client
        .get("https://open.spotify.com")
        .header("User-Agent", UA)
        .send()
        .await
        .map_err(|e| format!("Session init failed: {e}"))?;

    let body = session_page.text().await.unwrap_or_default();
    let client_version = extract_client_version(&body);

    let client_token = get_client_token(client, &client_id, &device_id, &client_version).await?;

    let session = Session {
        access_token,
        client_token,
        client_id,
        client_version,
        device_id,
    };

    {
        let mut cache = CACHED_SESSION.lock().unwrap_or_else(|e| e.into_inner());
        *cache = Some(CachedSession {
            session: session.clone(),
            created_at: Instant::now(),
        });
    }

    Ok(session)
}

fn extract_client_version(html: &str) -> String {
    if let Some(start) = html.find("clientVersion") {
        let slice = &html[start..];
        if let Some(colon) = slice.find(':') {
            let after = &slice[colon + 1..].trim_start();
            if let Some(after_q) = after.strip_prefix('"') {
                if let Some(end) = after_q.find('"') {
                    return after_q[..end].to_string();
                }
            }
        }
    }
    "1.2.52.0".to_string()
}

async fn get_access_token(
    client: &Client,
) -> Result<(String, String, std::collections::HashMap<String, String>), String> {
    let (totp_version, key_bytes) = fetch_latest_totp_secret(client).await;

    let windows: [i64; 5] = [-60, -30, 0, 30, 60];
    let mut last_err = String::new();

    for offset in &windows {
        let totp_code = generate_totp_from_key(&key_bytes, *offset)?;
        eprintln!("[Vynl] TOTP code: {totp_code} (version: {totp_version}, offset: {offset}, key_len: {})", key_bytes.len());

        let url = format!(
            "https://open.spotify.com/api/token?reason=init&productType=web-player&totp={}&totpVer={}&totpServer={}",
            totp_code, totp_version, totp_code
        );

        let resp = client
            .get(&url)
            .header("User-Agent", UA)
            .header("Content-Type", "application/json;charset=UTF-8")
            .send()
            .await
            .map_err(|e| format!("Access token request failed: {e}"))?;

        let status = resp.status();
        let mut cookies = std::collections::HashMap::new();
        for cookie in resp.cookies() {
            cookies.insert(cookie.name().to_string(), cookie.value().to_string());
        }

        if status.is_success() {
            let data: Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse access token response: {e}"))?;

            let access_token = data
                .get("accessToken")
                .and_then(|v| v.as_str())
                .ok_or("No accessToken in response")?
                .to_string();
            let client_id = data
                .get("clientId")
                .and_then(|v| v.as_str())
                .ok_or("No clientId in response")?
                .to_string();

            return Ok((access_token, client_id, cookies));
        }

        let body = resp.text().await.unwrap_or_default();
        last_err = format!("TOTP offset {offset}: HTTP {status}, body: {body}");
        eprintln!("[Vynl] {last_err}");
        tokio::time::sleep(Duration::from_millis(800)).await;
    }

    Err(format!("All TOTP windows failed: {last_err}"))
}

async fn get_client_token(
    client: &Client,
    client_id: &str,
    device_id: &str,
    client_version: &str,
) -> Result<String, String> {
    let payload = json!({
        "client_data": {
            "client_version": client_version,
            "client_id": client_id,
            "js_sdk_data": {
                "device_brand": "unknown",
                "device_model": "unknown",
                "os": "windows",
                "os_version": "NT 10.0",
                "device_id": device_id,
                "device_type": "computer"
            }
        }
    });

    let resp = client
        .post("https://clienttoken.spotify.com/v1/clienttoken")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("User-Agent", UA)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Client token request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Client token HTTP {status}: {body}"));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse client token response: {e}"))?;

    data.get("granted_token")
        .and_then(|gt| gt.get("token"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "No token in client token response".to_string())
}

pub async fn graphql_query(
    client: &Client,
    session: &Session,
    payload: &Value,
) -> Result<Value, String> {
    for attempt in 0..3u32 {
        let resp = client
            .post("https://api-partner.spotify.com/pathfinder/v2/query")
            .header("Authorization", format!("Bearer {}", session.access_token))
            .header("Client-Token", &session.client_token)
            .header("Spotify-App-Version", &session.client_version)
            .header("Content-Type", "application/json")
            .header("User-Agent", UA)
            .json(payload)
            .send()
            .await
            .map_err(|e| format!("GraphQL request failed: {e}"))?;

        let status = resp.status();

        if status.as_u16() == 429 {
            let backoff_ms = 1000 * (attempt + 1);
            eprintln!(
                "[Vynl] GraphQL 429, retrying in {backoff_ms}ms (attempt {})",
                attempt + 1
            );
            tokio::time::sleep(Duration::from_millis(backoff_ms as u64)).await;
            continue;
        }

        if status.as_u16() == 401 || status.as_u16() == 403 {
            let mut cache = CACHED_SESSION.lock().unwrap_or_else(|e| e.into_inner());
            *cache = None;
        }

        if !status.is_success() {
            let _body = resp.text().await.unwrap_or_default();
            return Err(format!("GraphQL HTTP {status}"));
        }

        return resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse GraphQL response: {e}"));
    }

    Err("GraphQL: too many 429 retries".to_string())
}

fn page_count(total: u32) -> usize {
    if total == 0 {
        1
    } else {
        (total as usize).div_ceil(PAGE_SIZE as usize)
    }
}

fn playlist_payload(playlist_id: &str, offset: u32) -> Value {
    json!({
        "variables": {
            "uri": format!("spotify:playlist:{}", playlist_id),
            "offset": offset,
            "limit": PAGE_SIZE,
            "enableWatchFeedEntrypoint": false
        },
        "operationName": "fetchPlaylist",
        "extensions": {
            "persistedQuery": {
                "version": 1,
                "sha256Hash": FETCH_PLAYLIST_HASH
            }
        }
    })
}

fn album_payload(album_id: &str, offset: u32) -> Value {
    json!({
        "variables": {
            "uri": format!("spotify:album:{}", album_id),
            "locale": "",
            "offset": offset,
            "limit": PAGE_SIZE
        },
        "operationName": "getAlbum",
        "extensions": {
            "persistedQuery": {
                "version": 1,
                "sha256Hash": GET_ALBUM_HASH
            }
        }
    })
}

async fn fetch_pages(
    http: &Client,
    session: &Session,
    requests: Vec<(u32, Value)>,
) -> Result<Vec<Value>, String> {
    if requests.is_empty() {
        return Ok(Vec::new());
    }

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(GQL_PAGE_CONCURRENCY));
    let mut handles = Vec::new();

    for (offset, payload) in requests {
        let http = http.clone();
        let session = session.clone();
        let semaphore = std::sync::Arc::clone(&semaphore);
        handles.push((
            offset,
            tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(_) => return Err("page fetch aborted".to_string()),
                };
                graphql_query(&http, &session, &payload).await
            }),
        ));
    }

    let mut ordered: Vec<(u32, Result<Value, String>)> = Vec::new();
    for (offset, handle) in handles {
        let result = handle.await.map_err(|e| e.to_string())?;
        ordered.push((offset, result));
    }
    ordered.sort_by_key(|(offset, _)| *offset);

    let mut pages = Vec::new();
    for (_, result) in ordered {
        pages.push(result?);
    }
    Ok(pages)
}

pub async fn warm_session() -> Result<(), String> {
    let http = build_client(UA, TIMEOUT_SECS)?;
    init_session(&http).await.map(|_| ())
}

pub async fn fetch_all_playlist_tracks(playlist_id: &str) -> Result<Collection, String> {
    let http = build_client(UA, TIMEOUT_SECS)?;
    let session = init_session(&http).await?;

    let first = graphql_query(&http, &session, &playlist_payload(playlist_id, 0)).await?;

    let title: String;
    let owner: Option<String>;
    let cover: Option<String>;
    let total_count: u32;
    let mut all_items: Vec<Value>;

    {
        let pd = first
            .pointer("/data/playlistV2")
            .cloned()
            .unwrap_or_default();

        title = pd
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Playlist")
            .to_string();

        owner = pd
            .pointer("/ownerV2/data/name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        cover = extract_cover_from_gql(&pd);

        total_count = pd
            .pointer("/content/totalCount")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(0);

        all_items = pd
            .pointer("/content/items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
    }

    let pages = page_count(total_count);
    if pages > 1 {
        let requests: Vec<(u32, Value)> = (1..pages as u32)
            .map(|i| {
                let offset = i * PAGE_SIZE;
                (offset, playlist_payload(playlist_id, offset))
            })
            .collect();

        let responses = fetch_pages(&http, &session, requests).await?;
        for response in responses {
            let items = response
                .pointer("/data/playlistV2/content/items")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            all_items.extend(items);
        }
    }

    let mut seen_ids = std::collections::HashSet::new();
    let mut tracks: Vec<TrackMeta> = Vec::new();
    for (i, item) in all_items.iter().enumerate() {
        let track_data = item.pointer("/itemV2/data").cloned().unwrap_or_default();
        if track_data.is_null() {
            continue;
        }
        let track = parse_gql_track(&track_data, i);
        if track.id.is_empty() || !seen_ids.insert(track.id.clone()) {
            continue;
        }
        tracks.push(track);
    }

    Ok(Collection {
        kind: crate::commands::types::CollectionKind::Playlist,
        id: playlist_id.to_string(),
        title,
        owner,
        cover,
        tracks,
    })
}

pub async fn fetch_all_album_tracks(album_id: &str) -> Result<Collection, String> {
    let http = build_client(UA, TIMEOUT_SECS)?;
    let session = init_session(&http).await?;

    let first = graphql_query(&http, &session, &album_payload(album_id, 0)).await?;

    let album_name: String;
    let cover: Option<String>;
    let total_count: u32;
    let mut all_items: Vec<Value>;

    {
        let ad = first
            .pointer("/data/albumUnion")
            .cloned()
            .unwrap_or_default();

        album_name = ad
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Album")
            .to_string();

        cover = extract_cover_from_gql(&ad);

        let tracks_data = ad.get("tracksV2").cloned().unwrap_or_default();
        total_count = tracks_data
            .get("totalCount")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(0);
        all_items = tracks_data
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
    }

    let pages = page_count(total_count);
    if pages > 1 {
        let requests: Vec<(u32, Value)> = (1..pages as u32)
            .map(|i| {
                let offset = i * PAGE_SIZE;
                (offset, album_payload(album_id, offset))
            })
            .collect();

        let responses = fetch_pages(&http, &session, requests).await?;
        for response in responses {
            let items = response
                .pointer("/data/albumUnion/tracksV2/items")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            all_items.extend(items);
        }
    }

    let mut seen_ids = std::collections::HashSet::new();
    let mut tracks: Vec<TrackMeta> = Vec::new();
    for (i, item) in all_items.iter().enumerate() {
        let track = item.get("track").cloned().unwrap_or_default();
        let mut t = parse_gql_track(&track, i);
        t.album = album_name.clone();
        if t.id.is_empty() || !seen_ids.insert(t.id.clone()) {
            continue;
        }
        tracks.push(t);
    }

    Ok(Collection {
        kind: crate::commands::types::CollectionKind::Album,
        id: album_id.to_string(),
        title: album_name,
        owner: None,
        cover,
        tracks,
    })
}

fn parse_gql_track(track_data: &Value, index: usize) -> TrackMeta {
    let id = track_data
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            track_data
                .get("uri")
                .and_then(|v| v.as_str())
                .and_then(|uri| uri.rsplit(':').next())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or("")
        .to_string();

    let title = track_data
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&format!("Unknown Track {}", index + 1))
        .to_string();

    let artist = extract_artists_from_gql(track_data);
    let artist = if artist.is_empty() {
        "Unknown Artist".to_string()
    } else {
        artist
    };

    let album_data = track_data.get("albumOfTrack").cloned().unwrap_or_default();
    let album = album_data
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let duration_ms = track_data
        .pointer("/duration/totalMilliseconds")
        .or_else(|| track_data.pointer("/trackDuration/totalMilliseconds"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let duration = if duration_ms > 0.0 {
        Some((duration_ms / 1000.0).round())
    } else {
        None
    };

    let cover = extract_cover_from_gql(track_data).or_else(|| extract_cover_from_gql(&album_data));

    let track_number = Some((index + 1) as i64);

    let url = if !id.is_empty() {
        format!("https://open.spotify.com/track/{id}")
    } else {
        String::new()
    };

    TrackMeta {
        id,
        title,
        artist,
        album,
        year: None,
        duration,
        cover,
        track_number,
        url,
    }
}

fn extract_artists_from_gql(data: &Value) -> String {
    let artists_data = data.get("artists").cloned().unwrap_or_default();
    let items = artists_data
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let names: Vec<String> = items
        .iter()
        .filter_map(|item| {
            item.get("profile")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .collect();

    names.join(", ")
}

fn extract_cover_from_gql(data: &Value) -> Option<String> {
    if let Some(url) = extract_from_sources(data.pointer("/coverArt/sources")) {
        return Some(url);
    }

    if let Some(url) =
        extract_from_sources(data.pointer("/coverArt/squareCoverImage/image/data/sources"))
    {
        return Some(url);
    }

    if let Some(url) = extract_from_sources(data.pointer("/visualIdentity/image")) {
        return Some(url);
    }

    if let Some(items) = data.pointer("/images/items").and_then(|v| v.as_array()) {
        for item in items {
            if let Some(url) = extract_from_sources(item.pointer("/sources")) {
                return Some(url);
            }
        }
    }

    if let Some(items) = data.pointer("/imagesV2/items").and_then(|v| v.as_array()) {
        for item in items {
            if let Some(url) = extract_from_sources(item.pointer("/sources")) {
                return Some(url);
            }
        }
    }

    if let Some(url) = extract_from_sources(data.pointer("/images/sources")) {
        return Some(url);
    }

    None
}

fn extract_from_sources(val: Option<&Value>) -> Option<String> {
    let sources = match val? {
        Value::Array(arr) => arr,
        Value::Object(_) => return extract_url_from_object(val?),
        _ => return None,
    };

    let mut best_url = None;
    let mut best_width: u64 = 0;

    for source in sources {
        let url = source.get("url").and_then(|v| v.as_str())?;
        let width = source
            .get("width")
            .or_else(|| source.get("maxWidth"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let height = source
            .get("height")
            .or_else(|| source.get("maxHeight"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if width > best_width && (width > 64 || height > 64) {
            best_width = width;
            best_url = Some(url.to_string());
        }
    }

    best_url
}

fn extract_url_from_object(val: &Value) -> Option<String> {
    val.get("url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

pub async fn search_spotify(
    query: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<crate::commands::types::SpotifySearchResult>, String> {
    let http = build_client(UA, TIMEOUT_SECS)?;
    let session = init_session(&http).await?;

    let payload = json!({
        "operationName": "searchDesktop",
        "variables": {
            "searchTerm": query,
            "offset": offset,
            "limit": limit,
            "numberOfTopResults": 5,
            "includeAudiobooks": false,
            "includePodcasts": false,
            "includeAuthors": false,
            "includePreReleases": false,
            "includeArtistHasConcertsField": false
        },
        "extensions": {
            "persistedQuery": {
                "version": 1,
                "sha256Hash": "3c9d3f60dac5dea3876b6db3f534192b1c1d90032c4233c1bbaba526db41eb31"
            }
        }
    });

    let resp = http
        .post("https://api-partner.spotify.com/pathfinder/v2/query")
        .header("Authorization", format!("Bearer {}", session.access_token))
        .header("Client-Token", &session.client_token)
        .header("Spotify-App-Version", &session.client_version)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("User-Agent", UA)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Search request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let _body = resp.text().await.unwrap_or_default();
        return Err(format!("Search HTTP {status}"));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse search response: {e}"))?;

    let mut results = Vec::new();

    if let Some(items) = data
        .pointer("/data/searchV2/tracksV2/items")
        .and_then(|v| v.as_array())
    {
        for item in items {
            let track_data = item.pointer("/item/data").cloned().unwrap_or_default();

            let id = track_data
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let title = track_data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let artist = track_data
                .pointer("/artists/items")
                .and_then(|v| v.as_array())
                .and_then(|arr| {
                    let names: Vec<&str> = arr
                        .iter()
                        .filter_map(|a| {
                            a.get("profile")
                                .and_then(|p| p.get("name"))
                                .and_then(|n| n.as_str())
                        })
                        .collect();
                    if names.is_empty() {
                        None
                    } else {
                        Some(names.join(", "))
                    }
                })
                .unwrap_or_default();

            let album = track_data
                .pointer("/albumOfTrack/name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let duration_ms = track_data
                .pointer("/duration/totalMilliseconds")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let duration = if duration_ms > 0.0 {
                Some((duration_ms / 1000.0).round())
            } else {
                None
            };

            let cover = track_data
                .pointer("/albumOfTrack/coverArt/sources")
                .and_then(|v| v.as_array())
                .and_then(|sources| {
                    sources
                        .iter()
                        .filter_map(|s| s.get("url").and_then(|u| u.as_str()))
                        .max_by_key(|u| {
                            let w = sources
                                .iter()
                                .find(|ss| ss.get("url").and_then(|uu| uu.as_str()) == Some(*u))
                                .and_then(|ss| ss.get("width").and_then(|ww| ww.as_u64()))
                                .unwrap_or(0);
                            w
                        })
                        .map(|s| s.to_string())
                });

            let url = if !id.is_empty() {
                format!("https://open.spotify.com/track/{id}")
            } else {
                String::new()
            };

            if !title.is_empty() && !artist.is_empty() {
                results.push(crate::commands::types::SpotifySearchResult {
                    id,
                    title,
                    artist,
                    album,
                    duration,
                    cover,
                    url,
                });
            }
        }
    }

    results.truncate(limit as usize);
    Ok(results)
}
