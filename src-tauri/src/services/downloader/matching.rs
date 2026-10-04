use std::collections::HashSet;

use once_cell::sync::Lazy;

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

struct PhraseMatcher {
    singles: Vec<String>,
    phrases: Vec<String>,
}

impl PhraseMatcher {
    fn new(src: &[&str]) -> Self {
        let mut singles = Vec::new();
        let mut phrases = Vec::new();
        for s in src {
            let n = normalize_phrase_input(s);
            let n = n.trim();
            if n.is_empty() {
                continue;
            }
            if n.contains(' ') {
                phrases.push(format!(" {} ", n));
            } else {
                singles.push(n.to_string());
            }
        }
        Self { singles, phrases }
    }

    fn is_match(&self, hay_norm: &str) -> bool {
        self.count(hay_norm) > 0
    }

    fn count(&self, hay_norm: &str) -> usize {
        self.phrases
            .iter()
            .filter(|p| hay_norm.contains(*p))
            .count()
            + self
                .singles
                .iter()
                .filter(|s| hay_norm.split(' ').any(|t| t == s.as_str()))
                .count()
    }
}

static REJECT_VARIANTS_MATCHER: Lazy<PhraseMatcher> =
    Lazy::new(|| PhraseMatcher::new(REJECT_VARIANTS));
static NON_MUSIC_MATCHER: Lazy<PhraseMatcher> = Lazy::new(|| PhraseMatcher::new(NON_MUSIC_TERMS));
static VARIANT_PENALTY_MATCHER: Lazy<PhraseMatcher> =
    Lazy::new(|| PhraseMatcher::new(VARIANT_PENALTY_TOKENS));

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

