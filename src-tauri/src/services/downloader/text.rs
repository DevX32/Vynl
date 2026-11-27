use super::{FEAT_RE, MULTI_SPACE_RE, NON_ALNUM_RE, PAREN_RE, WHITESPACE_RE};

pub(super) fn normalize(s: &str) -> String {
    let replaced = s.to_lowercase().replace('&', " and ");
    let collapsed = NON_ALNUM_RE.replace_all(&replaced, " ");
    WHITESPACE_RE
        .replace_all(&collapsed, " ")
        .trim()
        .to_string()
}

pub(super) fn song_key(title: &str, artist: &str) -> String {
    format!("{}||{}", normalize(title), normalize(artist))
}

pub(super) const DURATION_TOLERANCE: f64 = 5.0;

pub(super) fn same_song(duration: f64, wanted: Option<f64>) -> bool {
    let want = wanted.unwrap_or(0.0);
    want <= 0.0 || duration <= 0.0 || (want - duration).abs() <= DURATION_TOLERANCE
}

pub(super) fn clean_title(s: &str) -> String {
    let no_parens = PAREN_RE.replace_all(s, " ");
    let no_feat = FEAT_RE.replace(&no_parens, "");
    let cleaned = MULTI_SPACE_RE.replace_all(&no_feat, " ");
    cleaned.trim().to_string()
}
