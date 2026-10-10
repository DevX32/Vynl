use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::commands::types::{LibraryTrack, PlaylistMeta, Settings};
use crate::services::{history, library, playlists, util};

pub const BACKUP_VERSION: u32 = 1;
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    M3u,
}

impl ExportFormat {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "json" => Ok(ExportFormat::Json),
            "csv" => Ok(ExportFormat::Csv),
            "m3u" | "m3u8" => Ok(ExportFormat::M3u),
            other => Err(format!("Unsupported export format: {other}")),
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::M3u => "m3u",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Json => "Vynl backup (JSON)",
            ExportFormat::Csv => "Track list (CSV)",
            ExportFormat::M3u => "Playlist (M3U)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedTrack {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_number: Option<i64>,
    pub duration: f64,
    pub format: String,
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedPlaylist {
    pub id: String,
    pub name: String,
    pub track_count: usize,
    pub cover: Option<String>,
    pub paths: Vec<String>,
    pub tracks: Vec<ExportedTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryBackup {
    pub app: String,
    pub app_version: String,
    pub backup_version: u32,
    pub exported_at: i64,
    pub output_dir: String,
    pub track_count: usize,
    pub playlist_count: usize,
    pub settings: Settings,
    pub tracks: Vec<ExportedTrack>,
    pub playlists: Vec<ExportedPlaylist>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSummary {
    pub path: String,
    pub format: String,
    pub track_count: usize,
    pub playlist_count: usize,
    pub bytes: u64,
}

fn exported_track(track: &LibraryTrack, source_url: Option<String>) -> ExportedTrack {
    ExportedTrack {
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        year: track.year,
        track_number: track.track_number,
        duration: round_secs(track.duration),
        format: track.ext.clone(),
        file: track.path.clone(),
        source_url,
        added_at: track.added_at,
    }
}

fn round_secs(value: f64) -> f64 {
    if !value.is_finite() || value < 0.0 {
        return 0.0;
    }
    (value * 1000.0).round() / 1000.0
}

pub fn build_backup(
    settings: &Settings,
    user_data_dir: &Path,
    include_playlists: bool,
) -> LibraryBackup {
    let output_dir = PathBuf::from(&settings.output_dir);
    let (tracks, _) = library::scan_cached_library(&output_dir, user_data_dir);

    let mut url_by_file: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for entry in history::ok_entries(user_data_dir) {
        url_by_file.entry(entry.file).or_insert(entry.url);
    }

    let tracks = tracks
        .iter()
        .map(|t| {
            let url = url_by_file.get(&t.path).cloned();
            exported_track(t, url)
        })
        .collect::<Vec<_>>();

    let by_path: std::collections::HashMap<String, &ExportedTrack> =
        tracks.iter().map(|t| (t.file.clone(), t)).collect();

    let metas = if include_playlists {
        playlists::list_playlists(&output_dir)
    } else {
        Vec::new()
    };
    let playlists = metas
        .iter()
        .map(|meta: &PlaylistMeta| {
            let full = playlists::get_playlist(&output_dir, &meta.id);
            let paths = full.as_ref().map(|p| p.paths.clone()).unwrap_or_default();
            let playlist_tracks = paths
                .iter()
                .filter_map(|p| by_path.get(p).map(|t| (*t).clone()))
                .collect::<Vec<_>>();
            ExportedPlaylist {
                id: meta.id.clone(),
                name: meta.name.clone(),
                track_count: paths.len(),
                cover: full.as_ref().and_then(|p| p.cover.clone()),
                paths,
                tracks: playlist_tracks,
            }
        })
        .collect::<Vec<_>>();

    LibraryBackup {
        app: "vynl".to_string(),
        app_version: APP_VERSION.to_string(),
        backup_version: BACKUP_VERSION,
        exported_at: util::now_ms(),
        output_dir: settings.output_dir.clone(),
        track_count: tracks.len(),
        playlist_count: playlists.len(),
        settings: settings.clone(),
        tracks,
        playlists,
    }
}

pub fn suggested_filename(format: ExportFormat, backup: &LibraryBackup) -> String {
    let stamp = iso_stamp(backup.exported_at);
    format!("vynl-{stamp}.{}", format.extension())
}

fn iso_stamp(ms: i64) -> String {
    let secs = (ms / 1000).max(0) as u64;
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}{m:02}{d:02}-{hour:02}{minute:02}{second:02}")
}

pub fn render(backup: &LibraryBackup, format: ExportFormat) -> Result<String, String> {
    match format {
        ExportFormat::Json => serde_json::to_string_pretty(backup).map_err(|e| e.to_string()),
        ExportFormat::Csv => Ok(render_csv(backup)),
        ExportFormat::M3u => Ok(render_m3u(backup)),
    }
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn csv_opt<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

fn render_csv(backup: &LibraryBackup) -> String {
    let mut out = String::new();
    out.push_str("# Vynl library export\n");
    out.push_str(&format!("# app_version,{}\n", csv_field(&backup.app_version)));
    out.push_str(&format!("# output_dir,{}\n", csv_field(&backup.output_dir)));
    out.push_str(&format!(
        "# tracks,{},playlists,{}\n",
        backup.track_count, backup.playlist_count
    ));
    out.push_str("Title,Artist,Album,Year,Track,Duration,Format,Added,File\n");

    for track in &backup.tracks {
        out.push_str(&[
            csv_field(&track.title),
            csv_field(&track.artist),
            csv_field(&track.album),
            csv_opt(track.year),
            csv_opt(track.track_number),
            csv_opt(Some(track.duration.round() as i64)),
            csv_field(&track.format),
            csv_opt(track.added_at),
            csv_field(&track.file),
        ]
        .join(","));
        out.push('\n');
    }
    out
}

fn m3u_duration(secs: f64) -> i64 {
    if !secs.is_finite() || secs < 0.0 {
        return -1;
    }
    secs.round() as i64
}

fn m3u_section(out: &mut String, name: &str, tracks: &[&ExportedTrack]) {
    out.push_str("\n#PLAYLIST:");
    out.push_str(name);
    for track in tracks {
        out.push_str("\n#EXTINF:");
        out.push_str(&m3u_duration(track.duration).to_string());
        out.push(',');
        out.push_str(&track.artist);
        out.push_str(" - ");
        out.push_str(&track.title);
        out.push('\n');
        out.push_str(&track.file);
        out.push('\n');
    }
}

fn render_m3u(backup: &LibraryBackup) -> String {
    let mut out = String::from("#EXTM3U\n");
    let all = backup
        .tracks
        .iter()
        .map(|t| t as &ExportedTrack)
        .collect::<Vec<_>>();
    m3u_section(&mut out, "All Tracks", &all);

    for playlist in &backup.playlists {
        if playlist.tracks.is_empty() {
            continue;
        }
        let refs = playlist
            .tracks
            .iter()
            .map(|t| t as &ExportedTrack)
            .collect::<Vec<_>>();
        m3u_section(&mut out, &playlist.name, &refs);
    }
    out
}

pub fn write_backup(content: &str, path: &Path) -> Result<u64, String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create folder: {e}"))?;
    }
    std::fs::write(path, content).map_err(|e| format!("Failed to write backup: {e}"))?;
    Ok(content.len() as u64)
}

const MAX_BACKUP_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoredBackup {
    pub collection: crate::commands::types::Collection,
    pub matches: std::collections::HashMap<String, Vec<crate::commands::types::SearchCandidate>>,
    pub picks: std::collections::HashMap<String, i64>,
    pub total: usize,
    pub known_sources: usize,
}

pub fn parse_backup(content: &str) -> Result<LibraryBackup, String> {
    let backup: LibraryBackup = serde_json::from_str(content)
        .map_err(|e| format!("That file isn't a readable Vynl backup: {e}"))?;
    if backup.app != "vynl" {
        return Err("That file isn't a Vynl backup.".into());
    }
    if backup.backup_version > BACKUP_VERSION {
        return Err(format!(
            "That backup was made by a newer version of Vynl (v{}). Update Vynl first.",
            backup.app_version
        ));
    }
    Ok(backup)
}

pub fn read_backup_file(path: &Path) -> Result<LibraryBackup, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("Could not open the backup: {e}"))?;
    if meta.len() > MAX_BACKUP_BYTES {
        return Err("That backup file is too large to be a Vynl backup.".into());
    }
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Could not read the backup: {e}"))?;
    parse_backup(&content)
}

