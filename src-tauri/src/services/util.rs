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

pub fn github_client(read_timeout: std::time::Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(GITHUB_UA)
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(std::time::Duration::from_secs(15))
        .read_timeout(read_timeout)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))
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
    fs::write(path, json).map_err(|e| e.to_string())
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
