use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::sync::Mutex;

static CACHE: Lazy<Mutex<HashMap<String, Option<String>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static IN_FLIGHT: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(6))
        .build()
        .unwrap_or_default()
});

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

pub async fn lookup(artist: &str, album: &str, title: &str) -> Option<String> {
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

    let result = fetch(artist, album, title).await;

    let mut cache = CACHE.lock().await;
    cache.insert(key.clone(), result.clone());
    drop(cache);
    IN_FLIGHT.lock().await.remove(&key);

    result
}

async fn fetch(artist: &str, album: &str, title: &str) -> Option<String> {
    let artist = artist.trim();
    let album = album.trim();
    let title = title.trim();

    if !artist.is_empty() && !album.is_empty() {
        let term = format!("{artist} {album}");
        if let Some(url) = pick_album(&search(&term, "album", 5).await, artist, album) {
            return Some(upsize(&url));
        }
    }

    let term = match (artist.is_empty(), title.is_empty()) {
        (true, _) => title.to_string(),
        (false, true) => artist.to_string(),
        _ => format!("{artist} {title}"),
    };
    if term.is_empty() {
        return None;
    }

    let results = search(&term, "song", 10).await;
    if let Some(url) = pick_song(&results, artist, album, title) {
        return Some(upsize(&url));
    }

    if !album.is_empty() && !title.is_empty() {
        let term = format!("{artist} {title} {album}");
        if let Some(url) = pick_song(&search(&term, "song", 10).await, artist, album, title) {
            return Some(upsize(&url));
        }
    }

    None
}

async fn search(term: &str, entity: &str, limit: usize) -> Vec<SearchResult> {
    let limit = limit.to_string();
    let Ok(resp) = CLIENT
        .get("https://itunes.apple.com/search")
        .query(&[
            ("term", term),
            ("media", "music"),
            ("entity", entity),
            ("limit", limit.as_str()),
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

fn pick_album(results: &[SearchResult], artist: &str, album: &str) -> Option<String> {
    let want_artist = norm(artist);
    let want_album = norm(album);

    let mut best: Option<(&SearchResult, i32)> = None;
    for result in results {
        if result.artwork_url_100.is_none() {
            continue;
        }
        let score = album_score(result, &want_artist, &want_album);
        match best {
            Some((_, current)) if score <= current => {}
            _ => best = Some((result, score)),
        }
    }

    best.filter(|(_, score)| *score >= 8)
        .and_then(|(result, _)| result.artwork_url_100.clone())
}

fn album_score(result: &SearchResult, want_artist: &str, want_album: &str) -> i32 {
    let mut score = 0;

    let result_artist = norm(result.artist_name.as_deref().unwrap_or(""));
    if !want_artist.is_empty() {
        if result_artist == want_artist {
            score += 6;
        } else if result_artist.contains(want_artist) || want_artist.contains(&result_artist) {
            score += 2;
        }
    }

    let result_album = norm(result.collection_name.as_deref().unwrap_or(""));
    if !want_album.is_empty() {
        if result_album == want_album {
            score += 6;
        } else if result_album.contains(want_album) || want_album.contains(&result_album) {
            score += 3;
        }
    }

    if result.collection_name.is_some() {
        score += 1;
    }

    score
}

fn pick_song(results: &[SearchResult], artist: &str, album: &str, title: &str) -> Option<String> {
    let want_artist = norm(artist);
    let want_album = norm(album);
    let want_title = norm(title);
    let want_base = base_title(title);

    let mut best: Option<(&SearchResult, i32)> = None;
    for result in results {
        if result.artwork_url_100.is_none() {
            continue;
        }

        let result_artist = norm(result.artist_name.as_deref().unwrap_or(""));
        let result_album = norm(result.collection_name.as_deref().unwrap_or(""));
        let result_title = norm(result.track_name.as_deref().unwrap_or(""));
        let result_base = base_title(result.track_name.as_deref().unwrap_or(""));

        let mut score = 0;

        if !want_artist.is_empty() {
            if result_artist == want_artist {
                score += 6;
            } else if result_artist.contains(&want_artist) || want_artist.contains(&result_artist) {
                score += 2;
            } else {
                score -= 4;
            }
        }

        if !want_album.is_empty() {
            if result_album == want_album {
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
            } else {
                score -= 6;
            }
        }

        if BAD_TITLE_WORDS
            .iter()
            .any(|word| result_title.contains(word))
        {
            score -= 12;
        }

        match best {
            Some((_, current)) if score <= current => {}
            _ => best = Some((result, score)),
        }
    }

    best.filter(|(_, score)| *score >= 6)
        .and_then(|(result, _)| result.artwork_url_100.clone())
}

fn norm(s: &str) -> String {
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

fn upsize(url: &str) -> String {
    let url = url.trim();
    let url = match url.strip_prefix("http://") {
        Some(rest) => format!("https://{rest}"),
        None => url.to_string(),
    };
    url.replace("100x100bb", "1024x1024bb")
}
