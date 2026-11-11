use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Mp3,
    Opus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrackStatus {
    Queued,
    Searching,
    Downloading,
    Processing,
    Done,
    Skipped,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CollectionKind {
    Track,
    Album,
    Playlist,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToolNameEnum {
    #[serde(rename = "yt-dlp")]
    YtDlp,
}

pub const ALL_TOOLS: [ToolNameEnum; 1] = [ToolNameEnum::YtDlp];

impl ToolNameEnum {
    pub fn as_str(self) -> &'static str {
        match self {
            ToolNameEnum::YtDlp => "yt-dlp",
        }
    }

    pub fn version_args(self) -> &'static [&'static str] {
        match self {
            ToolNameEnum::YtDlp => &["--version"],
        }
    }
}

impl std::str::FromStr for ToolNameEnum {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "yt-dlp" => Ok(ToolNameEnum::YtDlp),
            other => Err(format!("Unknown tool: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToolState {
    Ok,
    Missing,
    Downloading,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LyricsKind {
    Lrc,
    Txt,
    Ttml,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LyricsSource {
    Local,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackMeta {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: Option<i64>,
    pub duration: Option<f64>,
    pub cover: Option<String>,
    pub track_number: Option<i64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub kind: CollectionKind,
    pub id: String,
    pub title: String,
    pub owner: Option<String>,
    pub cover: Option<String>,
    pub tracks: Vec<TrackMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub output_dir: String,
    pub format: AudioFormat,
    pub bitrate: Option<i64>,
    pub filename_pattern: String,
    pub overwrite: bool,
    pub discord_rpc: bool,
    pub confirm_matches: bool,
    pub minimize_to_tray: bool,
    pub accent_color: String,
    #[serde(default)]
    pub dynamic_accent: bool,
    #[serde(default = "default_true")]
    pub hardware_acceleration: bool,
    #[serde(default)]
    pub launch_at_startup: bool,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub eq_enabled: bool,
    #[serde(default = "default_eq_bands")]
    pub eq_bands: Vec<f32>,
    #[serde(default = "default_true")]
    pub plugins_auto_update: bool,
}

fn default_true() -> bool {
    true
}

fn default_eq_bands() -> Vec<f32> {
    vec![0.0; crate::services::player::EQ_BAND_COUNT]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MatchSource {
    YouTube,
    YouTubeMusic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchCandidate {
    pub url: String,
    pub title: String,
    pub duration: Option<f64>,
    pub channel: Option<String>,
    pub source: MatchSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_verified: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub ts: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub file: String,
    pub url: String,
    pub ok: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcPresence {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub time: f64,
    pub playing: bool,
    pub cover: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NowPlayingState {
    pub id: String,
    pub path: String,
    pub time: f64,
    pub queue: Vec<String>,
    #[serde(default)]
    pub user_queue: Vec<String>,
    #[serde(default)]
    pub context_queue: Vec<String>,
    #[serde(default)]
    pub context_index: i64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub name: ToolNameEnum,
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub state: ToolState,
    pub progress: Option<f64>,
    pub error: Option<String>,
    pub update_available: Option<bool>,
}

impl ToolStatus {
    pub fn new(name: ToolNameEnum, state: ToolState) -> Self {
        Self {
            name,
            installed: false,
            path: None,
            version: None,
            state,
            progress: None,
            error: None,
            update_available: None,
        }
    }

    pub fn downloading(name: ToolNameEnum, progress: Option<f64>) -> Self {
        Self::new(name, ToolState::Downloading).with_progress(progress)
    }

    pub fn with_progress(mut self, progress: Option<f64>) -> Self {
        self.progress = progress;
        self
    }

    pub fn found(mut self, path: String, version: Option<String>) -> Self {
        self.installed = true;
        self.path = Some(path);
        self.state = match version {
            Some(_) => ToolState::Ok,
            None => {
                self.error = Some("binary found but failed to run".into());
                ToolState::Error
            }
        };
        self.version = version;
        self
    }

    pub fn failed(name: ToolNameEnum, error: impl Into<String>) -> Self {
        let mut status = Self::new(name, ToolState::Error);
        status.error = Some(error.into());
        status
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryTrack {
    pub id: String,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: Option<i64>,
    pub duration: f64,
    pub cover: Option<String>,
    pub track_number: Option<i64>,
    pub lyrics: Option<String>,
    pub ext: String,
    pub mtime: Option<u64>,
    #[serde(default)]
    pub added_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistMeta {
    pub id: String,
    pub name: String,
    pub track_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub paths: Vec<String>,
    #[serde(default)]
    pub streams: Vec<CatalogueTrack>,
    pub cover: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaUpdate {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub cover: Option<String>,
    pub playing: bool,
    pub position: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum MediaControlAction {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    Stop,
    Seek { position: f64 },
    SeekBy { offset: f64 },
    SetVolume { volume: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackProgress {
    pub track_id: String,
    pub status: TrackStatus,
    pub percent: f64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSummary {
    pub total: i64,
    pub done: i64,
    pub skipped: i64,
    pub failed: i64,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsResult {
    pub kind: LyricsKind,
    pub text: String,
    pub source: LyricsSource,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsLookup {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
}

fn string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct StringOrNumber;

    impl<'de> Visitor<'de> for StringOrNumber {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or number")
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<String, E> {
            Ok(value.to_string())
        }

        fn visit_string<E: de::Error>(self, value: String) -> Result<String, E> {
            Ok(value)
        }

        fn visit_i64<E: de::Error>(self, value: i64) -> Result<String, E> {
            Ok(value.to_string())
        }

        fn visit_u64<E: de::Error>(self, value: u64) -> Result<String, E> {
            Ok(value.to_string())
        }

        fn visit_f64<E: de::Error>(self, value: f64) -> Result<String, E> {
            Ok(value.to_string())
        }
    }

    deserializer.deserialize_any(StringOrNumber)
}

fn string_or_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LrcSearchResult {
    #[serde(deserialize_with = "string_or_number")]
    pub id: String,
    #[serde(rename = "trackName", default, deserialize_with = "string_or_default")]
    pub track_name: String,
    #[serde(rename = "artistName", default, deserialize_with = "string_or_default")]
    pub artist_name: String,
    #[serde(rename = "albumName", default, deserialize_with = "string_or_default")]
    pub album_name: String,
    pub duration: f64,
    pub instrumental: bool,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::LrcSearchResult;

    #[test]
    fn lrc_search_result_accepts_numeric_id() {
        let parsed: LrcSearchResult = serde_json::from_str(
            r#"{"id":123456,"trackName":"Song","artistName":"Artist","albumName":null,"duration":12.5,"instrumental":false,"plainLyrics":null,"syncedLyrics":"[00:01.00]hi"}"#,
        )
        .expect("numeric LRCLIB id should deserialize");

        assert_eq!(parsed.id, "123456");
        assert_eq!(parsed.track_name, "Song");
        assert_eq!(parsed.artist_name, "Artist");
        assert_eq!(parsed.album_name, "");
        assert_eq!(parsed.synced_lyrics.as_deref(), Some("[00:01.00]hi"));
    }

    #[test]
    fn lrc_search_result_accepts_string_id_and_missing_names() {
        let parsed: LrcSearchResult = serde_json::from_str(
            r#"{"id":"USRC17607839","duration":0.0,"instrumental":false,"plainLyrics":"hi","syncedLyrics":null}"#,
        )
        .expect("ISRC string id should deserialize");

        assert_eq!(parsed.id, "USRC17607839");
        assert_eq!(parsed.track_name, "");
        assert_eq!(parsed.artist_name, "");
        assert_eq!(parsed.album_name, "");
        assert_eq!(parsed.plain_lyrics.as_deref(), Some("hi"));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpotifySearchResult {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: Option<f64>,
    pub cover: Option<String>,
    pub url: String,
}

pub use vynl_catalogue::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueStreamInfo {
    pub video_id: String,
    pub duration: Option<f64>,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueAddResult {
    pub path: String,
    pub downloaded: bool,
    pub playlist: Option<Playlist>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadOpts {
    pub matches: Option<std::collections::HashMap<String, Vec<SearchCandidate>>>,
    pub picks: Option<std::collections::HashMap<String, i64>>,
    pub skip_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DownloadEvent {
    Track { payload: TrackProgress },
    Summary { payload: DownloadSummary },
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub available: bool,
    pub version: Option<String>,
    pub current_version: String,
    pub downloading: Option<bool>,
    pub progress: Option<f64>,
    pub ready: Option<bool>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginInstallationMethod {
    Store,
    Dev,
    Sideload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEntry {
    pub id: String,
    pub version: String,
    pub path: String,
    pub installation_method: PluginInstallationMethod,
    pub original_path: Option<String>,
    pub enabled: bool,
    pub installed_at: i64,
    pub last_updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_repo: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifestVynl {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub categories: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default, rename = "vynl")]
    pub vynl: Option<PluginManifestVynl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorePlugin {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub added_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginStoreResult {
    pub entries: Vec<StorePlugin>,
    pub source_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHttpOptions {
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub headers: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHttpResponse {
    pub status: u16,
    pub ok: bool,
    pub headers: std::collections::HashMap<String, String>,
    pub text: String,
}
