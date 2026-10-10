use std::collections::HashSet;

use once_cell::sync::Lazy;

use crate::commands::types::{MatchSource, SearchCandidate, TrackMeta};

use super::clean_title;

const OFFICIAL_CHANNEL_BONUS: f64 = 12.0;

const CATALOG_SOURCE_BONUS: f64 = 10.0;

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

#[derive(PartialEq, Eq)]
enum MatchTier {
    Studio,
    Variant,
    NonMusic,
}

impl MatchTier {
    fn rank(self) -> u8 {
        match self {
            MatchTier::Studio => 0,
            MatchTier::Variant => 1,
            MatchTier::NonMusic => 2,
        }
    }
}

fn tier_of(info: &CandidateInfo) -> MatchTier {
    if NON_MUSIC_MATCHER.is_match(&info.norm_title) {
        MatchTier::NonMusic
    } else if REJECT_VARIANTS_MATCHER.is_match(&info.norm_title)
        || REJECT_VARIANTS_MATCHER.is_match(&info.norm_channel)
    {
        MatchTier::Variant
    } else {
        MatchTier::Studio
    }
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

    if candidate.source == MatchSource::YouTubeMusic {
        score += CATALOG_SOURCE_BONUS;
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

    let best_rank = relevant
        .iter()
        .map(|(info, _)| tier_of(info).rank())
        .min()
        .unwrap_or(MatchTier::Studio.rank());

    let pool: Vec<(CandidateInfo, SearchCandidate)> = relevant
        .into_iter()
        .filter(|(info, _)| tier_of(info).rank() == best_rank)
        .collect();

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