fn is_rejected_norm(norm: &str) -> bool {
    REJECT_VARIANTS_MATCHER.is_match(norm) || NON_MUSIC_MATCHER.is_match(norm)
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

struct CandidateInfo {
    title_tokens: HashSet<String>,
    channel_tokens: HashSet<String>,
    norm_title: String,
    norm_channel: String,
}

fn analyze_candidate(candidate: &SearchCandidate) -> CandidateInfo {
    let channel = candidate.channel.as_deref().unwrap_or("");
    CandidateInfo {
        title_tokens: strip_upload_noise(&tokenize(&candidate.title)),
        channel_tokens: strip_upload_noise(&tokenize(channel)),
        norm_title: normalize_phrase_input(&candidate.title),
        norm_channel: normalize_phrase_input(channel),
    }
}

fn score_candidate(
    candidate: &SearchCandidate,
    info: &CandidateInfo,
    track: &TrackMeta,
    track_title_tokens: &HashSet<String>,
    track_artist_tokens: &HashSet<String>,
) -> f64 {
    let candidate_title_tokens = &info.title_tokens;
    let candidate_channel_tokens = &info.channel_tokens;

    let title_sim = jaccard_similarity(track_title_tokens, candidate_title_tokens);
    let artist_sim = jaccard_similarity(track_artist_tokens, candidate_title_tokens);
    let channel_artist_sim = jaccard_similarity(track_artist_tokens, candidate_channel_tokens);

    let mut score = 0.0;
    score += title_sim * 45.0;
    score += artist_sim * 30.0;
    score += channel_artist_sim * 15.0;

    if track_artist_tokens.len() > 1 {
        let mut c_meta: HashSet<String> = candidate_title_tokens.clone();
        c_meta.extend(candidate_channel_tokens.iter().cloned());
        let covered = track_artist_tokens.intersection(&c_meta).count();
        if covered == 0 {
            score -= 80.0;
        } else if covered < track_artist_tokens.len() {
            score -= 30.0 * (track_artist_tokens.len() - covered) as f64
                / track_artist_tokens.len() as f64;
        }
    }

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
    } else if let Some(c_dur) = candidate.duration
        && c_dur > 600.0
    {
        score -= 20.0;
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

    if info.channel_tokens.contains("topic") || info.channel_tokens.contains("vevo") {
        score += OFFICIAL_CHANNEL_BONUS;
    }

    score += -30.0 * VARIANT_PENALTY_MATCHER.count(&info.norm_title) as f64;
    score += -30.0 * VARIANT_PENALTY_MATCHER.count(&info.norm_channel) as f64 / 2.0;

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
                return !(ratio > 0.2 || diff > 30.0);
            }
            true
        } else {
            c.duration.is_none_or(|d| d <= 600.0)
        }
    };

    let relevant = |info: &CandidateInfo, c: &SearchCandidate| {
        let artist_hit = !expected_artist.is_empty()
            && (expected_artist.intersection(&info.title_tokens).count() > 0
                || expected_artist.intersection(&info.channel_tokens).count() > 0);
        let title_hit = !expected_title.is_empty()
            && expected_title.intersection(&info.title_tokens).count() > 0;

        (artist_hit || title_hit) && duration_ok(c)
    };

    let relevant: Vec<(CandidateInfo, SearchCandidate)> = candidates
        .into_iter()
        .map(|c| (analyze_candidate(&c), c))
        .filter(|(info, c)| relevant(info, c))
        .collect();

    let (clean, rejected): (Vec<_>, Vec<_>) = relevant.into_iter().partition(|(info, _)| {
        !is_rejected_norm(&info.norm_title) && !REJECT_VARIANTS_MATCHER.is_match(&info.norm_channel)
    });
    let pool = if clean.is_empty() { rejected } else { clean };

    let mut scored: Vec<(f64, SearchCandidate)> = pool
        .into_iter()
        .map(|(info, c)| {
            let score = score_candidate(&c, &info, track, &expected_title, &expected_artist);
            (score, c)
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().map(|(_, c)| c).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(title: &str, artist: &str) -> TrackMeta {
        TrackMeta {
            id: "t1".into(),
            title: title.into(),
            artist: artist.into(),
            album: String::new(),
            year: None,
            duration: Some(200.0),
            cover: None,
            track_number: None,
            url: "https://example.com".into(),
        }
    }

    fn candidate(title: &str, channel: Option<&str>) -> SearchCandidate {
        SearchCandidate {
            url: "https://example.com/v".into(),
            title: title.into(),
            duration: Some(200.0),
            channel: channel.map(str::to_string),
            source: crate::commands::types::MatchSource::YouTube,
            view_count: None,
            channel_verified: None,
            upload_date: None,
        }
    }

    #[test]
    fn title_word_cover_is_not_penalised_when_the_track_is_called_cover() {
        let t = track("Cover", "The Band");
        let exact = candidate("The Band - Cover", Some("The Band Topic"));
        let other = candidate("The Band - Paper Roses", Some("The Band Topic"));

        let ranked = filter_and_rank_candidates(vec![other, exact], &t);
        assert_eq!(
            ranked.first().map(|c| c.title.as_str()),
            Some("The Band - Cover"),
            "the exact match must win even though 'cover' is a penalty token"
        );
    }

    #[test]
    fn cover_is_still_penalised_when_the_track_title_does_not_contain_it() {
        let t = track("Paper Roses", "The Band");
        let exact = candidate("The Band - Paper Roses", Some("The Band Topic"));
        let cover_version = candidate("The Band - Paper Roses (Cover)", Some("Someone Else"));

        let ranked = filter_and_rank_candidates(vec![cover_version, exact], &t);
        assert_eq!(
            ranked.first().map(|c| c.title.as_str()),
            Some("The Band - Paper Roses"),
            "a genuine cover version must still lose to the original"
        );
    }

    #[test]
    fn reject_phrase_in_the_track_title_is_not_treated_as_a_variant() {
        let t = track("Blue Monday (Speed Up)", "New Order");
        assert!(!is_rejected("New Order - Blue Monday (Speed Up)", &t.title));
    }

    #[test]
    fn reject_phrase_outside_the_track_title_still_rejects() {
        let t = track("Blue Monday", "New Order");
        assert!(is_rejected("New Order - Blue Monday (Speed Up)", &t.title));
    }

    #[test]
    fn speed_up_candidate_survives_for_a_track_titled_speed_up() {
        let t = track("Speed Up", "Artist");
        let exact = candidate("Artist - Speed Up", Some("Artist Topic"));
        let nightcore = candidate("Artist - Speed Up (Nightcore)", Some("Artist Topic"));

        let ranked = filter_and_rank_candidates(vec![nightcore, exact], &t);
        assert_eq!(
            ranked.first().map(|c| c.title.as_str()),
            Some("Artist - Speed Up")
        );
    }
}