pub fn restore(backup: &LibraryBackup) -> RestoredBackup {
    use crate::commands::types::{
        Collection, CollectionKind, MatchSource, SearchCandidate, TrackMeta,
    };

    let mut matches = std::collections::HashMap::new();
    let mut picks = std::collections::HashMap::new();

    let tracks: Vec<TrackMeta> = backup
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let id = format!("restored-{i}");
            if let Some(url) = t.source_url.as_ref().filter(|u| !u.trim().is_empty()) {
                matches.insert(
                    id.clone(),
                    vec![SearchCandidate {
                        url: url.clone(),
                        title: t.title.clone(),
                        duration: (t.duration > 0.0).then_some(t.duration.round()),
                        channel: None,
                        source: MatchSource::YouTube,
                        view_count: None,
                        channel_verified: None,
                    }],
                );
                picks.insert(id.clone(), 0);
            }
            TrackMeta {
                id,
                title: t.title.clone(),
                artist: t.artist.clone(),
                album: t.album.clone(),
                year: t.year,
                duration: (t.duration > 0.0).then_some(t.duration),
                cover: None,
                track_number: t.track_number,
                url: t.source_url.clone().unwrap_or_default(),
            }
        })
        .collect();

    let known_sources = matches.len();
    let total = tracks.len();

    RestoredBackup {
        collection: Collection {
            kind: CollectionKind::Playlist,
            id: format!("restored-{}", backup.exported_at),
            title: String::new(),
            owner: None,
            cover: None,
            tracks,
        },
        matches,
        picks,
        total,
        known_sources,
    }
}