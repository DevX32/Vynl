

use std::sync::OnceLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const LRCLIB_BASE: &str = "https://lrclib.net";
const LRCRED_BASE: &str = "https://lrc.red";
const BETTERLYRICS_BASE: &str = "https://lyrics-api.boidu.dev";

const DURATION_EXACT_SECS: f64 = 2.0;
const DURATION_TOLERANCE_SECS: f64 = 5.0;
const DURATION_HARD_REJECT_SECS: f64 = 8.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsLookup {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsResult {
    pub kind: String,
    pub text: String,
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("Vynl/1.0 (https://github.com/DevX32/Vynl)")
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

pub fn is_lrc(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        if !line.starts_with('[') {
            return false;
        }
        let Some(rest) = line[1..].split(']').next() else {
            return false;
        };
        let rest = rest.trim();
        let mut parts = rest.split(':');
        let Some(minutes) = parts.next() else {
            return false;
        };
        let Some(seconds) = parts.next() else {
            return false;
        };
        if !minutes.trim().chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        let seconds = seconds.split(['.', ',']).next().unwrap_or("");
        !seconds.is_empty() && seconds.chars().all(|c| c.is_ascii_digit())
    })
}

pub fn is_ttml(text: &str) -> bool {
    let bytes = text.as_bytes();
    for index in 0..bytes.len().saturating_sub(3) {
        if bytes[index] != b'<' {
            continue;
        }
        let (b1, b2, b3) = (bytes[index + 1], bytes[index + 2], bytes[index + 3]);
        if !b1.eq_ignore_ascii_case(&b't') || !b2.eq_ignore_ascii_case(&b't') {
            continue;
        }
        if matches!(b3, b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/') {
            return true;
        }
        if b3.eq_ignore_ascii_case(&b'm') || b3.eq_ignore_ascii_case(&b'l') {
            return true;
        }
    }
    false
}

pub fn kind_of(text: &str) -> String {
    if is_ttml(text) {
        "ttml".to_string()
    } else if is_lrc(text) {
        "lrc".to_string()
    } else {
        "txt".to_string()
    }
}

fn is_synced(kind: &str) -> bool {
    kind == "lrc" || kind == "ttml"
}

struct Hit {
    title: String,
    artist: String,
    duration: Option<f64>,
}

fn lrclib_hit(data: &serde_json::Value) -> Hit {
    Hit {
        title: data["trackName"].as_str().unwrap_or("").to_string(),
        artist: data["artistName"].as_str().unwrap_or("").to_string(),
        duration: item_duration(data),
    }
}

fn lrcred_hit(data: &serde_json::Value) -> Hit {
    Hit {
        title: data["title"].as_str().unwrap_or("").to_string(),
        artist: data["artist"].as_str().unwrap_or("").to_string(),
        duration: item_duration(data),
    }
}

