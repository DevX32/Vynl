use once_cell::sync::Lazy;
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::sync::Mutex;

use super::internal;

static CACHE: Lazy<Mutex<HashMap<String, Option<String>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static IN_FLIGHT: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(6))
        .build()
        .unwrap_or_default()
});

const MIN_SCORE: i32 = 9;
const ITUNES_STOREFRONTS: &[&str] = &["US", "IN", "GB"];
const DEEZER_SEARCH: &str = "https://api.deezer.com/search";

const BAD_TITLE_WORDS: &[&str] = &[
    "karaoke",
    "tribute",
    "made famous",
    "originally performed",
    "in the style of",
    "cover version",
    "instrumental",
];

#[derive(Deserialize)]
struct SearchResponse {
    #[serde(default)]
    results: Vec<SearchResult>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SearchResult {
    #[serde(default)]
    track_name: Option<String>,
    #[serde(default)]
    artist_name: Option<String>,
    #[serde(default)]
    collection_name: Option<String>,
    #[serde(default)]
    artwork_url_100: Option<String>,
    #[serde(default)]
    track_time_millis: Option<f64>,
}

#[derive(Deserialize, Default)]
struct DeezerSearchResponse {
    #[serde(default)]
    data: Vec<DeezerTrack>,
}

#[derive(Deserialize, Default)]
struct DeezerTrack {
    #[serde(default)]
    title: String,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    artist: DeezerArtist,
    #[serde(default)]
    album: DeezerAlbum,
}

#[derive(Deserialize, Default)]
struct DeezerArtist {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize, Default)]
struct DeezerAlbum {
    #[serde(default)]
    title: String,
    #[serde(default)]
    cover_xl: Option<String>,
    #[serde(default)]
    cover_big: Option<String>,
}

struct Candidate {
    title: String,
    artist: String,
    album: String,
    duration: Option<f64>,
    cover: Option<String>,
}

fn cache_key(artist: &str, album: &str, title: &str) -> String {
    format!("{}|{}|{}", norm(artist), norm(album), norm(title))
}

pub async fn cached(artist: &str, album: &str, title: &str) -> Option<Option<String>> {
    if artist.trim().is_empty() && album.trim().is_empty() && title.trim().is_empty() {
        return Some(None);
    }
    CACHE
        .lock()
        .await
        .get(&cache_key(artist, album, title))
        .cloned()
}

pub async fn lookup(artist: &str, album: &str, title: &str, duration: f64) -> Option<String> {
    if artist.trim().is_empty() && album.trim().is_empty() && title.trim().is_empty() {
        return None;
    }

    let key = cache_key(artist, album, title);

    {
        let cache = CACHE.lock().await;
        if let Some(hit) = cache.get(&key) {
            return hit.clone();
        }
    }

    {
        let mut in_flight = IN_FLIGHT.lock().await;
        if !in_flight.insert(key.clone()) {
            return None;
        }
    }

    let result = fetch(artist, album, title, duration).await;

    let mut cache = CACHE.lock().await;
    cache.insert(key.clone(), result.clone());
    drop(cache);
    IN_FLIGHT.lock().await.remove(&key);

    result
}

/// Cover lookup chain. Spotify has the largest catalog and matches tracks that
/// were downloaded through Vynl; Deezer and iTunes cover the gaps.
async fn fetch(artist: &str, album: &str, title: &str, duration: f64) -> Option<String> {
    let artist = repair_mojibake(artist.trim()).into_owned();
    let album = repair_mojibake(album.trim()).into_owned();
    let title = repair_mojibake(title.trim()).into_owned();

    if let Some(cover) = spotify(&artist, &album, &title, duration).await {
        return Some(cover);
    }

    if let Some(cover) = deezer(&artist, &album, &title, duration).await {
        return Some(cover);
    }

    itunes(&artist, &album, &title, duration).await
}

async fn spotify(artist: &str, album: &str, title: &str, duration: f64) -> Option<String> {
    for term in search_terms(artist, album, title) {
        let Ok(results) = internal::search_spotify(&term, 10, 0).await else {
            break;
        };

        let candidates: Vec<Candidate> = results
            .iter()
            .map(|result| Candidate {
                title: result.title.clone(),
                artist: result.artist.clone(),
                album: result.album.clone(),
                duration: result.duration,
                cover: result.cover.clone(),
            })
            .collect();

        if let Some(cover) = pick_cover(&candidates, artist, album, title, duration) {
            return Some(cover);
        }
    }

    None
}

