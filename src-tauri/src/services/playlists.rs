use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::commands::types::{Playlist, PlaylistMeta};
use crate::services::util;

crate::declare_file_mutex!();

fn playlists_dir(output_dir: &Path) -> std::path::PathBuf {
    output_dir.join("playlists")
}

fn playlist_file(output_dir: &Path, id: &str) -> std::path::PathBuf {
    playlists_dir(output_dir).join(format!("{}.json", id))
}

pub fn validate_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn read_playlist(output_dir: &Path, id: &str) -> Option<Playlist> {
    let mut pl: Playlist = util::read_json(&playlist_file(output_dir, id))?;
    if pl.name.trim().is_empty() {
        pl.name = "Untitled".into();
    }
    pl.paths.retain(|p| !p.is_empty());
    let now = util::now_ms();
    if pl.created_at == 0 {
        pl.created_at = now;
    }
    if pl.updated_at == 0 {
        pl.updated_at = pl.created_at;
    }
    Some(pl)
}

fn write_playlist(output_dir: &Path, pl: &Playlist) -> Result<(), String> {
    util::write_json(&playlist_file(output_dir, &pl.id), pl)
}

fn touch(pl: &mut Playlist) {
    pl.updated_at = util::now_ms();
}

pub fn list_playlists(output_dir: &Path) -> Vec<PlaylistMeta> {
    let _lock = file_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let dir = playlists_dir(output_dir);
    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".json") {
                    names.push(name.to_string());
                }
            }
        }
    }
    let mut out: Vec<PlaylistMeta> = Vec::new();
    for name in &names {
        let id = &name[..name.len() - 5];
        if let Some(pl) = read_playlist(output_dir, id) {
            out.push(PlaylistMeta {
                id: pl.id,
                name: pl.name,
                track_count: pl.paths.len() as i64,
                created_at: pl.created_at,
                updated_at: pl.updated_at,
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub fn get_playlist(output_dir: &Path, id: &str) -> Option<Playlist> {
    if !validate_id(id) {
        return None;
    }
    let _lock = file_mutex().lock().unwrap_or_else(|e| e.into_inner());
    read_playlist(output_dir, id)
}

fn to_base36(mut num: u64) -> String {
    const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if num == 0 {
        return "0".into();
    }
    let mut result = Vec::new();
    while num > 0 {
        result.push(CHARS[(num % 36) as usize] as char);
        num /= 36;
    }
    result.iter().rev().collect()
}

fn random_hex_8() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let nanos = now.as_nanos() as u64;
    let secs = now.as_secs();
    let pid = std::process::id();
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(nanos);
    hasher.write_u64(secs);
    hasher.write_u32(pid);
    let mixed = hasher.finish();
    format!("{:08x}", (mixed >> 16) as u32)
}

pub fn create_playlist(output_dir: &Path, name: &str) -> Result<Playlist, String> {
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let now = util::now_ms();
    let timestamp_part = to_base36((now / 1000) as u64);
    let random_part = random_hex_8();
    let id = format!("pl-{}-{}", timestamp_part, random_part);
    let pl = Playlist {
        id: id.clone(),
        name: if name.trim().is_empty() {
            "Untitled".into()
        } else {
            name.trim().into()
        },
        paths: Vec::new(),
        cover: None,
        created_at: now,
        updated_at: now,
    };
    write_playlist(output_dir, &pl)?;
    Ok(pl)
}

pub fn rename_playlist(output_dir: &Path, id: &str, name: &str) -> Result<Playlist, String> {
    if !validate_id(id) {
        return Err("invalid playlist id".into());
    }
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let mut pl = read_playlist(output_dir, id).ok_or("playlist not found")?;
    let new_name = if name.trim().is_empty() {
        pl.name.clone()
    } else {
        name.trim().into()
    };
    pl.name = new_name;
    touch(&mut pl);
    write_playlist(output_dir, &pl)?;
    Ok(pl)
}

pub fn delete_playlist(output_dir: &Path, id: &str) {
    if !validate_id(id) {
        return;
    }
    let _lock = file_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let path = playlist_file(output_dir, id);
    let _ = fs::remove_file(&path);
}

pub fn add_to_playlist(output_dir: &Path, id: &str, paths: &[String]) -> Result<Playlist, String> {
    if !validate_id(id) {
        return Err("invalid playlist id".into());
    }
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let mut pl = read_playlist(output_dir, id).ok_or("playlist not found")?;
    let existing: std::collections::HashSet<String> = pl.paths.iter().cloned().collect();
    for p in paths {
        if !existing.contains(p) {
            pl.paths.push(p.clone());
        }
    }
    touch(&mut pl);
    write_playlist(output_dir, &pl)?;
    Ok(pl)
}

pub fn remove_from_playlist(
    output_dir: &Path,
    id: &str,
    track_path: &str,
) -> Result<Playlist, String> {
    if !validate_id(id) {
        return Err("invalid playlist id".into());
    }
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let mut pl = read_playlist(output_dir, id).ok_or("playlist not found")?;
    pl.paths.retain(|p| p != track_path);
    touch(&mut pl);
    write_playlist(output_dir, &pl)?;
    Ok(pl)
}

pub fn move_in_playlist(
    output_dir: &Path,
    id: &str,
    from: usize,
    to: usize,
) -> Result<Playlist, String> {
    if !validate_id(id) {
        return Err("invalid playlist id".into());
    }
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let mut pl = read_playlist(output_dir, id).ok_or("playlist not found")?;
    if from >= pl.paths.len() {
        return Err("index out of bounds".into());
    }
    let item = pl.paths.remove(from);
    let clamped = to.min(pl.paths.len());
    pl.paths.insert(clamped, item);
    touch(&mut pl);
    write_playlist(output_dir, &pl)?;
    Ok(pl)
}

pub fn set_playlist_cover(
    output_dir: &Path,
    id: &str,
    cover: Option<String>,
) -> Result<Playlist, String> {
    if !validate_id(id) {
        return Err("invalid playlist id".into());
    }
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let mut pl = read_playlist(output_dir, id).ok_or("playlist not found")?;
    pl.cover = cover;
    touch(&mut pl);
    write_playlist(output_dir, &pl)?;
    Ok(pl)
}

pub fn validate_cover_ext(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "webp"
    )
}
