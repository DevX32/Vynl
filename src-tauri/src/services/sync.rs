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
const ORDER_TABLE: &str = "library_order";
const SESSION_DIR: &str = "cloud";
const SESSION_FILE: &str = "session.json";
const PENDING_TOMBSTONES_FILE: &str = "playlist-tombstones.json";

const PENDING_TOMBSTONE_WARN: usize = 200;

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
    strict: bool,
) -> LocalResolution {
    if let Some(playlist) = playlists::get_playlist(output_dir, &row.local_id) {
        return LocalResolution::Present(playlist);
    }
    if strict {
        return LocalResolution::Missing;
    }
    let wanted = row.name.trim().to_lowercase();
    let target = row_signature(row);
    let mut best: Option<Playlist> = None;
    let mut has_consumed_compatible = false;
    for meta in playlists::list_playlists(output_dir) {
        if consumed.contains(&meta.id) {
            if let Some(playlist) = playlists::get_playlist(output_dir, &meta.id)
                && playlist.name.trim().eq_ignore_ascii_case(&wanted)
            {
                let local_signature = catalogue_signature(&playlist);
                if local_signature.is_empty()
                    || target.is_empty()
                    || local_signature.intersection(&target).next().is_some()
                {
                    has_consumed_compatible = true;
                }
            }
            continue;
        }
        let Some(playlist) = playlists::get_playlist(output_dir, &meta.id) else {
            continue;
        };
        if !playlist.name.trim().eq_ignore_ascii_case(&wanted) {
            continue;
        }
        let local_signature = catalogue_signature(&playlist);

        let compatible = if local_signature.is_empty() || target.is_empty() {
            true
        } else {
            local_signature.intersection(&target).next().is_some()
        };
        if !compatible {
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
        return if has_consumed_compatible {
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

#[derive(Default)]
struct LibraryMarkMaps {
    hash_to_path: std::collections::HashMap<String, String>,
    path_to_hash: std::collections::HashMap<String, String>,
}

impl LibraryMarkMaps {
    fn contains_hash(&self, candidate: &str) -> bool {
        candidate.len() == 64
            && candidate.chars().all(|c| c.is_ascii_hexdigit())
            && self.hash_to_path.contains_key(candidate)
    }
}

fn library_mark_maps(data_dir: &Path) -> LibraryMarkMaps {
    let mut maps = LibraryMarkMaps::default();
    if let Some(tracks) = crate::services::library::load_cache(data_dir) {
        for track in tracks {
            if track.id.is_empty() || track.path.is_empty() {
                continue;
            }
            maps.hash_to_path
                .insert(track.id.clone(), track.path.clone());
            maps.path_to_hash
                .insert(util::norm_path_key(&track.path), track.id);
        }
    }
    maps
}

fn translate_marks_inbound(row: &mut SyncRow, maps: &LibraryMarkMaps) {
    for mark in row.adds.iter_mut().chain(row.removals.iter_mut()) {
        if let Some(hash) = mark.track_id.strip_prefix("ytm:")
            && maps.contains_hash(hash)
            && let Some(path) = maps.hash_to_path.get(hash)
        {
            mark.track_id = path.clone();
        }
    }
}

fn normalized_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

struct FoldedRows {
    rows: Vec<SyncRow>,
    stale: Vec<SyncRow>,
}

fn signatures_compatible(
    a: &std::collections::BTreeSet<String>,
    b: &std::collections::BTreeSet<String>,
) -> bool {
    if a.is_empty() || b.is_empty() {
        return true;
    }
    a.intersection(b).next().is_some()
}

fn fold_equivalent_rows(rows: Vec<SyncRow>) -> FoldedRows {
    let mut groups: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    for (index, row) in rows.iter().enumerate() {
        if row.deleted_at.is_some() {
            continue;
        }
        let key = normalized_name(&row.name);
        if key.is_empty() {
            continue;
        }
        groups.entry(key).or_default().push(index);
    }

    let mut stale: Vec<SyncRow> = Vec::new();
    let mut folded: std::collections::HashMap<usize, SyncRow> = std::collections::HashMap::new();
    let mut shadowed: std::collections::HashSet<usize> = std::collections::HashSet::new();

    for indexes in groups.values() {
        if indexes.len() < 2 {
            continue;
        }
        let mut order = indexes.clone();
        order.sort_by(|&a, &b| {
            rows[a]
                .created_at
                .cmp(&rows[b].created_at)
                .then_with(|| rows[a].local_id.cmp(&rows[b].local_id))
        });
        let canonical = order[0];
        let canonical_signature = row_signature(&rows[canonical]);
        let mut members: Vec<usize> = Vec::new();
        for &index in order.iter().skip(1) {
            if signatures_compatible(&canonical_signature, &row_signature(&rows[index])) {
                members.push(index);
            }
        }
        if members.is_empty() {
            continue;
        }
        let mut synthetic = rows[canonical].clone();
        let mut tracks = synthetic.tracks.clone();
        let mut adds = synthetic.adds.clone();
        let mut removals = synthetic.removals.clone();
        let mut updated_at = synthetic.updated_at;
        for &index in &members {
            let row = &rows[index];
            updated_at = updated_at.max(row.updated_at);
            for track in &row.tracks {
                if tracks.iter().any(|existing| existing.id == track.id) {
                    continue;
                }
                tracks.push(track.clone());
            }
            adds = merge_marks(&adds, &row.adds, false);
            removals = merge_marks(&removals, &row.removals, true);
        }
        synthetic.tracks = tracks;
        synthetic.adds = adds;
        synthetic.removals = removals;
        synthetic.updated_at = updated_at;
        stale.extend(members.iter().map(|&index| rows[index].clone()));
        for &index in &members {
            shadowed.insert(index);
        }
        folded.insert(canonical, synthetic);
    }

    let mut out: Vec<SyncRow> = Vec::with_capacity(rows.len());
    for (index, row) in rows.into_iter().enumerate() {
        if shadowed.contains(&index) {
            continue;
        }
        if let Some(synthetic) = folded.remove(&index) {
            out.push(synthetic);
        } else {
            out.push(row);
        }
    }
    FoldedRows { rows: out, stale }
}

fn mark_map(marks: &[PlaylistMark]) -> std::collections::HashMap<String, i64> {
    let mut map = std::collections::HashMap::new();
    for mark in marks {
        let key = util::norm_path_key(&mark.track_id);
        map.entry(key)
            .and_modify(|at| {
                if mark.at > *at {
                    *at = mark.at;
                }
            })
            .or_insert(mark.at);
    }
    map
}

const ADD_MARKS_CAP: usize = 10_000;
const REMOVAL_MARKS_CAP: usize = 50_000;

fn merge_marks(a: &[PlaylistMark], b: &[PlaylistMark], is_removals: bool) -> Vec<PlaylistMark> {
    let cap = if is_removals {
        REMOVAL_MARKS_CAP
    } else {
        ADD_MARKS_CAP
    };
    let combined: Vec<PlaylistMark> = a.iter().chain(b.iter()).cloned().collect();
    let map = mark_map(&combined);
    let mut merged: Vec<PlaylistMark> = map
        .into_iter()
        .map(|(track_id, at)| PlaylistMark { track_id, at })
        .collect();
    merged.sort_by(|x, y| y.at.cmp(&x.at).then_with(|| x.track_id.cmp(&y.track_id)));
    merged.truncate(cap);
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
    let adds = merge_marks(&local.adds, &row.adds, false);
    let removals = merge_marks(&local.removals, &row.removals, true);
    let add_map = mark_map(&adds);
    let rem_map = mark_map(&removals);
    let visible = |key: &str| {
        let normalized = util::norm_path_key(key);
        let add = add_map.get(normalized.as_str()).copied().unwrap_or(0);
        let removal = rem_map.get(normalized.as_str()).copied().unwrap_or(0);
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
        let removal = rem_map.get(key.as_str()).copied().unwrap_or(0);
        let add = add_map.get(key.as_str()).copied().unwrap_or(0);
        if removal > add {
            continue;
        }
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
    pub failed: usize,
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

fn pending_tombstones_path(data_dir: &Path) -> PathBuf {
    data_dir.join(SESSION_DIR).join(PENDING_TOMBSTONES_FILE)
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PendingTombstone {
    id: String,
    name: String,
    created_at: i64,
    deleted_at: i64,
    #[serde(default)]
    signature: Vec<String>,
    #[serde(default)]
    user_id: String,
}

fn read_pending_tombstones(data_dir: &Path) -> Vec<PendingTombstone> {
    let path = pending_tombstones_path(data_dir);
    if let Some(parsed) = util::read_json::<Vec<PendingTombstone>>(&path) {
        return parsed;
    }
    util::read_json::<Vec<(String, i64)>>(&path)
        .unwrap_or_default()
        .into_iter()
        .map(|(id, deleted_at)| PendingTombstone {
            id,
            name: String::new(),
            created_at: 0,
            deleted_at,
            signature: Vec::new(),
            user_id: String::new(),
        })
        .collect()
}

fn tombstones_for_session(
    pending: &[PendingTombstone],
    session: &Session,
) -> Vec<PendingTombstone> {
    pending
        .iter()
        .filter(|entry| entry.user_id.is_empty() || entry.user_id == session.user_id)
        .cloned()
        .collect()
}

fn write_pending_tombstones(data_dir: &Path, entries: &[PendingTombstone]) {
    if let Err(error) = util::write_json(&pending_tombstones_path(data_dir), &entries) {
        eprintln!("[Vynl sync] could not persist playlist deletion marker: {error}");
    }
}

static PENDING_TOMBSTONES_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn with_pending_tombstones_lock<R>(f: impl FnOnce() -> R) -> R {
    let guard = PENDING_TOMBSTONES_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let result = f();
    drop(guard);
    result
}

fn record_pending_tombstone_impl(
    data_dir: &Path,
    id: &str,
    name: &str,
    created_at: i64,
    signature: &[String],
) -> i64 {
    let user_id = read_session(data_dir)
        .map(|s| s.user_id)
        .unwrap_or_default();
    with_pending_tombstones_lock(|| {
        let mut entries = read_pending_tombstones(data_dir);
        if let Some(existing) = entries.iter().find(|existing| existing.id == id) {
            return existing.deleted_at;
        }
        let at = now_ms();
        entries.push(PendingTombstone {
            id: id.to_string(),
            name: name.trim().to_string(),
            created_at,
            deleted_at: at,
            signature: signature.to_vec(),
            user_id,
        });
        if entries.len() > PENDING_TOMBSTONE_WARN {
            eprintln!(
                "Vynl sync: {} pending tombstone(s) wait to be delivered to the server; the delete queue is not being dropped but is unusually large",
                entries.len()
            );
        }
        write_pending_tombstones(data_dir, &entries);
        at
    })
}

async fn patch_remote_tombstone(
    data_dir: &Path,
    session: &Session,
    id: &str,
    deleted_at: i64,
) -> bool {
    if !crate::services::playlists::validate_id(id) {
        return false;
    }
    let Ok(client) = http() else {
        return false;
    };
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
        .json(&json!({ "deleted_at": deleted_at }));
    match rest_headers(data_dir, request).await {
        Ok(request) => request
            .send()
            .await
            .map(|response| response.status().is_success())
            .unwrap_or(false),
        Err(_) => false,
    }
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

static MARKS_SUPPORTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

fn marks_supported() -> bool {
    MARKS_SUPPORTED.load(std::sync::atomic::Ordering::Relaxed)
}

fn set_marks_supported(supported: bool) {
    MARKS_SUPPORTED.store(supported, std::sync::atomic::Ordering::Relaxed);
    if !supported {
        eprintln!(
            "[Vynl sync] remote schema rejected the marks columns; continuing without adds/removals this sync"
        );
    }
}

fn body_missing_marks(body: &str) -> bool {
    let lowered = body.to_lowercase();
    (lowered.contains("column") && (lowered.contains("adds") || lowered.contains("removals")))
        || lowered.contains("schema cache") && lowered.contains("playlists")
}

async fn fetch_rows_once(data_dir: &Path, query: &str) -> Result<reqwest::Response, String> {
    let request = http()?
        .get(format!("{}/rest/v1/{}?{}", base_url(), TABLE, query))
        .header("Accept", "application/json");
    rest_headers(data_dir, request)
        .await?
        .send()
        .await
        .map_err(|e| format!("Could not reach Vynl Sync: {e}"))
}

const PAGE_SIZE: i64 = 500;

fn page_query(select: &str, user_id: &str, offset: i64) -> String {
    format!(
        "select={select}&user_id=eq.{user_id}&order=created_at.asc,local_id.asc&limit={PAGE_SIZE}&offset={offset}"
    )
}

const NO_MARKS_SELECT: &str = "local_id,name,tracks,created_at,updated_at,deleted_at";
const MARKS_SELECT: &str = "local_id,name,tracks,created_at,updated_at,deleted_at,adds,removals";

async fn fetch_order(data_dir: &Path) -> Result<Option<serde_json::Value>, String> {
    let Some(session) = read_session(data_dir) else {
        return Err("Vynl Sync is not signed in.".into());
    };
    let uid = urlencoded_user_id(&session.user_id);
    let request = http()?
        .get(format!(
            "{}/rest/v1/{}?select=order_ids,updated_at&user_id=eq.{uid}",
            base_url(),
            ORDER_TABLE
        ))
        .header("Accept", "application/json");
    let response = rest_headers(data_dir, request)
        .await?
        .send()
        .await
        .map_err(|e| format!("Could not reach Vynl Sync: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Ok(None);
    }
    let body = response.text().await.unwrap_or_default();
    let list: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap_or_default();
    Ok(list.into_iter().next())
}

async fn push_order(data_dir: &Path, order: &[String], updated_at: i64) -> Result<(), String> {
    let Some(session) = read_session(data_dir) else {
        return Err("Vynl Sync is not signed in.".into());
    };
    let request = http()?
        .post(format!(
            "{}/rest/v1/{}?on_conflict=user_id",
            base_url(),
            ORDER_TABLE
        ))
        .header("Content-Type", "application/json")
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&[json!({
            "user_id": session.user_id,
            "order_ids": order,
            "updated_at": updated_at,
        })]);
    let response = rest_headers(data_dir, request)
        .await?
        .send()
        .await
        .map_err(|e| format!("Could not reach Vynl Sync: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Could not save the order ({}).", status.as_u16()));
    }
    Ok(())
}

fn select_star() -> &'static str {
    if marks_supported() {
        MARKS_SELECT
    } else {
        NO_MARKS_SELECT
    }
}

fn parse_rows_page(body: &str) -> (Vec<SyncRow>, usize, usize) {
    match serde_json::from_str::<Vec<serde_json::Value>>(body) {
        Ok(values) => {
            let total = values.len();
            let mut dropped = 0;
            let rows = values
                .into_iter()
                .filter_map(|value| match serde_json::from_value::<SyncRow>(value) {
                    Ok(row) => Some(row),
                    Err(error) => {
                        dropped += 1;
                        eprintln!("sync: skipping malformed row: {error}");
                        None
                    }
                })
                .collect();
            (rows, total, dropped)
        }
        Err(_) => (Vec::new(), 0, 0),
    }
}

fn urlencoded_user_id(user_id: &str) -> String {
    urlencoding::encode(user_id).into_owned()
}

async fn fetch_rows(data_dir: &Path) -> Result<Vec<SyncRow>, String> {
    let session =
        read_session(data_dir).ok_or_else(|| "Vynl Sync is not signed in.".to_string())?;
    let uid = urlencoded_user_id(&session.user_id);
    let mut out: Vec<SyncRow> = Vec::new();
    let mut offset: i64 = 0;
    loop {
        let select = select_star();
        let once = page_query(select, &uid, offset);
        let mut response = fetch_rows_once(data_dir, &once).await?;
        let mut status = response.status();
        let mut body = response.text().await.unwrap_or_default();
        if !status.is_success() && once.contains("adds") && body_missing_marks(&body) {
            set_marks_supported(false);
            let fallback = page_query(NO_MARKS_SELECT, &uid, offset);
            response = fetch_rows_once(data_dir, &fallback).await?;
            status = response.status();
            body = response.text().await.unwrap_or_default();
        }
        if !status.is_success() {
            return Err(format!(
                "Could not load synced playlists ({}).",
                status.as_u16()
            ));
        }
        let (rows, page, dropped) = parse_rows_page(&body);
        out.extend(rows);
        if dropped > 0 {
            eprintln!("sync: dropped {dropped} malformed row(s) on page at offset {offset}");
        }
        if page == 0 || (page as i64) < PAGE_SIZE {
            break;
        }
        offset += PAGE_SIZE;
    }
    Ok(out)
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

fn pushable_marks(marks: &[PlaylistMark], maps: &LibraryMarkMaps) -> Vec<PlaylistMark> {
    marks
        .iter()
        .filter_map(|mark| {
            if mark.at <= 0 {
                return None;
            }
            if mark.track_id.starts_with("ytm:") {
                return Some(mark.clone());
            }
            maps.path_to_hash
                .get(&util::norm_path_key(&mark.track_id))
                .map(|hash| PlaylistMark {
                    track_id: format!("ytm:{hash}"),
                    at: mark.at,
                })
        })
        .collect()
}

fn playlist_value(session: &Session, playlist: &Playlist, maps: &LibraryMarkMaps) -> Value {
    json!({
        "user_id": session.user_id,
        "local_id": playlist.id,
        "name": playlist.name,
        "tracks": playlist.streams.iter().map(track_dto).collect::<Vec<_>>(),
        "created_at": playlist.created_at.max(1),
        "updated_at": playlist.updated_at.max(1),
        "deleted_at": Value::Null,
        "adds": pushable_marks(&playlist.adds, maps),
        "removals": pushable_marks(&playlist.removals, maps),
    })
}

fn playlist_value_push(session: &Session, playlist: &Playlist, maps: &LibraryMarkMaps) -> Value {
    let mut value = playlist_value(session, playlist, maps);
    if let Some(object) = value.as_object_mut() {
        object.remove("deleted_at");
    }
    value
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
    let maps = library_mark_maps(data_dir);
    let _ = upsert_rows(data_dir, &[playlist_value_push(&session, playlist, &maps)]).await;
}

fn signature_hits(row_tracks: &[SyncTrack], signature: &[String]) -> usize {
    let wanted: std::collections::HashSet<String> = signature.iter().map(|s| norm_id(s)).collect();
    if wanted.is_empty() {
        return 0;
    }
    row_tracks
        .iter()
        .map(|track| norm_id(&track.id))
        .filter(|id| wanted.contains(id))
        .collect::<std::collections::HashSet<_>>()
        .len()
}

fn signature_matches(row_tracks: &[SyncTrack], signature: &[String]) -> bool {
    if signature.is_empty() || row_tracks.is_empty() {
        return false;
    }
    let wanted_len = signature
        .iter()
        .map(|s| norm_id(s))
        .collect::<std::collections::HashSet<_>>()
        .len();
    let row_len = row_tracks
        .iter()
        .map(|track| norm_id(&track.id))
        .collect::<std::collections::HashSet<_>>()
        .len();
    let overlap = signature_hits(row_tracks, signature);
    let wanted_needed = (wanted_len as f64 * 0.8).ceil() as usize;
    let row_needed = (row_len.min(wanted_len) as f64 * 0.8).ceil() as usize;
    overlap >= wanted_needed && overlap >= row_needed
}

fn norm_id(id: &str) -> String {
    util::norm_path_key(id)
}

pub(crate) fn record_pending_tombstone(
    data_dir: &Path,
    id: &str,
    name: String,
    created_at: i64,
    signature: &[String],
) -> i64 {
    if !crate::services::playlists::validate_id(id) {
        return now_ms();
    }
    record_pending_tombstone_impl(data_dir, id, &name, created_at, signature)
}

pub async fn tombstone_playlist_remote(data_dir: &Path, id: &str) {
    if !crate::services::playlists::validate_id(id) {
        return;
    }
    if !sync_enabled() {
        return;
    }
    let pending = read_pending_tombstones(data_dir);
    let session = read_session(data_dir);
    let Some(entry) = pending.iter().find(|entry| {
        entry.id == id
            && (entry.user_id.is_empty()
                || session.as_ref().is_some_and(|s| s.user_id == entry.user_id))
    }) else {
        return;
    };
    let at = entry.deleted_at;
    let name = entry.name.clone();
    let signature = entry.signature.clone();
    if read_session(data_dir).is_none() {
        eprintln!(
            "[Vynl sync] playlist deletion recorded locally; not signed in — will sync later"
        );
        return;
    }
    let session = match valid_session(data_dir).await {
        Ok(session) => session,
        Err(error) => {
            eprintln!("[Vynl sync] playlist deletion waiting for session: {error}");
            return;
        }
    };
    let own_row = patch_remote_tombstone(data_dir, &session, id, at).await;
    patch_equivalent_tombstones(data_dir, &session, &name, &signature, at).await;
    eprintln!(
        "[Vynl sync] playlist {name} deleted (own row: {})",
        if own_row {
            "tombstoned"
        } else {
            "failed — kept pending"
        },
    );
}

async fn patch_equivalent_tombstones(
    data_dir: &Path,
    session: &Session,
    name: &str,
    signature: &[String],
    deleted_at: i64,
) -> bool {
    if name.trim().is_empty() || signature.is_empty() {
        return true;
    }
    let wanted_name = name.trim().to_lowercase();
    let Ok(rows) = fetch_rows(data_dir).await else {
        return false;
    };
    for row in &rows {
        if row.deleted_at.is_some() {
            continue;
        }
        let same_name = !wanted_name.is_empty() && row.name.trim().to_lowercase() == wanted_name;
        let same_playlist =
            same_name && signature_matches(&row.tracks, signature) && row.created_at <= deleted_at;
        if same_playlist
            && !patch_remote_tombstone(data_dir, session, &row.local_id, deleted_at).await
        {
            return false;
        }
    }
    true
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
    set_marks_supported(true);
    let session = valid_session(data_dir).await?;
    let pending = tombstones_for_session(&read_pending_tombstones(data_dir), &session);
    let mut tombstone_succeeded: Vec<String> = Vec::with_capacity(pending.len());
    if !pending.is_empty() {
        for entry in &pending {
            if patch_remote_tombstone(data_dir, &session, &entry.id, entry.deleted_at).await
                && patch_equivalent_tombstones(
                    data_dir,
                    &session,
                    &entry.name,
                    &entry.signature,
                    entry.deleted_at,
                )
                .await
            {
                tombstone_succeeded.push(entry.id.clone());
            }
        }
    }
    let raw_rows = fetch_rows(data_dir).await?;
    let maps = library_mark_maps(data_dir);
    let folded = fold_equivalent_rows(raw_rows);
    let mut rows = folded.rows;
    for row in &mut rows {
        translate_marks_inbound(row, &maps);
    }
    let pending = tombstones_for_session(&read_pending_tombstones(data_dir), &session);
    if !pending.is_empty() {
        for row in &mut rows {
            if row.deleted_at.is_some() {
                continue;
            }
            let wanted = normalized_name(&row.name);
            let forced = pending
                .iter()
                .filter(|entry| {
                    if entry.id == row.local_id {
                        return true;
                    }
                    if entry.name.is_empty() || normalized_name(&entry.name) != wanted {
                        return false;
                    }
                    if row.created_at > entry.deleted_at {
                        return false;
                    }
                    signature_matches(&row.tracks, &entry.signature)
                })
                .map(|entry| entry.deleted_at)
                .max();
            if let Some(at) = forced {
                row.deleted_at = Some(at);
            }
        }
    }

    if !tombstone_succeeded.is_empty() {
        with_pending_tombstones_lock(|| {
            let mut entries = read_pending_tombstones(data_dir);
            entries.retain(|entry| !tombstone_succeeded.contains(&entry.id));
            write_pending_tombstones(data_dir, &entries);
        });
    }
    let local_order_updated = playlists::order_updated_at(output_dir);

    if let Ok(Some(row)) = fetch_order(data_dir).await {
        let remote_updated = row.get("updated_at").and_then(|v| v.as_i64()).unwrap_or(0);
        if remote_updated > playlists::order_updated_at(output_dir) {
            let ids: Vec<String> = row
                .get("order_ids")
                .and_then(|v| v.as_array())
                .map(|array| {
                    array
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            if !ids.is_empty() && playlists::order_updated_at(output_dir) == local_order_updated {
                let _ = playlists::write_order_at(output_dir, &ids, remote_updated);
            }
        }
    }

    let tombstone_undelivered = pending.len().saturating_sub(tombstone_succeeded.len());
    let mut result = SyncResult::default();
    result.failed += tombstone_undelivered;
    let mut known: std::collections::HashSet<String> = std::collections::HashSet::new();
    for row in &folded.stale {
        known.insert(row.local_id.clone());
    }
    if !folded.stale.is_empty() {
        for row in &folded.stale {
            if row.local_id == LIKED_PLAYLIST_ID {
                continue;
            }
            let signature: Vec<String> = row.tracks.iter().map(|track| track.id.clone()).collect();
            record_pending_tombstone(
                data_dir,
                &row.local_id,
                row.name.clone(),
                row.created_at,
                &signature,
            );
            let _ = playlists::delete_playlist(output_dir, data_dir, &row.local_id);
        }
    }
    if let Err(error) = ensure_hidden_liked(output_dir, data_dir) {
        eprintln!("sync: could not ensure Liked Songs playlist: {error}");
    }

    let mut consumed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for row in &rows {
        known.insert(row.local_id.clone());
        if row.local_id == LIKED_PLAYLIST_ID {
            continue;
        }
        if !crate::services::playlists::validate_id(&row.local_id) {
            eprintln!(
                "Vynl sync: skipping remote row with invalid local_id: {}",
                row.local_id
            );
            result.failed += 1;
            continue;
        }
        let local = match resolve_local(
            output_dir,
            data_dir,
            row,
            &mut consumed,
            row.deleted_at.is_some(),
        ) {
            LocalResolution::Present(playlist) => Some(playlist),
            LocalResolution::Duplicate => continue,
            LocalResolution::Missing => None,
        };
        match row.deleted_at {
            Some(deleted_at) => {
                if local.is_some() {
                    match playlists::delete_playlist_if_stale(
                        output_dir,
                        data_dir,
                        &row.local_id,
                        deleted_at,
                    ) {
                        Ok(true) => result.deleted += 1,
                        Ok(false) => {
                            if let Some(playlist) =
                                playlists::get_playlist(output_dir, &row.local_id)
                            {
                                if let Err(error) = upsert_rows(
                                    data_dir,
                                    &[playlist_value(&session, &playlist, &maps)],
                                )
                                .await
                                {
                                    eprintln!(
                                        "Vynl sync: could not save playlist {}: {error}",
                                        playlist.id
                                    );
                                    result.failed += 1;
                                } else {
                                    result.pushed += 1;
                                }
                            }
                        }
                        Err(error) => {
                            eprintln!(
                                "Vynl sync: could not delete playlist {}: {error}",
                                row.local_id
                            );
                            result.failed += 1;
                        }
                    }
                }
            }
            None => match local {
                None => {
                    let synced = row_to_playlist(row);
                    match playlists::upsert_playlist(output_dir, &synced) {
                        Ok(_) => result.pulled += 1,
                        Err(error) => {
                            eprintln!(
                                "Vynl sync: could not create playlist {}: {error}",
                                synced.id
                            );
                            result.failed += 1;
                        }
                    }
                }
                Some(_playlist) => {
                    match playlists::merge_playlist_with_remote(
                        output_dir,
                        &row.local_id,
                        |current| union_playlist(current, row),
                        |before, merged| merge_signature(before) != merge_signature(merged),
                    ) {
                        Ok(Some((before, merged, changed))) => {
                            if changed {
                                let remote_change =
                                    merged.paths.iter().any(|p| !before.paths.contains(p))
                                        || merged.removals.iter().any(|m| {
                                            before
                                                .removals
                                                .iter()
                                                .all(|r| r.track_id != m.track_id || r.at < m.at)
                                        });
                                if remote_change {
                                    result.pulled += 1;
                                } else {
                                    result.pushed += 1;
                                }
                                if let Err(error) = upsert_rows(
                                    data_dir,
                                    &[playlist_value_push(&session, &merged, &maps)],
                                )
                                .await
                                {
                                    eprintln!(
                                        "Vynl sync: could not save playlist {}: {error}",
                                        merged.id
                                    );
                                    result.failed += 1;
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            eprintln!(
                                "Vynl sync: could not merge playlist {}: {error}",
                                row.local_id
                            );
                            result.failed += 1;
                        }
                    }
                }
            },
        }
    }

    for meta in playlists::list_playlists(output_dir) {
        if known.contains(&meta.id) {
            continue;
        }
        if meta.id == LIKED_PLAYLIST_ID {
            continue;
        }
        if let Some(playlist) = playlists::get_playlist(output_dir, &meta.id) {
            if let Err(error) =
                upsert_rows(data_dir, &[playlist_value_push(&session, &playlist, &maps)]).await
            {
                eprintln!(
                    "Vynl sync: could not save playlist {}: {error}",
                    playlist.id
                );
                result.failed += 1;
            } else {
                result.pushed += 1;
            }
        }
    }

    let order = playlists::read_order(output_dir);
    if !order.is_empty() {
        let remote_at = fetch_order(data_dir)
            .await
            .ok()
            .flatten()
            .and_then(|row| row.get("updated_at").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        let local_at = playlists::order_updated_at(output_dir);
        if local_at > remote_at
            && let Err(error) = push_order(data_dir, &order, local_at).await
        {
            eprintln!("Vynl sync: could not save the playlist order: {error}");
            result.failed += 1;
        }
    }

    if let Some(liked) = playlists::get_playlist(output_dir, LIKED_PLAYLIST_ID)
        && let Err(error) = reconcile_taste(data_dir, &liked)
    {
        eprintln!("sync: could not reconcile taste: {error}");
        result.failed += 1;
    }

    Ok(result)
}
