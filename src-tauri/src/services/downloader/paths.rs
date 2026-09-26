use std::path::{Path, PathBuf};

use crate::commands::types::{AudioFormat, TrackMeta};

pub(crate) fn tmp_dir(user_data_dir: &Path) -> PathBuf {
    let d = user_data_dir.join("tmp");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn audio_format_ext(_fmt: &AudioFormat) -> &'static str {
    "mp3"
}

pub fn sanitize(name: &str) -> String {
    let cleaned = name
        .chars()
        .map(|c| {
            if matches!(
                c,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '\0'..='\x1f'
            ) {
                ' '
            } else {
                c
            }
        })
        .collect::<String>();
    let cleaned = super::WHITESPACE_RE.replace_all(&cleaned, " ").to_string();
    let cleaned = super::TRAILING_DOT_SPACE_RE
        .replace_all(&cleaned, "")
        .to_string();
    let trimmed = cleaned.trim().to_string();
    let trimmed: String = trimmed
        .split_whitespace()
        .filter(|s| !matches!(*s, "." | ".."))
        .collect::<Vec<_>>()
        .join(" ");
    if trimmed.is_empty() {
        "track".to_string()
    } else {
        trimmed.chars().take(150).collect()
    }
}

pub fn render_pattern(pattern: &str, t: &TrackMeta) -> String {
    let track = t
        .track_number
        .map(|n| format!("{:02}", n))
        .unwrap_or_else(|| "00".to_string());
    let year = t.year.map(|y| y.to_string()).unwrap_or_default();
    let step1 = pattern
        .replace("{track}", "\x01TRACK\x02")
        .replace("{title}", "\x01TITLE\x02")
        .replace("{artist}", "\x01ARTIST\x02")
        .replace("{album}", "\x01ALBUM\x02")
        .replace("{year}", "\x01YEAR\x02");
    step1
        .replace("\x01TRACK\x02", &track)
        .replace("\x01TITLE\x02", &t.title)
        .replace("\x01ARTIST\x02", &t.artist)
        .replace("\x01ALBUM\x02", &t.album)
        .replace("\x01YEAR\x02", &year)
}
