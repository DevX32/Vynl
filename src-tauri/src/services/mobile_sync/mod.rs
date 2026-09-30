mod server;

use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, oneshot};

use crate::commands::types::{LibraryTrack, Playlist};
use crate::services::{library, playlists, settings, util};

pub use server::MobileCatalogTrack;
pub use server::MobilePlaylistDto;
pub use server::MobileSyncManifestEntry;

pub const DEFAULT_PORT: u16 = 17865;

const MDNS_SERVICE_TYPE: &str = "_vynl._tcp.local.";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileSyncConfig {
    pub enabled: bool,
    pub port: u16,
    pub pairing_pin: String,
    pub api_token: String,
}

impl Default for MobileSyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_PORT,
            pairing_pin: generate_pin(),
            api_token: generate_token(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileSyncStatus {
    pub enabled: bool,
    pub running: bool,
    pub port: u16,
    pub pairing_pin: String,
    pub lan_addresses: Vec<String>,
    pub pair_urls: Vec<String>,
    pub hostname: Option<String>,
    pub qr_svg: Option<String>,
}

struct Runtime {
    stop: Option<oneshot::Sender<()>>,
    mdns: Option<mdns_sd::ServiceDaemon>,
    user_data: PathBuf,
}

static RUNTIME: Lazy<Mutex<Runtime>> = Lazy::new(|| {
    Mutex::new(Runtime {
        stop: None,
        mdns: None,
        user_data: PathBuf::new(),
    })
});

fn config_path(user_data_dir: &Path) -> PathBuf {
    user_data_dir.join("mobile_sync.json")
}

fn generate_pin() -> String {
    let n = (uuid::Uuid::new_v4().as_u128() % 1_000_000) as u32;
    format!("{n:06}")
}

fn generate_token() -> String {
    format!(
        "{}{}",
        hex::encode(uuid::Uuid::new_v4().as_bytes()),
        hex::encode(uuid::Uuid::new_v4().as_bytes())
    )
}

pub fn load_config(user_data_dir: &Path) -> MobileSyncConfig {
    util::read_json(&config_path(user_data_dir)).unwrap_or_default()
}

fn save_config(user_data_dir: &Path, cfg: &MobileSyncConfig) -> Result<(), String> {
    util::write_json(&config_path(user_data_dir), cfg)
}

pub fn list_lan_addresses() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(ifaces) = local_addrs() {
        for ip in ifaces {
            if ip.is_loopback() {
                continue;
            }
            match ip {
                IpAddr::V4(v4) if !v4.is_link_local() => out.push(v4.to_string()),
                IpAddr::V6(_) => {}
                _ => {}
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn local_addrs() -> Result<Vec<IpAddr>, String> {
    use std::net::UdpSocket;
    let mut addrs = Vec::new();
    // Best-effort: connect UDP to a public IP (no packets sent) to discover the LAN iface.
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0")
        && socket.connect("8.8.8.8:80").is_ok()
        && let Ok(local) = socket.local_addr()
    {
        addrs.push(local.ip());
    }
    // Also try hostname resolution for extra candidates.
    if let Ok(host) = hostname()
        && let Ok(iter) = dns_lookup(&host)
    {
        for ip in iter {
            if !addrs.contains(&ip) {
                addrs.push(ip);
            }
        }
    }
    Ok(addrs)
}

fn hostname() -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        let out = Command::new("hostname")
            .output()
            .map_err(|e| e.to_string())?;
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
    #[cfg(not(windows))]
    {
        let mut buf = [0u8; 256];
        let ok = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut _, buf.len()) };
        if ok != 0 {
            return Err("gethostname failed".into());
        }
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8(buf[..end].to_vec()).map_err(|e| e.to_string())
    }
}

fn dns_lookup(host: &str) -> Result<Vec<IpAddr>, String> {
    use std::net::ToSocketAddrs;
    let mut out = Vec::new();
    for addr in format!("{host}:0")
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
    {
        out.push(addr.ip());
    }
    Ok(out)
}

fn mdns_label() -> Option<String> {
    let raw = hostname().ok()?;
    let mut label: String = raw
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    while label.starts_with('-') {
        label.remove(0);
    }
    while label.ends_with('-') {
        label.pop();
    }
    if label.is_empty() {
        None
    } else {
        Some(label.chars().take(15).collect())
    }
}

pub fn mdns_hostname() -> Option<String> {
    mdns_label().map(|l| format!("{l}.local"))
}

fn start_mdns(port: u16) -> Option<mdns_sd::ServiceDaemon> {
    let label = mdns_label()?;
    let daemon = mdns_sd::ServiceDaemon::new().ok()?;
    let mut registered = false;

    for ip in list_lan_addresses() {
        let info = mdns_sd::ServiceInfo::new(
            MDNS_SERVICE_TYPE,
            &label,
            &format!("{label}.local."),
            &ip,
            port,
            &[] as &[(&str, &str)],
        );
        if let Ok(info) = info
            && daemon.register(info).is_ok()
        {
            registered = true;
        }
    }

    if registered {
        eprintln!("[mobile_sync] advertising {label}.local:{port} over mDNS");
        Some(daemon)
    } else {
        let _ = daemon.shutdown();
        None
    }
}

fn pair_urls(port: u16) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(host) = mdns_hostname() {
        out.push(format!("http://{host}:{port}"));
    }
    out.extend(
        list_lan_addresses()
            .into_iter()
            .map(|ip| format!("http://{ip}:{port}")),
    );
    out
}

