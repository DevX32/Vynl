use discord_rich_presence::activity::{Activity, ActivityType, Assets, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use once_cell::sync::Lazy;
use std::sync::{Mutex, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::commands::types::RpcPresence;

const APP_ID: u64 = 1538939101225427014;

static WORKER_TX: Lazy<Mutex<Option<mpsc::Sender<Option<RpcPresence>>>>> =
    Lazy::new(|| Mutex::new(None));
static LAST_PRESENCE: Lazy<Mutex<Option<RpcPresence>>> = Lazy::new(|| Mutex::new(None));

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn is_public_v4(ip: &std::net::Ipv4Addr) -> bool {
    if ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
    {
        return false;
    }
    let octets = ip.octets();
    if octets[0] == 0 {
        return false;
    }
    if octets[0] == 100 && (octets[1] & 0b1100_0000) == 0b0100_0000 {
        return false;
    }
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 0 {
        return false;
    }
    if octets[0] == 198 && (octets[1] & 0b1111_1110) == 0b0001_0010 {
        return false;
    }
    true
}

fn is_public_v6_embedded_v4(ip: &std::net::Ipv6Addr) -> Option<std::net::IpAddr> {
    let segments = ip.segments();
    if segments[0] == 0x64 && segments[1] == 0xff9b {
        let embedded = std::net::Ipv4Addr::new(
            ((segments[6] >> 8) & 0xff) as u8,
            (segments[6] & 0xff) as u8,
            ((segments[7] >> 8) & 0xff) as u8,
            (segments[7] & 0xff) as u8,
        );
        return Some(std::net::IpAddr::V4(embedded));
    }
    if segments[0] == 0x2002 {
        let embedded = std::net::Ipv4Addr::new(
            ((segments[1] >> 8) & 0xff) as u8,
            (segments[1] & 0xff) as u8,
            ((segments[2] >> 8) & 0xff) as u8,
            (segments[2] & 0xff) as u8,
        );
        return Some(std::net::IpAddr::V4(embedded));
    }
    None
}

pub fn is_public_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ip) => is_public_v4(ip),
        std::net::IpAddr::V6(ip) => {
            if let Some(embedded) = is_public_v6_embedded_v4(ip) {
                return is_public_ip(&embedded);
            }
            if let Some(mapped) = ip.to_ipv4_mapped() {
                return is_public_v4(&mapped);
            }
            let segments = ip.segments();
            let unique_local = (segments[0] & 0xfe00) == 0xfc00;
            let link_local = (segments[0] & 0xffc0) == 0xfe80;
            let multicast = (segments[0] & 0xff00) == 0xff00;
            !(ip.is_loopback() || ip.is_unspecified() || unique_local || link_local || multicast)
        }
    }
}

pub fn is_public_host(host: &str) -> bool {
    let trimmed = host.trim();
    let trimmed = trimmed.trim_start_matches('[').trim_end_matches(']');
    let normalized = trimmed.trim_end_matches('.');
    if let Ok(ip) = normalized.parse::<std::net::IpAddr>() {
        return is_public_ip(&ip);
    }

    let normalized = normalized.to_lowercase();
    !(normalized.eq_ignore_ascii_case("localhost")
        || normalized.ends_with(".local")
        || normalized.ends_with(".localhost")
        || !normalized.contains('.'))
}

pub fn is_public_cover(url: &str) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "https" {
        return false;
    }

    let Some(host) = parsed.host_str() else {
        return false;
    };
    is_public_host(host)
}

fn build_activity(p: &RpcPresence) -> Activity<'static> {
    let title = if p.title.is_empty() {
        "Vynl".to_string()
    } else {
        p.title.clone()
    };
    let state = if p.artist.is_empty() {
        if p.album.is_empty() {
            "Vynl".to_string()
        } else {
            p.album.clone()
        }
    } else {
        p.artist.clone()
    };

    let activity = Activity::new()
        .details(title)
        .state(state)
        .activity_type(ActivityType::Listening);

    let activity = if p.playing {
        let start = (now_secs() as i64 - p.time as i64).max(0);
        if p.duration > 0.0 {
            activity.timestamps(
                Timestamps::new()
                    .start(start)
                    .end(start + p.duration as i64),
            )
        } else {
            activity.timestamps(Timestamps::new().start(start))
        }
    } else {
        activity
    };

    let public_cover = p
        .cover
        .as_deref()
        .filter(|cover| is_public_cover(cover))
        .map(crate::services::cover_cache::normalize_cover_url)
        .map(|cover| cover.into_owned());
    let has_cover = public_cover.is_some();
    let img = public_cover.unwrap_or_else(|| "vynl".to_string());
    let mut assets = Assets::new().large_image(img);

    if has_cover {
        assets = assets.small_image("vynl").small_text("Vynl");
    }
    if !p.album.is_empty() {
        assets = assets.large_text(p.album.clone());
    }

    activity.assets(assets)
}

fn rpc_worker(rx: mpsc::Receiver<Option<RpcPresence>>) {
    let mut client: Option<DiscordIpcClient> = None;
    while let Ok(presence) = rx.recv() {
        for attempt in 0..2 {
            if client.is_none() {
                let mut candidate = DiscordIpcClient::new(APP_ID.to_string());
                if candidate.connect().is_ok() {
                    client = Some(candidate);
                } else {
                    break;
                }
            }
            let activity = presence.as_ref().map(build_activity);
            let ok = match (client.as_mut(), activity) {
                (Some(c), Some(a)) => c.set_activity(a).is_ok(),
                (Some(c), None) => c.clear_activity().is_ok(),
                _ => false,
            };
            if ok {
                break;
            }
            client = None;
            if attempt == 0 {
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
}

fn worker_tx() -> Option<mpsc::Sender<Option<RpcPresence>>> {
    let mut guard = WORKER_TX.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        let (tx, rx) = mpsc::channel::<Option<RpcPresence>>();
        match std::thread::Builder::new()
            .name("vynl-discord-rpc".to_string())
            .spawn(move || rpc_worker(rx))
        {
            Ok(_) => *guard = Some(tx),
            Err(error) => {
                eprintln!("discord rpc worker failed to start: {error}");
                return None;
            }
        }
    }
    guard.clone()
}

pub fn update_presence(presence: Option<&RpcPresence>) {
    {
        let mut last = LAST_PRESENCE.lock().unwrap_or_else(|e| e.into_inner());
        *last = presence.cloned();
    }
    if let Some(tx) = worker_tx() {
        let _ = tx.send(presence.cloned());
    }
}

pub fn patch_cover(artist: &str, album: &str, title: &str, cover: String) {
    let cached = LAST_PRESENCE.lock().unwrap_or_else(|e| e.into_inner());
    let Some(current) = cached.as_ref() else {
        return;
    };
    if current.artist != artist || current.album != album || current.title != title {
        return;
    }
    if current.cover.as_deref() == Some(cover.as_str()) {
        return;
    }
    let mut next = current.clone();
    next.cover = Some(cover);
    drop(cached);
    update_presence(Some(&next));
}
