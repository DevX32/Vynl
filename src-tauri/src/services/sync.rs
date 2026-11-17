use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::catalogue::types::{CatalogueAlbumRef, CatalogueArtistRef, CatalogueTrack};
use crate::commands::types::{Playlist, PlaylistMark};
use crate::services::{playlists, util};

pub const LIKED_PLAYLIST_ID: &str = "liked";

const SUPABASE_URL: &str = match option_env!("SUPABASE_URL") {
    Some(value) => value,
    None => "",
};
const SUPABASE_ANON_KEY: &str = match option_env!("SUPABASE_ANON_KEY") {
    Some(value) => value,
    None => "",
};
const TABLE: &str = "playlists";
const SESSION_DIR: &str = "cloud";
const SESSION_FILE: &str = "session.json";

enum LocalResolution {
    Present(Playlist),
    Duplicate,
    Missing,
}

fn catalogue_signature(playlist: &Playlist) -> std::collections::BTreeSet<String> {
    playlist
        .paths
        .iter()
        .filter_map(|p| p.strip_prefix("ytm:").map(|id| id.to_string()))
        .collect()
}

fn row_signature(row: &SyncRow) -> std::collections::BTreeSet<String> {
    row.tracks
        .iter()
        .map(|track| track.id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect()
}

fn resolve_local(
    output_dir: &Path,
    data_dir: &Path,
    row: &SyncRow,
    consumed: &mut std::collections::HashSet<String>,
) -> LocalResolution {
    if let Some(playlist) = playlists::get_playlist(output_dir, &row.local_id) {
        return LocalResolution::Present(playlist);
    }
    let wanted = row.name.trim().to_lowercase();
    let target = row_signature(row);
    let mut best: Option<Playlist> = None;
    let mut has_consumed = false;
    for meta in playlists::list_playlists(output_dir) {
        if consumed.contains(&meta.id) {
            has_consumed = true;
            continue;
        }
        let Some(playlist) = playlists::get_playlist(output_dir, &meta.id) else {
            continue;
        };
        if !playlist.name.trim().eq_ignore_ascii_case(&wanted) {
            continue;
        }
        if catalogue_signature(&playlist) != target {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|b| playlist.updated_at > b.updated_at)
        {
            best = Some(playlist);
        }
    }
    let Some(best) = best else {
        return if has_consumed {
            LocalResolution::Duplicate
        } else {
            LocalResolution::Missing
        };
    };
    consumed.insert(best.id.clone());
    match playlists::rekey_playlist(output_dir, data_dir, &best.id, &row.local_id) {
        Ok(playlist) => LocalResolution::Present(playlist),
        Err(_) => match playlists::get_playlist(output_dir, &row.local_id) {
            Some(playlist) => LocalResolution::Present(playlist),
            None => LocalResolution::Missing,
        },
    }
}

fn mark_map(marks: &[PlaylistMark]) -> std::collections::HashMap<&str, i64> {
    let mut map = std::collections::HashMap::new();
    for mark in marks {
        map.entry(mark.track_id.as_str())
            .and_modify(|at: &mut i64| {
                if mark.at > *at {
                    *at = mark.at;
                }
            })
            .or_insert(mark.at);
    }
    map
}

fn merge_marks(a: &[PlaylistMark], b: &[PlaylistMark]) -> Vec<PlaylistMark> {
    let combined: Vec<PlaylistMark> = a.iter().chain(b.iter()).cloned().collect();
    let map = mark_map(&combined);
    let mut merged: Vec<PlaylistMark> = map
        .into_iter()
        .map(|(track_id, at)| PlaylistMark {
            track_id: track_id.to_string(),
            at,
        })
        .collect();
    merged.sort_by(|x, y| y.at.cmp(&x.at).then_with(|| x.track_id.cmp(&y.track_id)));
    const MAX_MARKS: usize = 500;
    merged.truncate(MAX_MARKS);
    merged
}

fn remote_track_to_catalogue(track: &SyncTrack) -> CatalogueTrack {
    CatalogueTrack {
        id: track.id.clone(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        artists: vec![CatalogueArtistRef {
            name: track.artist.clone(),
            id: None,
        }],
        album: track.album.as_ref().map(|name| CatalogueAlbumRef {
            name: name.clone(),
            id: None,
        }),
        duration: track.duration,
        cover: track.cover.clone(),
        explicit: false,
    }
}

fn union_playlist(local: &Playlist, row: &SyncRow) -> Playlist {
    let adds = merge_marks(&local.adds, &row.adds);
    let removals = merge_marks(&local.removals, &row.removals);
    let add_map = mark_map(&adds);
    let rem_map = mark_map(&removals);
    let visible = |key: &str| {
        let add = add_map.get(key).copied().unwrap_or(0);
        let removal = rem_map.get(key).copied().unwrap_or(0);
        add >= removal
    };

    let local_first = local.updated_at >= row.updated_at;
    let remote_keys: Vec<String> = row
        .tracks
        .iter()
        .filter(|t| !t.id.trim().is_empty())
        .map(|t| format!("ytm:{}", t.id.trim()))
        .collect();
    let mut ordered: Vec<String> = Vec::with_capacity(local.paths.len() + remote_keys.len());
    if local_first {
        ordered.extend(local.paths.iter().cloned());
        for key in &remote_keys {
            if !ordered.contains(key) {
                ordered.push(key.clone());
            }
        }
    } else {
        ordered.extend(remote_keys.iter().cloned());
        for key in &local.paths {
            if !ordered.contains(key) {
                ordered.push(key.clone());
            }
        }
    }
    let ordered: Vec<String> = ordered.into_iter().filter(|k| visible(k)).collect();

    let local_streams: std::collections::HashMap<&str, &CatalogueTrack> =
        local.streams.iter().map(|t| (t.id.as_str(), t)).collect();
    let remote_tracks: std::collections::HashMap<&str, &SyncTrack> =
        row.tracks.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut streams: Vec<CatalogueTrack> = Vec::new();
    for key in &ordered {
        let Some(id) = key.strip_prefix("ytm:") else {
            continue;
        };
        if let Some(track) = local_streams.get(id) {
            streams.push((*track).clone());
        } else if let Some(track) = remote_tracks.get(id) {
            streams.push(remote_track_to_catalogue(track));
        }
    }

    let local_newer = local.updated_at >= row.updated_at;
    let name = if local_newer {
        local.name.clone()
    } else {
        row.name.trim().to_string()
    };
    Playlist {
        id: row.local_id.clone(),
        name: if name.is_empty() {
            "Untitled".into()
        } else {
            name
        },
        paths: ordered,
        streams,
        adds,
        removals,
        cover: local.cover.clone(),
        created_at: local.created_at.min(row.created_at),
        updated_at: local.updated_at.max(row.updated_at),
    }
}

fn merge_signature(pl: &Playlist) -> String {
    serde_json::json!({
        "paths": pl.paths,
        "streams": pl.streams.iter().map(|t| &t.id).collect::<Vec<_>>(),
        "adds": pl.adds,
        "removals": pl.removals,
        "name": pl.name,
        "createdAt": pl.created_at,
        "updatedAt": pl.updated_at,
    })
    .to_string()
}

pub(crate) fn ensure_hidden_liked(
    output_dir: &Path,
    user_data_dir: &Path,
) -> Result<Option<Playlist>, String> {
    if let Some(existing) = playlists::get_playlist(output_dir, LIKED_PLAYLIST_ID) {
        return Ok(Some(existing));
    }
    let entries = crate::services::taste::load(user_data_dir);
    let liked: Vec<_> = entries.iter().filter(|entry| entry.liked).collect();
    if liked.is_empty() {
        return Ok(None);
    }
    let now = util::now_ms();
    let created_at = liked
        .iter()
        .filter_map(|entry| entry.liked_at)
        .min()
        .map(|at| at.min(i64::MAX as u64) as i64)
        .unwrap_or(now);
    let playlist = Playlist {
        id: LIKED_PLAYLIST_ID.to_string(),
        name: "Liked Songs".to_string(),
        paths: liked
            .iter()
            .map(|entry| format!("ytm:{}", entry.track.id))
            .collect(),
        streams: liked.iter().map(|entry| entry.track.clone()).collect(),
        adds: liked
            .iter()
            .map(|entry| PlaylistMark {
                track_id: format!("ytm:{}", entry.track.id),
                at: entry
                    .liked_at
                    .map(|at| at.min(i64::MAX as u64) as i64)
                    .unwrap_or(now),
            })
            .collect(),
        removals: Vec::new(),
        cover: None,
        created_at,
        updated_at: now,
    };
    playlists::upsert_playlist(output_dir, &playlist)?;
    Ok(Some(playlist))
}

fn reconcile_taste(user_data_dir: &Path, liked: &Playlist) -> Result<(), String> {
    let mut next = crate::services::taste::load(user_data_dir);
    let add_map = mark_map(&liked.adds);
    let rem_map = mark_map(&liked.removals);
    let stream_by_id: std::collections::HashMap<&str, &CatalogueTrack> = liked
        .streams
        .iter()
        .map(|track| (track.id.as_str(), track))
        .collect();
    let mut changed = false;

    for entry in next.iter_mut() {
        if !entry.liked {
            continue;
        }
        let key = format!("ytm:{}", entry.track.id);
        let removal = rem_map.get(key.as_str()).copied().unwrap_or(0);
        if removal == 0 {
            continue;
        }
        let add = add_map.get(key.as_str()).copied().unwrap_or(0);
        if removal > add {
            entry.liked = false;
            changed = true;
        }
    }

    for key in &liked.paths {
        let Some(id) = key.strip_prefix("ytm:") else {
            continue;
        };
        if let Some(entry) = next.iter_mut().find(|entry| entry.track.id == id) {
            if entry.liked {
                continue;
            }
            entry.liked = true;
            entry.liked_at = add_map.get(key.as_str()).copied().map(|at| at as u64);
            changed = true;
            continue;
        }
        let Some(track) = stream_by_id.get(id) else {
            continue;
        };
        next.push(crate::services::taste::TasteEntry {
            track: (*track).clone(),
            plays: 0,
            last_played: 0,
            liked: true,
            liked_at: add_map.get(key.as_str()).copied().map(|at| at as u64),
        });
        changed = true;
    }

    if changed {
        crate::services::taste::save(user_data_dir, &next)?;
    }
    Ok(())
}

fn sync_enabled() -> bool {
    !base_url().is_empty() && !anon_key().is_empty()
}

fn base_url() -> &'static str {
    SUPABASE_URL.trim().trim_end_matches('/')
}

fn anon_key() -> &'static str {
    SUPABASE_ANON_KEY.trim()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub email: String,
    pub user_id: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub signed_in: bool,
    pub email: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncAuthResult {
    pub needs_confirmation: bool,
    pub status: SyncStatus,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub pushed: usize,
    pub pulled: usize,
    pub deleted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncTrack {
    id: String,
    title: String,
    artist: String,
    album: Option<String>,
    duration: Option<f64>,
    cover: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyncRow {
    local_id: String,
    name: String,
    #[serde(default)]
    tracks: Vec<SyncTrack>,
    created_at: i64,
    updated_at: i64,
    #[serde(default)]
    deleted_at: Option<i64>,
    #[serde(default)]
    adds: Vec<PlaylistMark>,
    #[serde(default)]
    removals: Vec<PlaylistMark>,
}

fn now_ms() -> i64 {
    util::now_ms()
}

fn session_path(data_dir: &Path) -> PathBuf {
    data_dir.join(SESSION_DIR).join(SESSION_FILE)
}

fn read_session(data_dir: &Path) -> Option<Session> {
    util::read_json::<Session>(&session_path(data_dir))
}

fn write_session(data_dir: &Path, session: &Session) -> Result<(), String> {
    let dir = data_dir.join(SESSION_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not save the session: {e}"))?;
    util::write_json(&session_path(data_dir), session)
        .map_err(|e| format!("Could not save the session: {e}"))
}

fn clear_session(data_dir: &Path) {
    let _ = std::fs::remove_file(session_path(data_dir));
}

fn http() -> Result<reqwest::Client, String> {
    if !sync_enabled() {
        return Err("Vynl isn't configured.".into());
    }
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("Could not reach Vynl Sync: {e}"))
}

fn auth_error(status: reqwest::StatusCode, body: &str) -> String {
    let code = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| {
            v.get("error_code")
                .or_else(|| v.get("code"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default();
    match code.as_str() {
        "invalid_credentials" => "Wrong email or password.".into(),
        "email_not_confirmed" => "Confirm your email first, then sign in.".into(),
        "user_already_exists" | "email_exists" => "That email is already registered.".into(),
        "weak_password" => "That password is too weak.".into(),
        "over_email_send_rate_limit" => {
            "Too many emails sent — wait a minute and try again.".into()
        }
        "same_password" => "Pick a different password.".into(),
        _ => {
            let msg = serde_json::from_str::<Value>(body)
                .ok()
                .and_then(|v| {
                    v.get("msg")
                        .or_else(|| v.get("message"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .filter(|m| !m.is_empty());
            if status.is_server_error() {
                format!(
                    "Vynl Sync had a problem ({}). Try again later.",
                    status.as_u16()
                )
            } else {
                msg.unwrap_or_else(|| format!("Sign-in failed ({}).", status.as_u16()))
            }
        }
    }
}

fn parse_session(value: &Value, email_hint: &str) -> Option<Session> {
    let access_token = value.get("access_token")?.as_str()?.to_string();
    let refresh_token = value
        .get("refresh_token")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let expires_in = value
        .get("expires_in")
        .and_then(Value::as_i64)
        .unwrap_or(3600);
    let user = value.get("user");
    let user_id = user
        .and_then(|u| u.get("id"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if user_id.is_empty() {
        return None;
    }
    let email = user
        .and_then(|u| u.get("email"))
        .and_then(Value::as_str)
        .unwrap_or(email_hint)
        .to_string();
    Some(Session {
        email,
        user_id,
        access_token,
        refresh_token,
        expires_at: now_ms() + expires_in.saturating_mul(1000),
    })
}

static REFRESH_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn refresh_lock() -> &'static Mutex<()> {
    REFRESH_LOCK.get_or_init(|| Mutex::new(()))
}

async fn sign_in_with_password(email: &str, password: &str) -> Result<Session, String> {
    let response = http()?
        .post(format!("{}/auth/v1/token?grant_type=password", base_url()))
        .header("apikey", anon_key())
        .header("Content-Type", "application/json")
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await
        .map_err(|e| format!("Could not reach Vynl Sync: {e}"))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(auth_error(status, &body));
    }
    let value: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    parse_session(&value, email).ok_or_else(|| "Vynl Sync returned an empty session.".into())
}

async fn sign_up_with_password(email: &str, password: &str) -> Result<Option<Session>, String> {
    let response = http()?
        .post(format!("{}/auth/v1/signup", base_url()))
        .header("apikey", anon_key())
        .header("Content-Type", "application/json")
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await
        .map_err(|e| format!("Could not reach Vynl Sync: {e}"))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(auth_error(status, &body));
    }
    let value: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(parse_session(&value, email))
}

async fn refresh_session(data_dir: &Path, session: &Session) -> Result<Session, String> {
    if session.refresh_token.is_empty() {
        clear_session(data_dir);
        return Err("Your session expired. Sign in again.".into());
    }
    let response = http()?
        .post(format!(
            "{}/auth/v1/token?grant_type=refresh_token",
            base_url()
        ))
        .header("apikey", anon_key())
        .header("Content-Type", "application/json")
        .json(&json!({ "refresh_token": session.refresh_token }))
        .send()
        .await
        .map_err(|e| format!("Could not reach Vynl Sync: {e}"))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        clear_session(data_dir);
        return Err("Your session expired. Sign in again.".into());
    }
    let value: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let mut fresh = parse_session(&value, &session.email)
        .ok_or_else(|| "Vynl Sync returned an empty session.".to_string())?;
    if fresh.refresh_token.is_empty() {
        fresh.refresh_token = session.refresh_token.clone();
    }
    write_session(data_dir, &fresh)?;
    Ok(fresh)
}

async fn valid_session(data_dir: &Path) -> Result<Session, String> {
    let session = read_session(data_dir).ok_or("You are not signed in.")?;
    if session.expires_at > now_ms() + 30_000 {
        return Ok(session);
    }
    let _guard = refresh_lock().lock().await;
    let session = read_session(data_dir).ok_or("You are not signed in.")?;
    if session.expires_at > now_ms() + 30_000 {
        return Ok(session);
    }
    refresh_session(data_dir, &session).await
}

pub fn status(data_dir: &Path) -> SyncStatus {
    match read_session(data_dir) {
        Some(session) => SyncStatus {
            signed_in: true,
            email: Some(session.email),
            user_id: Some(session.user_id),
        },
        None => SyncStatus {
            signed_in: false,
            email: None,
            user_id: None,
        },
    }
}

pub async fn sign_in(
    data_dir: &Path,
    email: &str,
    password: &str,
) -> Result<SyncAuthResult, String> {
    let email = email.trim().to_lowercase();
    if !email.contains('@') {
        return Err("Enter a valid email address.".into());
    }
    if password.is_empty() {
        return Err("Enter your password.".into());
    }
    let session = sign_in_with_password(&email, password).await?;
    write_session(data_dir, &session)?;
    Ok(SyncAuthResult {
        needs_confirmation: false,
        status: status(data_dir),
    })
}

pub async fn sign_up(
    data_dir: &Path,
    email: &str,
    password: &str,
) -> Result<SyncAuthResult, String> {
    let email = email.trim().to_lowercase();
    if !email.contains('@') {
        return Err("Enter a valid email address.".into());
    }
    if password.len() < 6 {
        return Err("Use at least 6 characters for the password.".into());
    }
    match sign_up_with_password(&email, password).await? {
        Some(session) => {
            write_session(data_dir, &session)?;
            Ok(SyncAuthResult {
                needs_confirmation: false,
                status: status(data_dir),
            })
        }
        None => Ok(SyncAuthResult {
            needs_confirmation: true,
            status: status(data_dir),
        }),
    }
}

pub async fn confirm_email(token_hash: &str, email_type: &str) -> Result<(), String> {
    if token_hash.is_empty() {
        return Err("Missing token_hash.".into());
    }
    if email_type.is_empty() {
        return Err("Missing type.".into());
    }
    if !sync_enabled() {
        return Err("Vynl isn't configured.".into());
    }
    let response = http()?
        .post(format!("{}/auth/v1/verify", base_url()))
        .header("apikey", anon_key())
        .header("Content-Type", "application/json")
        .json(&json!({
            "type": email_type,
            "token_hash": token_hash,
        }))
        .send()
        .await
        .map_err(|e| format!("Could not reach Vynl Sync: {e}"))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(auth_error(status, &body));
    }
    Ok(())
}

pub async fn sign_out(data_dir: &Path) -> Result<(), String> {
    if let Some(session) = read_session(data_dir)
        && sync_enabled()
    {
        let _ = http()?
            .post(format!("{}/auth/v1/logout", base_url()))
            .header("apikey", anon_key())
            .header("Authorization", format!("Bearer {}", session.access_token))
            .send()
            .await;
    }
    clear_session(data_dir);
    Ok(())
}

async fn rest_headers(
    data_dir: &Path,
    request: reqwest::RequestBuilder,
) -> Result<reqwest::RequestBuilder, String> {
    let session = valid_session(data_dir).await?;
    Ok(request
        .header("apikey", anon_key())
        .header("Authorization", format!("Bearer {}", session.access_token)))
}

static MARKS_SUPPORTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

fn marks_supported() -> bool {
    MARKS_SUPPORTED.load(std::sync::atomic::Ordering::Relaxed)
}

fn set_marks_supported(supported: bool) {
    MARKS_SUPPORTED.store(supported, std::sync::atomic::Ordering::Relaxed);
}

fn body_missing_marks(body: &str) -> bool {
    let lowered = body.to_lowercase();
    (lowered.contains("column") && (lowered.contains("adds") || lowered.contains("removals")))
        || lowered.contains("schema cache") && lowered.contains("playlists")
}

fn select_query() -> &'static str {
    if marks_supported() {
        "select=local_id,name,tracks,created_at,updated_at,deleted_at,adds,removals"
    } else {
        "select=local_id,name,tracks,created_at,updated_at,deleted_at"
    }
}

async fn fetch_rows_once(
    data_dir: &Path,
    query: &'static str,
) -> Result<reqwest::Response, String> {
    let request = http()?
        .get(format!("{}/rest/v1/{}?{}", base_url(), TABLE, query))
        .header("Accept", "application/json");
    rest_headers(data_dir, request)
        .await?
        .send()
        .await
        .map_err(|e| format!("Could not reach Vynl Sync: {e}"))
}

async fn fetch_rows(data_dir: &Path) -> Result<Vec<SyncRow>, String> {
    let query = select_query();
    let mut response = fetch_rows_once(data_dir, query).await?;
    let mut status = response.status();
    let mut body = response.text().await.unwrap_or_default();
    if !status.is_success() && query.contains("adds") && body_missing_marks(&body) {
        set_marks_supported(false);
        response = fetch_rows_once(
            data_dir,
            "select=local_id,name,tracks,created_at,updated_at,deleted_at",
        )
        .await?;
        status = response.status();
        body = response.text().await.unwrap_or_default();
    }
    if !status.is_success() {
        return Err(format!(
            "Could not load synced playlists ({}).",
            status.as_u16()
        ));
    }
    serde_json::from_str(&body).map_err(|e| format!("Unexpected Vynl Sync response: {e}"))
}

async fn upsert_rows_once(data_dir: &Path, rows: &[Value]) -> Result<reqwest::Response, String> {
    let request = http()?
        .post(format!(
            "{}/rest/v1/{}?on_conflict=user_id,local_id",
            base_url(),
            TABLE
        ))
        .header("Content-Type", "application/json")
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(rows);
    rest_headers(data_dir, request)
        .await?
        .send()
        .await
        .map_err(|e| format!("Could not reach Vynl Sync: {e}"))
}

async fn upsert_rows(data_dir: &Path, rows: &[Value]) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    let payload = if marks_supported() {
        rows.to_vec()
    } else {
        rows.iter().map(strip_marks).collect()
    };
    let response = upsert_rows_once(data_dir, &payload).await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        if marks_supported() && body_missing_marks(&body) {
            set_marks_supported(false);
            let stripped: Vec<Value> = rows.iter().map(strip_marks).collect();
            let response = upsert_rows_once(data_dir, &stripped).await?;
            if !response.status().is_success() {
                return Err(format!(
                    "Could not save to Vynl Sync ({}).",
                    response.status().as_u16()
                ));
            }
            return Ok(());
        }
        return Err(format!(
            "Could not save to Vynl Sync ({}).",
            status.as_u16()
        ));
    }
    Ok(())
}

fn track_dto(track: &CatalogueTrack) -> SyncTrack {
    SyncTrack {
        id: track.id.clone(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.as_ref().map(|album| album.name.clone()),
        duration: track.duration,
        cover: track
            .cover
            .as_deref()
            .map(crate::services::cover_cache::normalize_cover_url)
            .map(|cover| cover.into_owned()),
    }
}

fn playlist_value(session: &Session, playlist: &Playlist) -> Value {
    json!({
        "user_id": session.user_id,
        "local_id": playlist.id,
        "name": playlist.name,
        "tracks": playlist.streams.iter().map(track_dto).collect::<Vec<_>>(),
        "created_at": playlist.created_at,
        "updated_at": playlist.updated_at,
        "deleted_at": Value::Null,
        "adds": playlist.adds,
        "removals": playlist.removals,
    })
}

fn strip_marks(row: &Value) -> Value {
    let mut row = row.clone();
    if let Some(object) = row.as_object_mut() {
        object.remove("adds");
        object.remove("removals");
    }
    row
}

pub async fn push_playlist(data_dir: &Path, playlist: &Playlist) {
    if !sync_enabled() {
        return;
    }
    if read_session(data_dir).is_none() {
        return;
    }
    let Ok(session) = valid_session(data_dir).await else {
        return;
    };
    let _ = upsert_rows(data_dir, &[playlist_value(&session, playlist)]).await;
}

pub async fn tombstone_playlist(data_dir: &Path, id: &str) {
    if !sync_enabled() {
        return;
    }
    let Some(session) = read_session(data_dir) else {
        return;
    };
    if valid_session(data_dir).await.is_err() {
        return;
    }
    let Ok(client) = http() else {
        return;
    };
    if !crate::services::playlists::validate_id(id) {
        return;
    }
    let request = client
        .patch(format!(
            "{}/rest/v1/{}?user_id=eq.{}&local_id=eq.{}",
            base_url(),
            TABLE,
            session.user_id,
            urlencoding::encode(id),
        ))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=minimal")
        .json(&json!({ "deleted_at": now_ms() }));
    if let Ok(request) = rest_headers(data_dir, request).await {
        let _ = request.send().await;
    }
}

fn row_to_playlist(row: &SyncRow) -> Playlist {
    let mut playlist = Playlist {
        id: row.local_id.clone(),
        name: row.name.trim().to_string(),
        paths: Vec::new(),
        streams: Vec::new(),
        cover: None,
        created_at: row.created_at,
        updated_at: row.updated_at,
        adds: row.adds.clone(),
        removals: row.removals.clone(),
    };
    if playlist.name.is_empty() {
        playlist.name = "Untitled".into();
    }
    for track in &row.tracks {
        if track.id.trim().is_empty() {
            continue;
        }
        playlist.paths.push(format!("ytm:{}", track.id));
        playlist.streams.push(remote_track_to_catalogue(track));
    }
    playlist
}

pub async fn sync(data_dir: &Path, output_dir: &Path) -> Result<SyncResult, String> {
    if !sync_enabled() {
        return Err("Vynl isn't configured.".into());
    }
    let session = valid_session(data_dir).await?;
    let rows = fetch_rows(data_dir).await?;
    let mut result = SyncResult::default();
    let mut known: std::collections::HashSet<String> = std::collections::HashSet::new();

    let _ = ensure_hidden_liked(output_dir, data_dir);

    let mut consumed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for row in &rows {
        known.insert(row.local_id.clone());
        let local = match resolve_local(output_dir, data_dir, row, &mut consumed) {
            LocalResolution::Present(playlist) => Some(playlist),
            LocalResolution::Duplicate => continue,
            LocalResolution::Missing => None,
        };
        match row.deleted_at {
            Some(deleted_at) => match local {
                Some(playlist) if playlist.updated_at <= deleted_at => {
                    playlists::delete_playlist(output_dir, data_dir, &row.local_id);
                    result.deleted += 1;
                }
                Some(playlist) => {
                    upsert_rows(data_dir, &[playlist_value(&session, &playlist)]).await?;
                    result.pushed += 1;
                }
                None => {}
            },
            None => match local {
                None => {
                    let synced = row_to_playlist(row);
                    playlists::upsert_playlist(output_dir, &synced)?;
                    result.pulled += 1;
                }
                Some(playlist) => {
                    let merged = union_playlist(&playlist, row);
                    if merge_signature(&merged) != merge_signature(&playlist) {
                        playlists::upsert_playlist(output_dir, &merged)?;
                        let remote_change =
                            merged.paths.iter().any(|p| !playlist.paths.contains(p))
                                || merged.removals.iter().any(|m| {
                                    playlist
                                        .removals
                                        .iter()
                                        .all(|r| r.track_id != m.track_id || r.at < m.at)
                                });
                        if remote_change {
                            result.pulled += 1;
                        } else {
                            result.pushed += 1;
                        }
                    }
                    upsert_rows(data_dir, &[playlist_value(&session, &merged)]).await?;
                }
            },
        }
    }

    for meta in playlists::list_playlists(output_dir) {
        if known.contains(&meta.id) {
            continue;
        }
        if let Some(playlist) = playlists::get_playlist(output_dir, &meta.id) {
            upsert_rows(data_dir, &[playlist_value(&session, &playlist)]).await?;
            result.pushed += 1;
        }
    }

    if let Some(liked) = playlists::get_playlist(output_dir, LIKED_PLAYLIST_ID) {
        let _ = reconcile_taste(data_dir, &liked);
    }

    Ok(result)
}
