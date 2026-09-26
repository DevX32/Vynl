use std::collections::HashSet;

use crate::commands::types::{SearchCandidate, TrackMeta};

use super::clean_title;

const OFFICIAL_CHANNEL_BONUS: f64 = 12.0;

fn normalize_phrase_input(s: &str) -> String {
    let lowered = s.to_lowercase();
    let mut out = String::with_capacity(lowered.len() + 2);
    out.push(' ');
    let mut prev_was_space = true;
    for c in lowered.chars() {
        if c.is_alphanumeric() {
            out.push(c);
            prev_was_space = false;
        } else if !prev_was_space {
            out.push(' ');
            prev_was_space = true;
        }
    }
    out.push(' ');
    out
}

fn contains_phrase(haystack: &str, needle: &str) -> bool {
    let n = normalize_phrase_input(needle);
    let n = n.trim();
    if n.is_empty() {
        return false;
    }

    let hay = normalize_phrase_input(haystack);
    if n.contains(' ') {
        return hay.contains(&format!(" {} ", n));
    }
    hay.split(' ').any(|t| t == n)
}

const REJECT_VARIANTS: &[&str] = &[
    "remix",
    "remixed",
    "nightcore",
    "slowed",
    "reverb",
    "sped up",
    "speed up",
    "bass boosted",
    "8d",
    "karaoke",
    "instrumental",
    "acoustic",
    "unplugged",
    "live",
    "bootleg",
    "mashup",
    "medley",
    "radio edit",
    "extended mix",
    "cover by",
    "covered by",
    "performed by",
    "tribute",
    "parody",
    "fanmade",
    "fan made",
    "reupload",
];

const NON_MUSIC_TERMS: &[&str] = &[
    "interview",
    "interviews",
    "podcast",
    "ted talk",
    "tedx",
    "talk show",
    "keynote",
    "lecture",
    "seminar",
    "webinar",
    "panel discussion",
    "motivational",
    "audiobook",
    "audio book",
    "narration",
    "narrated",
    "storytime",
    "bedtime story",
    "asmr",
    "white noise",
    "brown noise",
    "pink noise",
    "rain sounds",
    "sleep sounds",
    "guided meditation",
    "meditation",
    "soundscape",
    "ambience",
    "gameplay",
    "game play",
    "walkthrough",
    "lets play",
    "full match",
    "highlights",
    "commentary",
    "explained",
    "summary of",
    "track by track",
    "behind the scenes",
    "bts",
    "making of",
    "album review",
    "review",
    "reacts",
    "reaction",
    "tutorial",
    "how to",
    "documentary",
    "lyrics explained",
    "first listen",
    "listening reaction",
    "song analysis",
    "full album analysis",
    "breakdown",
];

const VARIANT_PENALTY_TOKENS: &[&str] = &[
    "cover",
    "piano",
    "guitar",
    "violin",
    "drum",
    "drums",
    "bass",
    "orchestral",
    "orchestra",
    "symphonic",
    "teaser",
    "trailer",
    "preview",
    "snippet",
    "clean",
    "explicit",
];

const UPLOAD_NOISE_TOKENS: &[&str] = &[
    "official",
    "audio",
    "video",
    "music",
    "visualizer",
    "visualiser",
    "lyric",
    "lyrics",
    "letra",
    "high",
    "quality",
    "hq",
    "hd",
    "4k",
    "mv",
    "mvi",
    "full",
    "version",
    "stream",
    "clip",
    "track",
    "colors",
    "colour",
];

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "from", "with", "that", "this", "these", "those", "you", "your", "our",
    "their", "his", "her", "its", "into", "onto", "over", "under", "than", "then", "them", "they",
    "will", "was", "were", "been", "being", "are", "but", "not", "out", "off", "own", "too",
    "very", "just", "get", "got", "let",
];

fn tokenize(s: &str) -> HashSet<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 1 && !STOPWORDS.contains(t))
        .map(|t| t.to_string())
        .collect()
}

fn strip_upload_noise(tokens: &HashSet<String>) -> HashSet<String> {
    tokens
        .iter()
        .filter(|t| !UPLOAD_NOISE_TOKENS.contains(&t.as_str()))
        .cloned()
        .collect()
}

pub(crate) fn title_tokens(title: &str) -> HashSet<String> {
    let mut tokens = tokenize(&clean_title(title));
    tokens.retain(|t| !UPLOAD_NOISE_TOKENS.contains(&t.as_str()));
    tokens
}

fn has_reject_variant(text: &str) -> bool {
    REJECT_VARIANTS.iter().any(|v| contains_phrase(text, v))
}

fn is_non_music(text: &str) -> bool {
    NON_MUSIC_TERMS.iter().any(|v| contains_phrase(text, v))
}