async fn deezer(artist: &str, album: &str, title: &str, duration: f64) -> Option<String> {
    for term in search_terms(artist, album, title) {
        let Ok(resp) = CLIENT
            .get(DEEZER_SEARCH)
            .query(&[("q", term.as_str()), ("limit", "10")])
            .send()
            .await
        else {
            break;
        };

        if !resp.status().is_success() {
            break;
        }

        let Ok(parsed) = resp.json::<DeezerSearchResponse>().await else {
            continue;
        };

        let candidates: Vec<Candidate> = parsed
            .data
            .into_iter()
            .map(|track| {
                let cover = track
                    .album
                    .cover_xl
                    .or(track.album.cover_big)
                    .filter(|url| !url.is_empty());

                Candidate {
                    title: track.title,
                    artist: track.artist.name,
                    album: track.album.title,
                    duration: track.duration,
                    cover,
                }
            })
            .collect();

        if let Some(cover) = pick_cover(&candidates, artist, album, title, duration) {
            return Some(cover);
        }
    }

    None
}

async fn itunes(artist: &str, album: &str, title: &str, duration: f64) -> Option<String> {
    for (index, country) in ITUNES_STOREFRONTS.iter().enumerate() {
        if let Some(cover) =
            itunes_storefront(artist, album, title, duration, country, index == 0).await
        {
            return Some(cover);
        }
    }

    None
}

async fn itunes_storefront(
    artist: &str,
    album: &str,
    title: &str,
    duration: f64,
    country: &str,
    include_album: bool,
) -> Option<String> {
    if include_album && !artist.is_empty() && !album.is_empty() {
        let term = format!("{artist} {album}");
        let results = itunes_search(&term, "album", 5, country).await;
        if let Some(cover) = pick_cover(&itunes_candidates(results), artist, album, title, duration)
        {
            return Some(cover);
        }
    }

    let term = match (artist.is_empty(), title.is_empty()) {
        (true, _) => title.to_string(),
        (false, true) => artist.to_string(),
        _ => format!("{artist} {title}"),
    };

    if !term.is_empty() {
        let results = itunes_search(&term, "song", 10, country).await;
        if let Some(cover) = pick_cover(&itunes_candidates(results), artist, album, title, duration)
        {
            return Some(cover);
        }
    }

    if include_album && !album.is_empty() && !title.is_empty() {
        let term = format!("{artist} {title} {album}");
        let results = itunes_search(&term, "song", 10, country).await;
        if let Some(cover) = pick_cover(&itunes_candidates(results), artist, album, title, duration)
        {
            return Some(cover);
        }
    }

    None
}

async fn itunes_search(term: &str, entity: &str, limit: usize, country: &str) -> Vec<SearchResult> {
    let limit = limit.to_string();
    let Ok(resp) = CLIENT
        .get("https://itunes.apple.com/search")
        .query(&[
            ("term", term),
            ("media", "music"),
            ("entity", entity),
            ("limit", limit.as_str()),
            ("country", country),
        ])
        .send()
        .await
    else {
        return Vec::new();
    };

    let Ok(parsed) = resp.json::<SearchResponse>().await else {
        return Vec::new();
    };

    parsed.results
}

fn itunes_candidates(results: Vec<SearchResult>) -> Vec<Candidate> {
    results
        .into_iter()
        .map(|result| {
            let cover = result
                .artwork_url_100
                .as_deref()
                .and_then(https_url)
                .map(|url| upsize(&url));

            Candidate {
                title: result.track_name.unwrap_or_default(),
                artist: result.artist_name.unwrap_or_default(),
                album: result.collection_name.unwrap_or_default(),
                duration: result
                    .track_time_millis
                    .filter(|millis| *millis > 0.0)
                    .map(|millis| millis / 1000.0),
                cover,
            }
        })
        .collect()
}

fn search_terms(artist: &str, album: &str, title: &str) -> Vec<String> {
    let mut terms = Vec::with_capacity(3);

    if !artist.is_empty() && !album.is_empty() {
        terms.push(format!("{artist} {album}"));
    }

    let term = match (artist.is_empty(), title.is_empty()) {
        (true, _) => title.to_string(),
        (false, true) => String::new(),
        _ => format!("{artist} {title}"),
    };
    if !term.is_empty() {
        terms.push(term);
    }

    if !artist.is_empty() && !album.is_empty() && !title.is_empty() {
        let combined = format!("{artist} {title} {album}");
        if !terms.contains(&combined) {
            terms.push(combined);
        }
    }

    terms
}

