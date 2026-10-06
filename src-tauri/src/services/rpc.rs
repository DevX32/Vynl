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

    let img = match &p.cover {
        Some(cover) if cover.starts_with("http://") || cover.starts_with("https://") => {
            cover.clone()
        }
        _ => "vynl".to_string(),
    };
    let assets = Assets::new().large_image(img);
    let assets = if !p.album.is_empty() {
        assets.large_text(p.album.clone())
    } else {
        assets
    };
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

pub fn patch_cover(artist: &str, title: &str, cover: String) {
    let next = PENDING.lock().ok().and_then(|p| {
        let current = p.as_ref()?;
        if current.artist != artist || current.title != title {
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
