use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use regex::Regex;

use crate::commands::types::*;

mod finalize;
mod fs;
mod matching;
mod media;
mod orchestrator;
mod paths;
mod pipeline;
mod process;
mod request;
mod resolve;
mod tagging;
mod text;
use fs::*;
pub use fs::{
    existing_songs, final_output_path, final_output_path_in, find_song_on_disk, get_done_track_ids,
    is_already_on_disk,
};
use media::*;
pub use orchestrator::download_collection;
use pipeline::run_pass;
use request::*;
pub use resolve::resolve_link;
use text::*;
pub(crate) mod search;

pub(crate) use matching::filter_and_rank_candidates;
pub use paths::{audio_format_ext, keep_native, render_pattern, sanitize};
pub use process::cancel_download;
pub use process::resolve_tool;
use process::{
    CANCELLED_DOWNLOAD, CHILDREN, describe_no_audio_format, fmt_dur, last_error,
    progress_from_line, watch_for_stall,
};
pub use search::find_matches;
pub(crate) use search::{cancel_find, find_cancelled};

const VERIFY_TOLERANCE: f64 = 0.10;

static PROGRESS_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"\[download\]\s+(\d+(?:\.\d+)?)%").unwrap());

static PAREN_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"(?i)\s*[\(\[].*?[\)\]]\s*").unwrap());

static FEAT_RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r"(?i)\s*[-ΓÇôΓÇö]\s*(feat\.|ft\.|featuring)\s+.*$").unwrap()
});

static MULTI_SPACE_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"\s{2,}").unwrap());

pub(super) static SHORT_LINK_RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r"https?://(spotify\.link|spotify\.app\.link)/").unwrap()
});

static LRC_TIME_RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(r"(?m)^\s*(?:\[\d{1,2}:\d{2}(?:[.:]\d{1,3})?\]\s*)+").unwrap()
});

static LRC_META_RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
    Regex::new(
        r"(?im)^\s*\[(ar|ti|al|au|by|offset|re|ve|length|created|tool|version|application):[^\]]*\]\s*",
    )
    .unwrap()
});

static LAST_ERROR_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"(?i)^ERROR:\s*(.+)$").unwrap());

static WHITESPACE_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"\s+").unwrap());

static TRAILING_DOT_SPACE_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"[. ]+$").unwrap());

static NON_ALNUM_RE: once_cell::sync::Lazy<Regex> =
    once_cell::sync::Lazy::new(|| Regex::new(r"[^a-z0-9]+").unwrap());

pub(super) const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

pub(super) static BUSY: AtomicBool = AtomicBool::new(false);
pub(super) static GENERATION: AtomicU64 = AtomicU64::new(0);
pub(super) static TEMP_KEY_GEN: AtomicU64 = AtomicU64::new(0);

pub(super) const DOWNLOAD_WORKERS: usize = 4;

pub(super) const DOWNLOAD_STALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);

pub(super) const SEARCH_BUDGET: std::time::Duration = std::time::Duration::from_secs(120);

pub(super) const COVER_PREFETCH_WORKERS: usize = 8;
pub(super) const MAX_COVER_BYTES: usize = 15 * 1024 * 1024;

pub(super) const STREAM_EXTS: &[&str] = &[
    "webm", "m4a", "mp4", "opus", "ogg", "oga", "aac", "mpga", "weba", "m4b",
];

pub(super) struct ProcessResult {
    pub(super) ok: bool,
    pub(super) message: String,
    pub(super) raw_path: Option<String>,
    pub(super) used_url: Option<String>,
}