fn is_rejected(text: &str) -> bool {
    has_reject_variant(text) || is_non_music(text)
}

fn is_official_music_channel(channel: &str) -> bool {
    let tokens = tokenize(channel);
    tokens.contains("topic") || tokens.contains("vevo")
}

fn variant_penalty(text: &str) -> f64 {
    VARIANT_PENALTY_TOKENS
        .iter()
        .filter(|v| contains_phrase(text, v))
        .count() as f64
        * -30.0
}

fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    intersection as f64 / union as f64
}

pub(crate) fn score_candidate(candidate: &SearchCandidate, track: &TrackMeta) -> f64 {
    let c_title = candidate.title.to_lowercase();
    let c_channel = candidate.channel.as_deref().unwrap_or("").to_lowercase();

    let track_title_tokens = title_tokens(&track.title);
    let track_artist_tokens = strip_upload_noise(&tokenize(&track.artist));
    let candidate_title_tokens = strip_upload_noise(&tokenize(&c_title));
    let candidate_channel_tokens = strip_upload_noise(&tokenize(&c_channel));

    let title_sim = jaccard_similarity(&track_title_tokens, &candidate_title_tokens);
    let artist_sim = jaccard_similarity(&track_artist_tokens, &candidate_title_tokens);
    let channel_artist_sim = jaccard_similarity(&track_artist_tokens, &candidate_channel_tokens);

    let mut score = 0.0;
    score += title_sim * 45.0;
    score += artist_sim * 30.0;
    score += channel_artist_sim * 15.0;

    if let (Some(expected), Some(actual)) = (track.duration, candidate.duration) {
        if expected > 0.0 && actual > 0.0 {
            let diff = (actual - expected).abs();
            let ratio = diff / expected;
            if ratio < 0.05 {
                score += 15.0;
            } else if ratio < 0.1 {
                score += 10.0;
            } else if ratio < 0.2 {
                score += 5.0;
            } else if ratio > 0.5 || diff > 120.0 {
                score -= 50.0;
            }
        }
    } else if let Some(c_dur) = candidate.duration {
        if c_dur > 600.0 {
            score -= 20.0;
        }
    }

    if candidate.channel_verified.unwrap_or(false) {
        score += 10.0;
    }

    if let Some(views) = candidate.view_count {
        if views > 1_000_000 {
            score += 5.0;
        } else if views > 100_000 {
            score += 3.0;
        } else if views > 10_000 {
            score += 1.0;
        }
    }

    if is_official_music_channel(&c_channel) {
        score += OFFICIAL_CHANNEL_BONUS;
    }

    score += variant_penalty(&c_title);
    score += variant_penalty(&c_channel) / 2.0;

    score
}

pub(crate) fn filter_and_rank_candidates(
    candidates: Vec<SearchCandidate>,
    track: &TrackMeta,
) -> Vec<SearchCandidate> {
    let expected_artist = strip_upload_noise(&tokenize(&track.artist));
    let expected_title = title_tokens(&track.title);

    let duration_ok = |c: &SearchCandidate| {
        if let (Some(dur), Some(c_dur)) = (track.duration, c.duration) {
            if dur > 0.0 && c_dur > 0.0 {
                let diff = (c_dur - dur).abs();
                let ratio = diff / dur;
                return !(ratio > 0.5 || diff > 120.0);
            }
            true
        } else {
            c.duration.map_or(true, |d| d <= 600.0)
        }
    };

    let relevant = |c: &SearchCandidate| {
        let c_title = strip_upload_noise(&tokenize(&c.title));
        let c_channel = strip_upload_noise(&tokenize(c.channel.as_deref().unwrap_or("")));

        let artist_hit = !expected_artist.is_empty()
            && (expected_artist.intersection(&c_title).count() > 0
                || expected_artist.intersection(&c_channel).count() > 0);
        let title_hit =
            !expected_title.is_empty() && expected_title.intersection(&c_title).count() > 0;

        (artist_hit || title_hit) && duration_ok(c)
    };

    let relevant: Vec<SearchCandidate> =
        candidates.iter().filter(|c| relevant(c)).cloned().collect();

    let clean: Vec<SearchCandidate> = relevant
        .iter()
        .filter(|c| {
            !is_rejected(&c.title) && !has_reject_variant(c.channel.as_deref().unwrap_or(""))
        })
        .cloned()
        .collect();
    let pool = if clean.is_empty() { relevant } else { clean };

    let mut scored: Vec<(f64, SearchCandidate)> = pool
        .into_iter()
        .map(|c| {
            let score = score_candidate(&c, track);
            (score, c)
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().map(|(_, c)| c).collect()
}
