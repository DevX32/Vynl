use std::path::{Path, PathBuf};
use std::sync::{Mutex as StdMutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::catalogue::types::{CatalogueAlbumRef, CatalogueArtistRef, CatalogueTrack};
use crate::commands::types::Playlist;
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
const SESSION_DIR: &str = "cloud";
const SESSION_FILE: &str = "session.json";
const SYNC_META_FILE: &str = "sync-meta.json";
const SUPPORTED_SYNC_VERSION: u32 = 1;

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
    pub last_synced_at: Option<i64>,
    pub last_error: Option<String>,
    pub pending: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncAuthResult {
    pub needs_confirmation: bool,
    pub status: SyncStatus,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SyncMeta {
    #[serde(default)]
    dirty: bool,
    #[serde(default)]
    last_synced_at: Option<i64>,
    #[serde(default)]
    last_error: Option<String>,
}

fn sync_meta_path(data_dir: &Path) -> PathBuf {
    data_dir.join(SESSION_DIR).join(SYNC_META_FILE)
}

fn read_sync_meta(data_dir: &Path) -> SyncMeta {
    util::read_json::<SyncMeta>(&sync_meta_path(data_dir)).unwrap_or_default()
}

fn write_sync_meta(data_dir: &Path, meta: &SyncMeta) {
    let dir = data_dir.join(SESSION_DIR);
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let _ = util::write_json(&sync_meta_path(data_dir), meta);
}

static SYNC_META_LOCK: StdMutex<()> = StdMutex::new(());

fn sync_meta_lock() -> std::sync::MutexGuard<'static, ()> {
    SYNC_META_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn mark_sync_dirty(data_dir: &Path) {
    let _guard = sync_meta_lock();
    let mut meta = read_sync_meta(data_dir);
    if meta.dirty {
        return;
    }
    meta.dirty = true;
    write_sync_meta(data_dir, &meta);
}

pub fn is_sync_dirty(data_dir: &Path) -> bool {
    read_sync_meta(data_dir).dirty
}

pub fn record_sync_error(data_dir: &Path, error: &str) {
    let _guard = sync_meta_lock();
    let mut meta = read_sync_meta(data_dir);
    meta.last_error = Some(error.to_string());
    write_sync_meta(data_dir, &meta);
}

pub fn clear_sync_error(data_dir: &Path) {
    let _guard = sync_meta_lock();
    let mut meta = read_sync_meta(data_dir);
    meta.last_error = None;
    write_sync_meta(data_dir, &meta);
}

pub fn set_sync_result(data_dir: &Path, error: Option<&str>) {
    let _guard = sync_meta_lock();
    let mut meta = read_sync_meta(data_dir);
    meta.dirty = error.is_some();
    meta.last_error = error.map(str::to_string);
    if error.is_none() {
        meta.last_synced_at = Some(util::now_ms());
    }
    write_sync_meta(data_dir, &meta);
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
        if matches!(status.as_u16(), 400 | 401 | 403) {
            clear_session(data_dir);
            return Err("Your session expired. Sign in again.".into());
        }
        return Err(format!(
            "Vynl Sync had a problem ({}). Try again later.",
            status.as_u16()
        ));
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
    let meta = read_sync_meta(data_dir);
    match read_session(data_dir) {
        Some(session) => SyncStatus {
            signed_in: true,
            email: Some(session.email),
            user_id: Some(session.user_id),
            last_synced_at: meta.last_synced_at,
            last_error: meta.last_error,
            pending: meta.dirty,
        },
        None => SyncStatus {
            signed_in: false,
            email: None,
            user_id: None,
            last_synced_at: None,
            last_error: None,
            pending: false,
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
        && let Ok(client) = http()
    {
        let _ = client
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

fn urlencoded_user_id(user_id: &str) -> String {
    urlencoding::encode(user_id).into_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncTrackMeta {
    pub id: String,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: Option<i64>,
    pub duration: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub added_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncPlaylist {
    pub id: String,
    pub name: String,
    pub cover: Option<String>,
    pub paths: Vec<String>,
    pub tracks: Vec<SyncTrackMeta>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<crate::commands::types::Settings>,
    #[serde(default)]
    pub order: Vec<String>,
    #[serde(default)]
    pub playlists: Vec<SyncPlaylist>,
    #[serde(default)]
    pub tracks: Vec<SyncTrackMeta>,
    #[serde(default)]
    pub missing: Vec<String>,
    #[serde(default)]
    pub taste: Vec<crate::services::taste::TasteEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncedSettings {
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub accent_color: String,
    #[serde(default)]
    pub dynamic_accent: bool,
    #[serde(default)]
    pub eq_enabled: bool,
    #[serde(default)]
    pub eq_bands: Vec<f32>,
    #[serde(default)]
    pub display_name: String,
}

impl From<&crate::commands::types::Settings> for SyncedSettings {
    fn from(value: &crate::commands::types::Settings) -> Self {
        Self {
            theme: value.theme.clone(),
            accent_color: value.accent_color.clone(),
            dynamic_accent: value.dynamic_accent,
            eq_enabled: value.eq_enabled,
            eq_bands: value.eq_bands.clone(),
            display_name: value.display_name.clone(),
        }
    }
}

fn apply_synced_settings(data_dir: &Path, synced: &SyncedSettings) -> Result<(), String> {
    let mut current = crate::services::settings::get_settings(data_dir);
    current.theme = synced.theme.clone();
    current.accent_color = synced.accent_color.clone();
    current.dynamic_accent = synced.dynamic_accent;
    current.eq_enabled = synced.eq_enabled;
    current.eq_bands = synced.eq_bands.clone();
    current.display_name = synced.display_name.clone();
    crate::services::settings::set_settings(data_dir, current)?;
    Ok(())
}

fn sync_track_meta(
    track: &crate::commands::types::LibraryTrack,
    source_url: Option<String>,
) -> SyncTrackMeta {
    SyncTrackMeta {
        id: track.id.clone(),
        path: track.path.clone(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        year: track.year,
        duration: (track.duration > 0.0).then_some(track.duration),
        cover: track.cover.clone(),
        source_url,
        added_at: track.added_at,
    }
}

fn stream_from_meta(meta: &SyncTrackMeta) -> CatalogueTrack {
    CatalogueTrack {
        id: meta.id.clone(),
        title: meta.title.clone(),
        artist: meta.artist.clone(),
        artists: vec![CatalogueArtistRef {
            name: meta.artist.clone(),
            id: None,
        }],
        album: if meta.album.is_empty() {
            None
        } else {
            Some(CatalogueAlbumRef {
                name: meta.album.clone(),
                id: None,
            })
        },
        duration: meta.duration.filter(|d| *d > 0.0),
        cover: meta.cover.clone(),
        explicit: false,
    }
}

fn sync_state_cache_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("sync_state.json")
}

fn write_sync_state_cache_rs(data_dir: &Path, state: &SyncState) {
    let _ = util::write_json(&sync_state_cache_path(data_dir), state);
}

pub fn read_sync_state_cache(data_dir: &Path) -> Option<SyncState> {
    let path = sync_state_cache_path(data_dir);
    let data = std::fs::read(path).ok()?;
    serde_json::from_slice(&data).ok()
}

fn deleted_playlists_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("sync_deleted.json")
}

fn read_deleted_playlists(data_dir: &Path) -> std::collections::HashMap<String, i64> {
    let path = deleted_playlists_path(data_dir);
    std::fs::read(&path)
        .ok()
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default()
}

fn write_deleted_playlists(data_dir: &Path, map: &std::collections::HashMap<String, i64>) {
    let _ = util::write_json(&deleted_playlists_path(data_dir), map);
}

static DELETED_LOCK: StdMutex<()> = StdMutex::new(());

fn deleted_lock() -> std::sync::MutexGuard<'static, ()> {
    DELETED_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn record_deleted_playlist(data_dir: &Path, id: &str) {
    let _guard = deleted_lock();
    let mut map = read_deleted_playlists(data_dir);
    map.insert(id.to_string(), util::now_ms());
    write_deleted_playlists(data_dir, &map);
}

fn forget_deleted_playlist(data_dir: &Path, id: &str) {
    let _guard = deleted_lock();
    let mut map = read_deleted_playlists(data_dir);
    if map.remove(id).is_some() {
        write_deleted_playlists(data_dir, &map);
    }
}

pub fn build_sync_state(
    data_dir: &Path,
    output_dir: &Path,
    settings: &crate::commands::types::Settings,
) -> SyncState {
    let (tracks, _) = crate::services::library::scan_cached_library(output_dir, data_dir);

    let mut url_by_file: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for entry in crate::services::history::ok_entries(data_dir) {
        url_by_file.entry(entry.file).or_insert(entry.url);
    }

    let track_meta: Vec<SyncTrackMeta> = tracks
        .iter()
        .map(|t| sync_track_meta(t, url_by_file.get(&t.path).cloned()))
        .collect();
    let by_path: std::collections::HashMap<&str, &SyncTrackMeta> =
        track_meta.iter().map(|t| (t.path.as_str(), t)).collect();

    let mut missing = std::collections::BTreeSet::new();
    let mut playlists_out = Vec::new();
    for meta in playlists::list_playlists(output_dir) {
        let Some(full) = playlists::get_playlist(output_dir, &meta.id) else {
            continue;
        };
        let mut playlist_tracks = full
            .paths
            .iter()
            .filter_map(|p| by_path.get(p.as_str()).map(|t| (*t).clone()))
            .collect::<Vec<_>>();
        let mut seen_tracks: std::collections::HashSet<String> =
            playlist_tracks.iter().map(|t| t.path.clone()).collect();
        for stream in &full.streams {
            let id = stream.id.trim();
            if id.is_empty() {
                continue;
            }
            let path = format!("ytm:{id}");
            if !seen_tracks.insert(path.clone()) {
                continue;
            }
            let source_url = url_by_file.get(&path).cloned();
            playlist_tracks.push(SyncTrackMeta {
                id: id.to_string(),
                path,
                title: stream.title.clone(),
                artist: stream.artist.clone(),
                album: stream
                    .album
                    .as_ref()
                    .map(|album| album.name.clone())
                    .unwrap_or_default(),
                year: None,
                duration: stream.duration.filter(|d| *d > 0.0),
                cover: stream.cover.clone(),
                source_url,
                added_at: None,
            });
        }
        for path in &full.paths {
            if !by_path.contains_key(path.as_str()) && !path.starts_with("ytm:") {
                missing.insert(path.clone());
            }
        }
        playlists_out.push(SyncPlaylist {
            id: meta.id.clone(),
            name: meta.name.clone(),
            cover: full.cover.clone(),
            paths: full.paths.clone(),
            tracks: playlist_tracks,
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            deleted_at: None,
        });
    }

    let active_ids: std::collections::HashSet<String> =
        playlists_out.iter().map(|p| p.id.clone()).collect();
    for (id, deleted_at) in read_deleted_playlists(data_dir) {
        if active_ids.contains(&id) {
            continue;
        }
        playlists_out.push(SyncPlaylist {
            id,
            name: String::new(),
            cover: None,
            paths: Vec::new(),
            tracks: Vec::new(),
            created_at: 0,
            updated_at: deleted_at,
            deleted_at: Some(deleted_at),
        });
    }

    SyncState {
        version: 1,
        settings: Some(settings.clone()),
        order: playlists::read_order(output_dir),
        playlists: playlists_out,
        tracks: track_meta,
        missing: missing.into_iter().collect(),
        taste: crate::services::taste::load(data_dir),
    }
}

pub async fn push_sync_state(
    data_dir: &Path,
    output_dir: &Path,
    settings: &crate::commands::types::Settings,
) -> Result<(), String> {
    let result = push_sync_state_inner(data_dir, output_dir, settings).await;
    match &result {
        Ok(()) => set_sync_result(data_dir, None),
        Err(error) => set_sync_result(data_dir, Some(error)),
    }
    result
}

async fn push_sync_state_inner(
    data_dir: &Path,
    output_dir: &Path,
    settings: &crate::commands::types::Settings,
) -> Result<(), String> {
    let session = valid_session(data_dir).await?;
    let state = build_sync_state(data_dir, output_dir, settings);
    write_sync_state_cache_rs(data_dir, &state);
    let data =
        serde_json::to_value(&state).map_err(|e| format!("Could not encode sync state: {e}"))?;
    let request = http()?
        .post(format!(
            "{}/rest/v1/sync_state?on_conflict=user_id",
            base_url()
        ))
        .header("Content-Type", "application/json")
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&[serde_json::json!({
            "user_id": session.user_id,
            "data": data,
            "updated_at": util::now_ms(),
        })]);
    let response = rest_headers(data_dir, request)
        .await?
        .send()
        .await
        .map_err(|e| format!("Could not reach Vynl Sync: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Sync push failed (HTTP {status}): {body}"));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaylistAction {
    Create,
    Update,
    Keep,
    Delete,
}

fn playlist_action(local: Option<&Playlist>, remote: &SyncPlaylist) -> PlaylistAction {
    match local {
        None => {
            if remote.deleted_at.is_some() {
                PlaylistAction::Keep
            } else {
                PlaylistAction::Create
            }
        }
        Some(local) => match remote.deleted_at {
            Some(deleted_at) if local.updated_at <= deleted_at => PlaylistAction::Delete,
            Some(_) => PlaylistAction::Keep,
            None if remote.updated_at > local.updated_at => PlaylistAction::Update,
            None => PlaylistAction::Keep,
        },
    }
}

fn validate_sync_state(state: &SyncState) -> Result<(), String> {
    if state.version == 0 || state.version > SUPPORTED_SYNC_VERSION {
        return Err(format!(
            "Unsupported cloud sync state version ({}).",
            state.version
        ));
    }
    let mut ids = std::collections::HashSet::new();
    for playlist in &state.playlists {
        if playlist.id.trim().is_empty() {
            return Err("Cloud sync state contains a playlist without an id.".into());
        }
        if !ids.insert(playlist.id.clone()) {
            return Err("Cloud sync state contains duplicate playlist ids.".into());
        }
    }
    Ok(())
}

fn remote_streams(
    remote: &SyncPlaylist,
    meta_by_path: &std::collections::HashMap<&str, &SyncTrackMeta>,
) -> Vec<CatalogueTrack> {
    let mut metas: std::collections::HashMap<String, &SyncTrackMeta> =
        std::collections::HashMap::new();
    for meta in &remote.tracks {
        if meta.path.starts_with("ytm:") && !meta.id.is_empty() {
            metas.entry(meta.id.clone()).or_insert(meta);
        }
    }
    for path in &remote.paths {
        if let Some(meta) = meta_by_path.get(path.as_str())
            && meta.path.starts_with("ytm:")
            && !meta.id.is_empty()
        {
            metas.entry(meta.id.clone()).or_insert(meta);
        }
    }
    metas.into_values().map(stream_from_meta).collect()
}

#[derive(Debug, Default)]
struct ApplyReport {
    created: usize,
    updated: usize,
    deleted: usize,
    errors: Vec<String>,
}

fn apply_sync_state(data_dir: &Path, output_dir: &Path, state: &SyncState) -> ApplyReport {
    let mut report = ApplyReport::default();
    let local_metas = playlists::list_playlists(output_dir);
    let mut local_ids: std::collections::HashSet<String> =
        local_metas.iter().map(|meta| meta.id.clone()).collect();
    let mut local_names: std::collections::HashSet<String> = local_metas
        .iter()
        .map(|meta| meta.name.trim().to_lowercase())
        .collect();
    let meta_by_path: std::collections::HashMap<&str, &SyncTrackMeta> = state
        .tracks
        .iter()
        .map(|track| (track.path.as_str(), track))
        .collect();

    for remote in &state.playlists {
        let local = if local_ids.contains(&remote.id) {
            playlists::get_playlist(output_dir, &remote.id)
        } else {
            let name_key = remote.name.trim().to_lowercase();
            local_metas
                .iter()
                .find(|meta| meta.name.trim().to_lowercase() == name_key)
                .and_then(|meta| playlists::get_playlist(output_dir, &meta.id))
        };
        match playlist_action(local.as_ref(), remote) {
            PlaylistAction::Delete => {
                match playlists::delete_playlist(output_dir, data_dir, &remote.id) {
                    Ok(_) => {
                        report.deleted += 1;
                        local_ids.remove(&remote.id);
                        if let Some(local) = &local {
                            local_names.remove(&local.name.trim().to_lowercase());
                        }
                    }
                    Err(error) => report.errors.push(format!("delete {}: {error}", remote.id)),
                }
            }
            PlaylistAction::Keep => {
                if let Some(local) = &local {
                    if remote.deleted_at.is_some()
                        && local.updated_at > remote.deleted_at.unwrap_or(0)
                    {
                        forget_deleted_playlist(data_dir, &local.id);
                    }
                    let streams = remote_streams(remote, &meta_by_path);
                    if !streams.is_empty()
                        && let Err(error) =
                            playlists::add_streams_to_playlist(output_dir, &local.id, &streams)
                    {
                        report
                            .errors
                            .push(format!("restore details for {}: {error}", local.id));
                    }
                }
            }
            PlaylistAction::Update => {
                let Some(local) = local else { continue };
                let mut merged = local.clone();
                let streams = remote_streams(remote, &meta_by_path);
                if !streams.is_empty() {
                    merged.streams = streams;
                }
                merged.name = remote.name.clone();
                merged.paths = remote.paths.clone();
                merged.cover = remote.cover.clone();
                merged.created_at = remote.created_at;
                merged.updated_at = remote.updated_at;
                match playlists::upsert_playlist(output_dir, &merged) {
                    Ok(_) => {
                        report.updated += 1;
                        local_names.remove(&local.name.trim().to_lowercase());
                        local_names.insert(remote.name.trim().to_lowercase());
                    }
                    Err(error) => report.errors.push(format!("update {}: {error}", remote.id)),
                }
            }
            PlaylistAction::Create => {
                let synced = Playlist {
                    id: remote.id.clone(),
                    name: remote.name.clone(),
                    paths: remote.paths.clone(),
                    streams: remote_streams(remote, &meta_by_path),
                    adds: Vec::new(),
                    removals: Vec::new(),
                    cover: remote.cover.clone(),
                    created_at: remote.created_at,
                    updated_at: remote.updated_at,
                };
                match playlists::upsert_playlist(output_dir, &synced) {
                    Ok(_) => {
                        report.created += 1;
                        local_ids.insert(remote.id.clone());
                        local_names.insert(remote.name.trim().to_lowercase());
                    }
                    Err(error) => report.errors.push(format!("create {}: {error}", remote.id)),
                }
            }
        }
    }

    if !state.order.is_empty()
        && let Err(error) = playlists::reorder_playlists(output_dir, &state.order)
    {
        report.errors.push(format!("order: {error}"));
    }
    if let Some(settings) = &state.settings
        && let Err(error) = apply_synced_settings(data_dir, &SyncedSettings::from(settings))
    {
        report.errors.push(format!("settings: {error}"));
    }
    if !state.taste.is_empty() {
        let mut merged: std::collections::HashMap<String, crate::services::taste::TasteEntry> =
            std::collections::HashMap::new();
        for entry in crate::services::taste::load(data_dir)
            .into_iter()
            .chain(state.taste.clone())
        {
            let id = entry.track.id.clone();
            match merged.get_mut(&id) {
                Some(existing) => {
                    existing.plays = existing.plays.max(entry.plays);
                    if entry.last_played > existing.last_played {
                        existing.last_played = entry.last_played;
                        existing.track = entry.track.clone();
                    }
                    if entry.liked {
                        existing.liked = true;
                        existing.liked_at = match (existing.liked_at, entry.liked_at) {
                            (Some(a), Some(b)) => Some(a.max(b)),
                            (Some(a), None) => Some(a),
                            (None, Some(b)) => Some(b),
                            (None, None) => None,
                        };
                    }
                }
                None => {
                    merged.insert(id, entry);
                }
            }
        }
        if let Err(error) =
            crate::services::taste::save(data_dir, &merged.into_values().collect::<Vec<_>>())
        {
            report.errors.push(format!("taste: {error}"));
        }
    }
    report
}

pub async fn pull_sync_state(data_dir: &Path, output_dir: &Path) -> Result<SyncState, String> {
    let session = valid_session(data_dir).await?;
    let uid = urlencoded_user_id(&session.user_id);
    let request = http()?
        .get(format!(
            "{}/rest/v1/sync_state?select=data,updated_at&user_id=eq.{uid}",
            base_url()
        ))
        .header("Accept", "application/json");
    let response = rest_headers(data_dir, request)
        .await?
        .send()
        .await
        .map_err(|e| format!("Could not reach Vynl Sync: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Sync pull failed (HTTP {status}): {body}"));
    }
    let body = response.text().await.unwrap_or_default();
    let list: Vec<serde_json::Value> =
        serde_json::from_str(&body).map_err(|e| format!("Could not read the sync state: {e}"))?;
    let row = list
        .into_iter()
        .next()
        .ok_or_else(|| "No cloud sync state found.".to_string())?;
    let data = row
        .get("data")
        .cloned()
        .ok_or_else(|| "Cloud sync state is empty.".to_string())?;
    let state: SyncState = serde_json::from_value(data)
        .map_err(|e| format!("That cloud sync state isn't readable: {e}"))?;

    validate_sync_state(&state)?;
    let report = apply_sync_state(data_dir, output_dir, &state);
    write_sync_state_cache_rs(data_dir, &state);
    if report.errors.is_empty() {
        clear_sync_error(data_dir);
    } else {
        for error in &report.errors {
            eprintln!("[Vynl sync] {error}");
        }
        record_sync_error(data_dir, &report.errors.join("; "));
    }

    Ok(state)
}
