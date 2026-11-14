use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::catalogue::types::{CatalogueAlbumRef, CatalogueArtistRef, CatalogueTrack};
use crate::commands::types::Playlist;
use crate::services::{playlists, util};

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

fn cloud_enabled() -> bool {
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
pub struct CloudStatus {
    pub signed_in: bool,
    pub email: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAuthResult {
    pub needs_confirmation: bool,
    pub status: CloudStatus,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncResult {
    pub pushed: usize,
    pub pulled: usize,
    pub deleted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudTrack {
    id: String,
    title: String,
    artist: String,
    album: Option<String>,
    duration: Option<f64>,
    cover: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CloudRow {
    local_id: String,
    name: String,
    #[serde(default)]
    tracks: Vec<CloudTrack>,
    created_at: i64,
    updated_at: i64,
    #[serde(default)]
    deleted_at: Option<i64>,
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
    if !cloud_enabled() {
        return Err("Vynl Cloud isn't configured.".into());
    }
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("Could not reach the cloud: {e}"))
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
                    "The cloud had a problem ({}). Try again later.",
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
        .map_err(|e| format!("Could not reach the cloud: {e}"))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(auth_error(status, &body));
    }
    let value: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    parse_session(&value, email).ok_or_else(|| "The cloud returned an empty session.".into())
}

async fn sign_up_with_password(email: &str, password: &str) -> Result<Option<Session>, String> {
    let response = http()?
        .post(format!("{}/auth/v1/signup", base_url()))
        .header("apikey", anon_key())
        .header("Content-Type", "application/json")
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await
        .map_err(|e| format!("Could not reach the cloud: {e}"))?;
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
        .map_err(|e| format!("Could not reach the cloud: {e}"))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        clear_session(data_dir);
        return Err("Your session expired. Sign in again.".into());
    }
    let value: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let mut fresh = parse_session(&value, &session.email)
        .ok_or_else(|| "The cloud returned an empty session.".to_string())?;
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

pub fn status(data_dir: &Path) -> CloudStatus {
    match read_session(data_dir) {
        Some(session) => CloudStatus {
            signed_in: true,
            email: Some(session.email),
            user_id: Some(session.user_id),
        },
        None => CloudStatus {
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
) -> Result<CloudAuthResult, String> {
    let email = email.trim().to_lowercase();
    if !email.contains('@') {
        return Err("Enter a valid email address.".into());
    }
    if password.is_empty() {
        return Err("Enter your password.".into());
    }
    let session = sign_in_with_password(&email, password).await?;
    write_session(data_dir, &session)?;
    Ok(CloudAuthResult {
        needs_confirmation: false,
        status: status(data_dir),
    })
}

pub async fn sign_up(
    data_dir: &Path,
    email: &str,
    password: &str,
) -> Result<CloudAuthResult, String> {
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
            Ok(CloudAuthResult {
                needs_confirmation: false,
                status: status(data_dir),
            })
        }
        None => Ok(CloudAuthResult {
            needs_confirmation: true,
            status: status(data_dir),
        }),
    }
}

pub async fn sign_out(data_dir: &Path) -> Result<(), String> {
    if let Some(session) = read_session(data_dir) {
        if cloud_enabled() {
            let _ = http()?
                .post(format!("{}/auth/v1/logout", base_url()))
                .header("apikey", anon_key())
                .header("Authorization", format!("Bearer {}", session.access_token))
                .send()
                .await;
        }
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

async fn fetch_rows(data_dir: &Path) -> Result<Vec<CloudRow>, String> {
    let request = http()?
        .get(format!(
            "{}/rest/v1/{}?select=local_id,name,tracks,created_at,updated_at,deleted_at",
            base_url(),
            TABLE
        ))
        .header("Accept", "application/json");
    let response = rest_headers(data_dir, request)
        .await?
        .send()
        .await
        .map_err(|e| format!("Could not reach the cloud: {e}"))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "Could not load cloud playlists ({}): {}",
            status.as_u16(),
            body.trim()
        ));
    }
    serde_json::from_str(&body).map_err(|e| format!("Unexpected cloud response: {e}"))
}

async fn upsert_rows(data_dir: &Path, rows: &[Value]) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    let request = http()?
        .post(format!(
            "{}/rest/v1/{}?on_conflict=user_id,local_id",
            base_url(),
            TABLE
        ))
        .header("Content-Type", "application/json")
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(rows);
    let response = rest_headers(data_dir, request)
        .await?
        .send()
        .await
        .map_err(|e| format!("Could not reach the cloud: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Could not save to the cloud ({}): {}",
            status.as_u16(),
            body.trim()
        ));
    }
    Ok(())
}

fn track_dto(track: &CatalogueTrack) -> CloudTrack {
    CloudTrack {
        id: track.id.clone(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.as_ref().map(|album| album.name.clone()),
        duration: track.duration,
        cover: track.cover.clone(),
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
    })
}

pub async fn push_playlist(data_dir: &Path, playlist: &Playlist) {
    if !cloud_enabled() {
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
    if !cloud_enabled() {
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
    let request = client
        .patch(format!(
            "{}/rest/v1/{}?user_id=eq.{}&local_id=eq.{}",
            base_url(),
            TABLE,
            session.user_id,
            id
        ))
        .header("Content-Type", "application/json")
        .header("Prefer", "return=minimal")
        .json(&json!({ "deleted_at": now_ms() }));
    match rest_headers(data_dir, request).await {
        Ok(request) => {
            let _ = request.send().await;
        }
        Err(_) => {}
    }
}

fn row_to_playlist(row: &CloudRow) -> Playlist {
    let mut playlist = Playlist {
        id: row.local_id.clone(),
        name: row.name.trim().to_string(),
        paths: Vec::new(),
        streams: Vec::new(),
        cover: None,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };
    if playlist.name.is_empty() {
        playlist.name = "Untitled".into();
    }
    for track in &row.tracks {
        playlist.paths.push(format!("ytm:{}", track.id));
        playlist.streams.push(CatalogueTrack {
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
        });
    }
    playlist
}

pub async fn sync(data_dir: &Path, output_dir: &Path) -> Result<CloudSyncResult, String> {
    if !cloud_enabled() {
        return Err("Vynl Cloud isn't configured.".into());
    }
    let session = valid_session(data_dir).await?;
    let rows = fetch_rows(data_dir).await?;
    let mut result = CloudSyncResult::default();
    let mut known: std::collections::HashSet<String> = std::collections::HashSet::new();

    for row in &rows {
        known.insert(row.local_id.clone());
        let local = playlists::get_playlist(output_dir, &row.local_id);
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
                    let cloud = row_to_playlist(row);
                    playlists::upsert_playlist(output_dir, &cloud)?;
                    result.pulled += 1;
                }
                Some(playlist) if playlist.updated_at < row.updated_at => {
                    let mut cloud = row_to_playlist(row);
                    cloud.cover = playlist.cover.clone();
                    playlists::upsert_playlist(output_dir, &cloud)?;
                    result.pulled += 1;
                }
                Some(playlist) if playlist.updated_at > row.updated_at => {
                    upsert_rows(data_dir, &[playlist_value(&session, &playlist)]).await?;
                    result.pushed += 1;
                }
                Some(_) => {}
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

    Ok(result)
}
