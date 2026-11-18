use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub const GITHUB_UA: &str = concat!("Vynl/", env!("CARGO_PKG_VERSION"));

pub(crate) fn public_addrs(host: &str) -> Option<Vec<std::net::SocketAddr>> {
    use std::net::ToSocketAddrs;
    let addrs: Vec<std::net::SocketAddr> = (host, 443u16).to_socket_addrs().ok()?.collect();
    if addrs.is_empty() {
        return None;
    }
    if addrs
        .iter()
        .all(|addr| crate::services::rpc::is_public_ip(&addr.ip()))
    {
        Some(addrs)
    } else {
        None
    }
}

pub fn host_resolves_publicly(host: &str) -> bool {
    public_addrs(host).is_some()
}

pub fn github_client(read_timeout: std::time::Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(GITHUB_UA)
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(std::time::Duration::from_secs(15))
        .read_timeout(read_timeout)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
}

pub async fn follow_public_https(
    _http: &reqwest::Client,
    url: &str,
    max_hops: u8,
) -> Result<reqwest::Response, String> {
    let mut current = url.to_string();
    for _ in 0..max_hops {
        let parsed = reqwest::Url::parse(&current).map_err(|_| "invalid URL".to_string())?;
        if parsed.scheme() != "https" {
            return Err("only https targets are allowed".into());
        }
        let host = parsed
            .host_str()
            .ok_or_else(|| "missing host".to_string())?
            .to_string();
        if !crate::services::rpc::is_public_host(&host) {
            return Err("host points at a private or reserved address".into());
        }
        let addrs = public_addrs(&host)
            .ok_or_else(|| "host resolves to a private or reserved address".to_string())?;
        let bound = addrs.iter().fold(
            reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none()),
            |builder, addr| builder.resolve(&host, *addr),
        );
        let bound = bound
            .build()
            .map_err(|e| format!("could not build the fetch client: {e}"))?;
        let resp = bound
            .get(&current)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;
        if resp.status().is_redirection() {
            let location = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| "missing redirect target".to_string())?
                .to_string();
            current = parsed
                .join(&location)
                .map_err(|_| "bad redirect target".to_string())?
                .to_string();
            continue;
        }
        return Ok(resp);
    }
    Err("too many redirects".into())
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn write_json<T: serde::Serialize>(path: &Path, data: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = path.with_extension(format!("tmp.{}.{}", std::process::id(), unique));
    fs::write(&tmp, &json).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        e.to_string()
    })
}

pub fn norm_path_key(key: &str) -> String {
    let stripped = key.trim_start_matches(r"\\?\");
    if cfg!(windows) {
        stripped.replace('/', "\\").to_lowercase()
    } else {
        stripped.replace('\\', "/")
    }
}

#[macro_export]
macro_rules! declare_file_mutex {
    () => {
        static FILE_MUTEX: ::std::sync::OnceLock<::std::sync::Mutex<()>> =
            ::std::sync::OnceLock::new();

        fn file_mutex() -> &'static ::std::sync::Mutex<()> {
            FILE_MUTEX.get_or_init(|| ::std::sync::Mutex::new(()))
        }
    };
}
