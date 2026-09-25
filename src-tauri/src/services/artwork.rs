use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::Mutex;

static CACHE: Lazy<Mutex<HashMap<String, Option<String>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Deserialize)]
struct SearchResponse {
    results: Vec<SearchResult>,
}

#[derive(Deserialize)]
struct SearchResult {
    #[serde(rename = "artworkUrl100")]
    artwork_url_100: Option<String>,
}

fn cache_key(artist: &str, title: &str) -> String {
    format!(
        "{}|{}",
        artist.trim().to_lowercase(),
        title.trim().to_lowercase()
    )
}

fn upsize(url: &str) -> String {
    let url = url.replacen("http://", "https://", 1);
    url.replace("100x100bb", "512x512bb")
}

pub async fn lookup(artist: &str, title: &str) -> Option<String> {
    if artist.trim().is_empty() && title.trim().is_empty() {
        return None;
    }

    let key = cache_key(artist, title);

    {
        let cache = CACHE.lock().await;
        if let Some(hit) = cache.get(&key) {
            return hit.clone();
        }
    }

    let result = fetch(artist, title).await;

    let mut cache = CACHE.lock().await;
    cache.insert(key, result.clone());
    result
}

async fn fetch(artist: &str, title: &str) -> Option<String> {
    let term = format!("{artist} {title}");
    let client = reqwest::Client::new();
    let resp = client
        .get("https://itunes.apple.com/search")
        .query(&[
            ("term", term.as_str()),
            ("media", "music"),
            ("entity", "song"),
            ("limit", "1"),
        ])
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?;

    let parsed: SearchResponse = resp.json().await.ok()?;
    parsed
        .results
        .into_iter()
        .next()
        .and_then(|r| r.artwork_url_100)
        .map(|u| upsize(&u))
}
