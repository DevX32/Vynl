use discord_rich_presence::activity::{Activity, ActivityType, Assets, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::commands::types::RpcPresence;

const APP_ID: u64 = 1538939101225427014;

static CLIENT: Lazy<Mutex<Option<DiscordIpcClient>>> = Lazy::new(|| Mutex::new(None));
static PENDING: Lazy<Mutex<Option<RpcPresence>>> = Lazy::new(|| Mutex::new(None));
static CONNECTED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

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

fn spawn_retry_thread() {
    std::thread::spawn(|| {
        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(250));
            let pending = match PENDING.lock() {
                Ok(p) => p.clone(),
                Err(_) => return,
            };
            let mut guard = match CLIENT.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            let Some(client) = guard.as_mut() else {
                continue;
            };
            let applied = match &pending {
                Some(p) => client.set_activity(build_activity(p)).is_ok(),
                None => client.clear_activity().is_ok(),
            };
            if applied {
                return;
            }
        }
    });
}

fn reset_client() {
    if let Ok(mut guard) = CLIENT.lock() {
        *guard = None;
    }
    if let Ok(mut connected) = CONNECTED.lock() {
        *connected = false;
    }
}

fn ensure_client() -> Option<std::sync::MutexGuard<'static, Option<DiscordIpcClient>>> {
    let mut guard = CLIENT.lock().ok()?;
    if guard.is_none() {
        let mut c = DiscordIpcClient::new(APP_ID.to_string());
        if c.connect().is_err() {
            return None;
        }
        *guard = Some(c);

        let should_retry = match CONNECTED.lock() {
            Ok(mut connected) => {
                if *connected {
                    false
                } else {
                    *connected = true;
                    true
                }
            }
            Err(_) => false,
        };

        if should_retry {
            drop(guard);
            spawn_retry_thread();
            return CLIENT.lock().ok();
        }
    }
    Some(guard)
}

enum Applied {
    Done,
    Stale,
    NoClient,
}

fn apply_presence(presence: Option<&RpcPresence>) -> Applied {
    let Some(mut guard) = ensure_client() else {
        return Applied::NoClient;
    };
    let Some(client) = guard.as_mut() else {
        return Applied::NoClient;
    };
    let ok = match presence {
        Some(p) => client.set_activity(build_activity(p)).is_ok(),
        None => client.clear_activity().is_ok(),
    };
    if ok { Applied::Done } else { Applied::Stale }
}

pub fn update_presence(presence: Option<&RpcPresence>) {
    if let Ok(mut pending) = PENDING.lock() {
        *pending = presence.cloned();
    }

    match apply_presence(presence) {
        Applied::Done => {}
        Applied::Stale => {
            reset_client();
            apply_presence(presence);
        }
        Applied::NoClient => {}
    }
}

pub fn patch_cover(artist: &str, album: &str, title: &str, cover: String) {
    let next = PENDING.lock().ok().and_then(|p| {
        let current = p.as_ref()?;
        if current.artist != artist || current.album != album || current.title != title {
            return None;
        }
        if current.cover.as_deref() == Some(cover.as_str()) {
            return None;
        }
        Some(RpcPresence {
            cover: Some(cover.clone()),
            ..current.clone()
        })
    });
    if let Some(next) = next {
        update_presence(Some(&next));
    }
}