fn item_duration(data: &serde_json::Value) -> Option<f64> {
    data.get("duration")
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn duration_acceptable(data: &serde_json::Value, lookup: &LyricsLookup) -> bool {
    if lookup.duration <= 0.0 {
        return true;
    }
    match item_duration(data) {
        Some(candidate) => (candidate - lookup.duration).abs() <= DURATION_HARD_REJECT_SECS,
        None => true,
    }
}

fn score_hit(hit: &Hit, lookup: &LyricsLookup) -> i32 {
    let title_lower = hit.title.to_lowercase();
    let artist_lower = hit.artist.to_lowercase();
    let lookup_title = lookup.title.to_lowercase();
    let lookup_artist = lookup.artist.to_lowercase();

    let title_match = if title_lower == lookup_title {
        10
    } else if title_lower.contains(&lookup_title) || lookup_title.contains(&title_lower) {
        5
    } else {
        0
    };

    let artist_match = if artist_lower == lookup_artist {
        10
    } else if artist_lower.contains(&lookup_artist) || lookup_artist.contains(&artist_lower) {
        5
    } else {
        0
    };

    let duration_match = match (hit.duration, lookup.duration > 0.0) {
        (Some(candidate), true) => {
            let difference = (candidate - lookup.duration).abs();
            if difference <= DURATION_EXACT_SECS {
                10
            } else if difference <= DURATION_TOLERANCE_SECS {
                5
            } else {
                -20
            }
        }
        _ => 0,
    };

    title_match + artist_match + duration_match
}

fn parse_lrclib(data: Option<&serde_json::Value>) -> Option<LyricsResult> {
    let data = data?;
    let synced = data["syncedLyrics"]
        .as_str()
        .map(|text| text.trim())
        .filter(|text| !text.is_empty())
        .map(|text| text.to_string());
    let plain = data["plainLyrics"]
        .as_str()
        .map(|text| text.trim())
        .filter(|text| !text.is_empty())
        .map(|text| text.to_string());

    if let Some(text) = synced {
        return Some(LyricsResult {
            kind: "lrc".to_string(),
            text,
        });
    }
    plain.map(|text| LyricsResult {
        kind: "txt".to_string(),
        text,
    })
}

async fn fetch_lrclib(lookup: &LyricsLookup) -> Option<LyricsResult> {
    let mut params = vec![
        ("track_name", lookup.title.clone()),
        ("artist_name", lookup.artist.clone()),
        ("album_name", lookup.album.clone()),
    ];
    if lookup.duration > 0.0 {
        params.push(("duration", format!("{}", lookup.duration.round() as i64)));
    }

    let query: String = params
        .iter()
        .map(|(key, value)| format!("{}={}", key, urlencoding::encode(value)))
        .collect::<Vec<_>>()
        .join("&");

    let client = http_client();
    let mut fallback: Option<LyricsResult> = None;

    if let Ok(response) = client
        .get(format!("{LRCLIB_BASE}/api/get?{query}"))
        .send()
        .await
        && response.status().as_u16() == 200
        && let Ok(data) = response.json::<serde_json::Value>().await
        && duration_acceptable(&data, lookup)
        && let Some(result) = parse_lrclib(Some(&data))
    {
        if is_synced(&result.kind) {
            return Some(result);
        }
        fallback = Some(result);
    }

    let mut search_params: Vec<String> = Vec::new();
    if !lookup.title.is_empty() {
        search_params.push(format!("track_name={}", urlencoding::encode(&lookup.title)));
    }
    if !lookup.artist.is_empty() {
        search_params.push(format!(
            "artist_name={}",
            urlencoding::encode(&lookup.artist)
        ));
    }
    if search_params.is_empty() {
        return None;
    }

    let search_url = format!("{LRCLIB_BASE}/api/search?{}", search_params.join("&"));
    let response = match client.get(search_url).send().await {
        Ok(response) if response.status().as_u16() == 200 => response,
        _ => return fallback,
    };
    let items: Vec<serde_json::Value> = response.json().await.unwrap_or_default();
    let mut best_synced: Option<(i32, LyricsResult)> = None;
    let mut best_plain: Option<(i32, LyricsResult)> = None;
    for item in &items {
        if !duration_acceptable(item, lookup) {
            continue;
        }
        let Some(result) = parse_lrclib(Some(item)) else {
            continue;
        };
        let score = score_hit(&lrclib_hit(item), lookup);
        let slot = if is_synced(&result.kind) {
            &mut best_synced
        } else {
            &mut best_plain
        };
        if slot.as_ref().is_none_or(|(best, _)| score > *best) {
            *slot = Some((score, result));
        }
    }
    if let Some((_, hit)) = best_synced {
        return Some(hit);
    }
    if let Some((_, hit)) = best_plain {
        fallback.get_or_insert(hit);
    }
    fallback
}

async fn fetch_lrcred(lookup: &LyricsLookup) -> Option<LyricsResult> {
    let query = format!("{} {}", lookup.artist, lookup.title);
    let search_url = format!(
        "{LRCRED_BASE}/search.json?q={}",
        urlencoding::encode(&query)
    );

    let hits: serde_json::Value = match http_client().get(search_url).send().await {
        Ok(response) if response.status().as_u16() == 200 => response.json().await.ok()?,
        _ => return None,
    };

    let items = hits.get("hits")?.as_array()?;
    let mut best: Option<(i32, String)> = None;
    for item in items {
        if !duration_acceptable(item, lookup) {
            continue;
        }
        let Some(isrc) = item["isrc"].as_str() else {
            continue;
        };
        let score = score_hit(&lrcred_hit(item), lookup);
        if best
            .as_ref()
            .is_none_or(|(best_score, _)| score > *best_score)
        {
            best = Some((score, isrc.to_string()));
        }
    }

    let (_, isrc) = best?;
    for extension in ["ttml", "lrc"] {
        let url = format!("{LRCRED_BASE}/s/{}.{extension}", urlencoding::encode(&isrc));
        let response = match http_client().get(url).send().await {
            Ok(response) if response.status().as_u16() == 200 => response,
            _ => continue,
        };
        let Ok(text) = response.text().await else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        return Some(LyricsResult {
            kind: extension.to_string(),
            text: text.trim().to_string(),
        });
    }
    None
}

fn parse_betterlyrics(data: &serde_json::Value) -> Option<LyricsResult> {
    for (key, kind) in [
        ("ttml", "ttml"),
        ("syncedLyrics", "lrc"),
        ("lrc", "lrc"),
        ("lyrics", "txt"),
    ] {
        let Some(text) = data.get(key).and_then(|value| value.as_str()) else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        return Some(LyricsResult {
            kind: kind.to_string(),
            text: text.trim().to_string(),
        });
    }
    None
}

async fn fetch_betterlyrics(lookup: &LyricsLookup) -> Option<LyricsResult> {
    if lookup.duration <= 0.0 {
        return None;
    }
    let client = http_client();
    for with_album in [true, false] {
        if !with_album && lookup.album.is_empty() {
            break;
        }
        let mut params = vec![
            ("s", lookup.title.clone()),
            ("a", lookup.artist.clone()),
            ("d", format!("{}", lookup.duration.round() as i64)),
        ];
        if with_album && !lookup.album.is_empty() {
            params.push(("al", lookup.album.clone()));
        }
        let query = params
            .iter()
            .map(|(key, value)| format!("{}={}", key, urlencoding::encode(value)))
            .collect::<Vec<_>>()
            .join("&");
        let response = match client
            .get(format!("{BETTERLYRICS_BASE}/getLyrics?{query}"))
            .send()
            .await
        {
            Ok(response) if response.status().as_u16() == 200 => response,
            _ => continue,
        };
        let Ok(json) = response.json::<serde_json::Value>().await else {
            continue;
        };
        if let Some(result) = parse_betterlyrics(&json) {
            return Some(result);
        }
    }
    None
}

pub async fn fetch(lookup: &LyricsLookup) -> Option<LyricsResult> {
    let lrclib = fetch_lrclib(lookup).await;
    if lrclib.as_ref().is_some_and(|hit| is_synced(&hit.kind)) {
        return lrclib;
    }
    if let Some(hit) = fetch_lrcred(lookup).await {
        return Some(hit);
    }
    let better = fetch_betterlyrics(lookup).await;
    if better.as_ref().is_some_and(|hit| is_synced(&hit.kind)) {
        return better;
    }
    lrclib.or(better)
}