fn pairing_payload(host: &str, port: u16, pin: &str) -> String {
    format!("vynl://pair?h={host}&p={port}&pin={pin}")
}

fn qr_for_payload(payload: &str) -> Option<String> {
    use qrcode::QrCode;
    use qrcode::render::svg;
    let code = QrCode::new(payload.as_bytes()).ok()?;
    Some(
        code.render::<svg::Color>()
            .min_dimensions(180, 180)
            .dark_color(svg::Color("#111111"))
            .light_color(svg::Color("#ffffff"))
            .build(),
    )
}

pub fn status(user_data_dir: &Path, running: bool) -> MobileSyncStatus {
    let cfg = load_config(user_data_dir);
    let urls = pair_urls(cfg.port);
    let primary = urls
        .iter()
        .find(|u| u.contains(".local"))
        .or_else(|| urls.first())
        .cloned();
    let qr_svg = primary.and_then(|url| {
        let host = url
            .trim_start_matches("http://")
            .split(':')
            .next()
            .unwrap_or_default();
        qr_for_payload(&pairing_payload(host, cfg.port, &cfg.pairing_pin))
    });
    MobileSyncStatus {
        enabled: cfg.enabled,
        running,
        port: cfg.port,
        pairing_pin: cfg.pairing_pin,
        lan_addresses: list_lan_addresses(),
        pair_urls: urls,
        hostname: mdns_hostname(),
        qr_svg,
    }
}

pub async fn is_running() -> bool {
    RUNTIME.lock().await.stop.is_some()
}

pub async fn get_status(user_data_dir: &Path) -> MobileSyncStatus {
    let running = is_running().await;
    status(user_data_dir, running)
}

pub async fn set_enabled(user_data_dir: &Path, enabled: bool) -> Result<MobileSyncStatus, String> {
    let mut cfg = load_config(user_data_dir);
    cfg.enabled = enabled;
    if cfg.pairing_pin.is_empty() {
        cfg.pairing_pin = generate_pin();
    }
    if cfg.api_token.is_empty() {
        cfg.api_token = generate_token();
    }
    if cfg.port == 0 {
        cfg.port = DEFAULT_PORT;
    }
    save_config(user_data_dir, &cfg)?;

    if enabled {
        start(user_data_dir).await?;
    } else {
        stop().await;
    }
    Ok(get_status(user_data_dir).await)
}

pub async fn rotate_pin(user_data_dir: &Path) -> Result<MobileSyncStatus, String> {
    let mut cfg = load_config(user_data_dir);
    cfg.pairing_pin = generate_pin();
    cfg.api_token = generate_token();
    save_config(user_data_dir, &cfg)?;
    // Restart so in-memory auth matches disk.
    if cfg.enabled {
        stop().await;
        start(user_data_dir).await?;
    }
    Ok(get_status(user_data_dir).await)
}

pub async fn start(user_data_dir: &Path) -> Result<(), String> {
    let cfg = load_config(user_data_dir);
    if !cfg.enabled {
        return Ok(());
    }

    let mut rt = RUNTIME.lock().await;
    if rt.stop.is_some() {
        return Ok(());
    }

    let (tx, rx) = oneshot::channel::<()>();
    let user_data = user_data_dir.to_path_buf();
    let port = if cfg.port == 0 {
        DEFAULT_PORT
    } else {
        cfg.port
    };
    let pin = cfg.pairing_pin.clone();
    let token = cfg.api_token.clone();

    let state = Arc::new(server::AppStateInner {
        user_data: user_data.clone(),
        pin: Mutex::new(pin),
        token: Mutex::new(token),
    });

    tokio::spawn(async move {
        if let Err(e) = server::serve(port, state, rx).await {
            eprintln!("[mobile_sync] server error: {e}");
        }
    });

    rt.stop = Some(tx);
    rt.mdns = start_mdns(port);
    rt.user_data = user_data;
    Ok(())
}

pub async fn stop() {
    let mut rt = RUNTIME.lock().await;
    if let Some(tx) = rt.stop.take() {
        let _ = tx.send(());
    }
    if let Some(daemon) = rt.mdns.take() {
        let _ = daemon.shutdown();
    }
}

pub async fn ensure_started_on_boot(user_data_dir: &Path) {
    let cfg = load_config(user_data_dir);
    if cfg.enabled {
        let _ = start(user_data_dir).await;
    }
}

pub fn load_library_tracks(user_data: &Path) -> Vec<LibraryTrack> {
    let settings = settings::get_settings(user_data);
    let output = PathBuf::from(&settings.output_dir);
    if let Some(cached) = library::load_cache(user_data) {
        return cached;
    }
    library::scan_library(&output, user_data)
}

pub fn load_playlists(user_data: &Path) -> Vec<Playlist> {
    let settings = settings::get_settings(user_data);
    let output = PathBuf::from(&settings.output_dir);
    playlists::list_playlists(&output)
        .into_iter()
        .filter_map(|meta| playlists::get_playlist(&output, &meta.id))
        .collect()
}