fn pick_cover(
    candidates: &[Candidate],
    artist: &str,
    album: &str,
    title: &str,
    duration: f64,
) -> Option<String> {
    let mut best: Option<(&Candidate, i32)> = None;
    for candidate in candidates {
        if candidate.cover.is_none() {
            continue;
        }
        let score = score_candidate(candidate, artist, album, title, duration);
        match best {
            Some((_, current)) if score <= current => {}
            _ => best = Some((candidate, score)),
        }
    }

    best.filter(|(_, score)| *score >= MIN_SCORE)
        .and_then(|(candidate, _)| candidate.cover.as_deref().and_then(https_url))
}

fn score_candidate(
    candidate: &Candidate,
    artist: &str,
    album: &str,
    title: &str,
    duration: f64,
) -> i32 {
    let want_artist = norm(artist);
    let want_album = norm(album);
    let want_title = norm(title);
    let want_base = base_title(title);

    let result_artist = norm(&candidate.artist);
    let result_album = norm(&candidate.album);
    let result_title = norm(&candidate.title);
    let result_base = base_title(&candidate.title);

    let mut score = 0;
    let album_exact = !want_album.is_empty() && result_album == want_album;

    if !want_artist.is_empty() {
        if result_artist == want_artist {
            score += 6;
        } else if result_artist.contains(&want_artist) || want_artist.contains(&result_artist) {
            score += 2;
        } else {
            score -= 5;
        }
    }

    if !want_album.is_empty() {
        if album_exact {
            score += 3;
        } else if result_album.contains(&want_album) || want_album.contains(&result_album) {
            score += 1;
        }
    }

    if !want_title.is_empty() {
        if result_title == want_title || (!want_base.is_empty() && result_base == want_base) {
            score += 6;
        } else if result_title.contains(&want_title) || want_title.contains(&result_title) {
            score += 1;
        } else if !album_exact {
            score -= 6;
        }
    }

    if duration > 0.0
        && let Some(result_duration) = candidate.duration.filter(|value| *value > 0.0)
    {
        let diff = (result_duration - duration).abs();
        if diff <= 2.0 {
            score += 3;
        } else if diff <= 5.0 {
            score += 1;
        } else if diff > 20.0 {
            score -= 3;
        }
    }

    if BAD_TITLE_WORDS
        .iter()
        .any(|word| result_title.contains(word))
    {
        score -= 12;
    }

    score
}

fn norm(s: &str) -> String {
    let s = repair_mojibake(s);
    let mut out = String::with_capacity(s.len());
    let mut needs_space = false;

    for ch in s.chars() {
        if ch.is_alphanumeric() {
            if needs_space && !out.is_empty() {
                out.push(' ');
            }
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
            needs_space = false;
        } else {
            needs_space = true;
        }
    }

    out
}

fn base_title(s: &str) -> String {
    let s = repair_mojibake(s);
    let lower = s.to_lowercase();
    let mut cut = lower.len();

    for (index, ch) in lower.char_indices() {
        if ch == '(' || ch == '[' {
            cut = cut.min(index);
        }
    }

    if let Some((head, _)) = lower.split_once(" - ") {
        cut = cut.min(head.len());
    }

    norm(&lower[..cut])
}

/// Repairs the common UTF-8-read-as-Latin-1 tag corruption, e.g. "MarÃ­as"
/// (U+00C3 U+00AD) back to "Marías".
fn repair_mojibake(s: &str) -> Cow<'_, str> {
    let has_latin1 = s.chars().any(|ch| ('\u{80}'..='\u{ff}').contains(&ch));
    if !has_latin1 || s.chars().any(|ch| ch as u32 > 0xff) {
        return Cow::Borrowed(s);
    }

    let bytes: Vec<u8> = s.chars().map(|ch| ch as u8).collect();
    match String::from_utf8(bytes) {
        Ok(fixed) if fixed != s => Cow::Owned(fixed),
        _ => Cow::Borrowed(s),
    }
}

fn upsize(url: &str) -> String {
    url.replace("100x100bb", "1024x1024bb")
}

fn https_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    if let Some(rest) = url.strip_prefix("http://") {
        return Some(format!("https://{rest}"));
    }

    url.starts_with("https://").then(|| url.to_string())
}
