use crate::commands::types::*;
use crate::services::catalog;

use super::{SHORT_LINK_RE, UA};

async fn follow_short_link(raw: &str) -> Option<catalog::CatalogLink> {
    let trimmed = raw.trim();
    if !SHORT_LINK_RE.is_match(trimmed) {
        return None;
    }

    if let Ok(parsed) = reqwest::Url::parse(trimmed) {
        match parsed.scheme() {
            "http" | "https" => {}
            _ => return None,
        }
        if let Some(host) = parsed.host_str() {
            let lower = host.to_lowercase();
            if lower == "localhost"
                || lower == "127.0.0.1"
                || lower == "::1"
                || lower.ends_with(".local")
                || lower.starts_with("10.")
                || lower.starts_with("192.168.")
                || lower.starts_with("172.")
            {
                return None;
            }
        }
    } else {
        return None;
    }

    let client = reqwest::Client::builder()
        .user_agent(UA)
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .ok()?;

    let resp = client.get(trimmed).send().await.ok()?;
    let location = resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if location.is_empty() {
        return None;
    }

    catalog::parse_link(location)
}

pub async fn resolve_link(raw_url: &str) -> Result<Collection, String> {
    if let Some(result) = crate::services::catalogue::try_resolve_yt_playlist(raw_url).await {
        return result;
    }
    let link = catalog::parse_link(raw_url)
        .or(async { follow_short_link(raw_url).await }.await)
        .ok_or_else(|| "That does not look like a track, album, or playlist link.".to_string())?;
    catalog::fetch_collection(&link).await
}
