use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::commands::types::{
    PluginEntry, PluginHttpOptions, PluginHttpResponse, PluginInstallationMethod, PluginManifest,
    PluginStoreResult, StorePlugin,
};
use crate::services::util;

const REGISTRY_FILE: &str = "registry.json";
const STORE_CACHE_FILE: &str = "store-cache.json";
const CONFIG_DIR: &str = "config";
const STAGING_DIR: &str = ".staging";

const MAX_ARCHIVE_BYTES: u64 = 50 * 1024 * 1024;
const MAX_EXTRACTED_BYTES: u64 = 200 * 1024 * 1024;
const MAX_HTTP_BODY_BYTES: usize = 10 * 1024 * 1024;

pub const KNOWN_CATEGORIES: [&str; 8] = [
    "metadata",
    "lyrics",
    "artwork",
    "download",
    "dashboard",
    "playlists",
    "scrobbling",
    "other",
];

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

crate::declare_file_mutex!();

fn http_client() -> Result<&'static reqwest::Client, String> {
    if let Some(c) = HTTP_CLIENT.get() {
        return Ok(c);
    }
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;
    Ok(HTTP_CLIENT.get_or_init(|| client))
}

pub fn app_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))
}

pub fn plugins_root(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_dir(app)?.join("plugins");
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create plugins dir: {e}"))?;
    Ok(dir)
}

fn registry_path(root: &Path) -> PathBuf {
    root.join(REGISTRY_FILE)
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct PluginRegistry {
    #[serde(default)]
    plugins: Vec<PluginEntry>,
}

fn load_registry_unlocked(root: &Path) -> PluginRegistry {
    util::read_json(&registry_path(root)).unwrap_or_default()
}

fn save_registry_unlocked(root: &Path, reg: &PluginRegistry) -> Result<(), String> {
    util::write_json(&registry_path(root), reg)
        .map_err(|e| format!("Failed to save plugin registry: {e}"))
}

fn mutate_registry<F, T>(root: &Path, f: F) -> Result<T, String>
where
    F: FnOnce(&mut PluginRegistry) -> Result<T, String>,
{
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let mut reg = load_registry_unlocked(root);
    let out = f(&mut reg)?;
    save_registry_unlocked(root, &reg)?;
    Ok(out)
}

pub fn list(app: &AppHandle) -> Result<Vec<PluginEntry>, String> {
    let root = plugins_root(app)?;
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let mut reg = load_registry_unlocked(&root);
    reg.plugins.sort_by_key(|plugin| plugin.installed_at);
    Ok(reg.plugins)
}

fn get_entry(app: &AppHandle, id: &str) -> Result<PluginEntry, String> {
    validate_plugin_id(id)?;
    let root = plugins_root(app)?;
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let reg = load_registry_unlocked(&root);
    reg.plugins
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("Plugin '{id}' is not installed"))
}

pub fn validate_plugin_id(id: &str) -> Result<(), String> {
    if id.len() < 2 || id.len() > 64 {
        return Err("Plugin id must be 2-64 characters".into());
    }
    let ok = id
        .bytes()
        .enumerate()
        .all(|(i, b)| b.is_ascii_lowercase() || b.is_ascii_digit() || (i > 0 && b == b'-'));
    if !ok {
        return Err("Plugin id must be lowercase alphanumeric with hyphens".into());
    }
    Ok(())
}

fn validate_version(version: &str) -> Result<(), String> {
    let core = version.split(['-', '+']).next().unwrap_or("");
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3
        || !parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(format!("Version '{version}' must be semver (e.g. 1.0.0)"));
    }
    Ok(())
}

fn clean_categories(cats: Option<Vec<String>>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in cats.into_iter().flatten() {
        let c = c.to_lowercase();
        if KNOWN_CATEGORIES.contains(&c.as_str()) && !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

fn read_manifest(dir: &Path) -> Result<PluginManifest, String> {
    let raw = fs::read_to_string(dir.join("package.json")).map_err(|_| {
        format!(
            "No package.json found in {} (plugin files must be at the root)",
            dir.display()
        )
    })?;
    let manifest: PluginManifest =
        serde_json::from_str(&raw).map_err(|e| format!("Invalid package.json: {e}"))?;
    validate_plugin_id(&manifest.name)?;
    validate_version(&manifest.version)?;
    Ok(manifest)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Failed to create {dst:?}: {e}"))?;
    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read {src:?}: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if file_type.is_dir() {
            copy_dir_all(&from, &to)?;
        } else if file_type.is_file() {
            fs::copy(&from, &to).map_err(|e| format!("Failed to copy {from:?}: {e}"))?;
        }
    }
    Ok(())
}

fn extract_zip_bytes(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("Invalid zip: {e}"))?;
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;

    let mut total: u64 = 0;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {e}"))?;
        let Some(rel) = file.enclosed_name() else {
            continue;
        };
        if file.is_symlink() {
            continue;
        }
        let out = dest.join(rel);
        if file.is_dir() {
            fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        total += file.size();
        if total > MAX_EXTRACTED_BYTES {
            return Err("Plugin archive is too large when extracted".into());
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out_file = fs::File::create(&out)
            .map_err(|e| format!("Failed to create {}: {e}", out.display()))?;
        std::io::copy(&mut file, &mut out_file)
            .map_err(|e| format!("Failed to extract {}: {e}", out.display()))?;
    }
    Ok(())
}

fn locate_plugin_root(staging: &Path) -> Result<PathBuf, String> {
    if staging.join("package.json").is_file() {
        return Ok(staging.to_path_buf());
    }
    let mut dirs = Vec::new();
    if let Ok(read) = fs::read_dir(staging) {
        for entry in read.flatten() {
            let name = entry.file_name();
            if name == "__MACOSX" || name.to_string_lossy().starts_with('.') {
                continue;
            }
            if entry.path().is_dir() {
                dirs.push(entry.path());
            }
        }
    }
    if dirs.len() == 1 && dirs[0].join("package.json").is_file() {
        return Ok(dirs.remove(0));
    }
    Err("package.json not found at the root of the plugin archive".into())
}

fn install_from_dir(
    app: &AppHandle,
    src: &Path,
    method: PluginInstallationMethod,
    original_path: Option<String>,
    source_repo: Option<String>,
) -> Result<PluginEntry, String> {
    let manifest = read_manifest(src)?;
    let root = plugins_root(app)?;
    let rel = format!("plugins/{}/{}", manifest.name, manifest.version);
    let dest = root.join(&manifest.name).join(&manifest.version);

    if dest.exists() {
        fs::remove_dir_all(&dest).map_err(|e| format!("Failed to replace plugin files: {e}"))?;
    }
    copy_dir_all(src, &dest)?;

    let vynl_meta = manifest.vynl.clone();
    let now = util::now_ms();
    let id = manifest.name.clone();

    mutate_registry(&root, |reg| {
        let existing = reg.plugins.iter().find(|p| p.id == id);
        let entry = PluginEntry {
            id: id.clone(),
            version: manifest.version.clone(),
            path: rel.clone(),
            installation_method: method,
            original_path: original_path
                .clone()
                .or_else(|| existing.and_then(|e| e.original_path.clone())),
            enabled: existing.map(|e| e.enabled).unwrap_or(true),
            installed_at: existing.map(|e| e.installed_at).unwrap_or(now),
            last_updated_at: now,
            display_name: vynl_meta.as_ref().and_then(|m| m.display_name.clone()),
            description: manifest.description.clone(),
            author: manifest.author.clone(),
            categories: clean_categories(vynl_meta.as_ref().and_then(|m| m.categories.clone())),
            source_repo: source_repo
                .clone()
                .or_else(|| existing.and_then(|e| e.source_repo.clone())),
        };
        reg.plugins.retain(|p| p.id != id);
        reg.plugins.push(entry.clone());
        Ok(entry)
    })
}

fn cleanup_staging(root: &Path, staging: &Path) {
    if staging.starts_with(root.join(STAGING_DIR)) {
        let _ = fs::remove_dir_all(staging);
    }
}

pub async fn install_from_url(
    app: AppHandle,
    url: String,
    source_repo: Option<String>,
) -> Result<PluginEntry, String> {
    let parsed =
        reqwest::Url::parse(&url).map_err(|_| "Invalid plugin download URL".to_string())?;
    if parsed.scheme() != "https" {
        return Err("Plugin downloads must use https".into());
    }

    let client = http_client()?.clone();
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to download plugin: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Plugin download failed ({})", resp.status()));
    }
    if let Some(len) = resp.content_length() {
        if len > MAX_ARCHIVE_BYTES {
            return Err("Plugin archive is too large".into());
        }
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read plugin download: {e}"))?;
    if bytes.len() as u64 > MAX_ARCHIVE_BYTES {
        return Err("Plugin archive is too large".into());
    }

    let root = plugins_root(&app)?;
    let staging = root
        .join(STAGING_DIR)
        .join(uuid::Uuid::new_v4().to_string());
    extract_zip_bytes(&bytes, &staging)?;
    let src = locate_plugin_root(&staging)?;
    let result = install_from_dir(
        &app,
        &src,
        PluginInstallationMethod::Store,
        None,
        source_repo,
    );
    cleanup_staging(&root, &staging);
    result
}

pub fn install_from_zip(app: &AppHandle, zip_path: &str) -> Result<PluginEntry, String> {
    let path = Path::new(zip_path);
    if !path.is_absolute() {
        return Err("Path must be absolute".into());
    }
    let meta = fs::metadata(path).map_err(|_| "Plugin archive does not exist".to_string())?;
    if meta.len() > MAX_ARCHIVE_BYTES {
        return Err("Plugin archive is too large".into());
    }
    let bytes = fs::read(path).map_err(|e| format!("Failed to read archive: {e}"))?;

    let root = plugins_root(app)?;
    let staging = root
        .join(STAGING_DIR)
        .join(uuid::Uuid::new_v4().to_string());
    extract_zip_bytes(&bytes, &staging)?;
    let src = locate_plugin_root(&staging)?;
    let result = install_from_dir(app, &src, PluginInstallationMethod::Sideload, None, None);
    cleanup_staging(&root, &staging);
    result
}

pub fn install_from_folder(app: &AppHandle, folder: &str) -> Result<PluginEntry, String> {
    let path = Path::new(folder);
    if !path.is_absolute() {
        return Err("Path must be absolute".into());
    }
    if !path.is_dir() {
        return Err("Plugin folder does not exist".into());
    }
    install_from_dir(
        app,
        path,
        PluginInstallationMethod::Dev,
        Some(folder.to_string()),
        None,
    )
}

pub fn reload_dev(app: &AppHandle, id: &str) -> Result<PluginEntry, String> {
    let entry = get_entry(app, id)?;
    if entry.installation_method != PluginInstallationMethod::Dev {
        return Err("Only locally added plugins can be reloaded".into());
    }
    let original = entry
        .original_path
        .clone()
        .ok_or("This plugin has no source folder to reload from")?;
    install_from_folder(app, &original)
}

pub fn set_enabled(app: &AppHandle, id: &str, enabled: bool) -> Result<PluginEntry, String> {
    let root = plugins_root(app)?;
    mutate_registry(&root, |reg| {
        let entry = reg
            .plugins
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("Plugin '{id}' is not installed"))?;
        entry.enabled = enabled;
        Ok(entry.clone())
    })
}

pub fn remove(app: &AppHandle, id: &str) -> Result<(), String> {
    validate_plugin_id(id)?;
    let root = plugins_root(app)?;
    let app_dir = app_dir(app)?;
    let entry = get_entry(app, id)?;

    let dest = app_dir.join(&entry.path);
    let canonical_root = fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
    if let Ok(canonical_dest) = fs::canonicalize(&dest) {
        if !canonical_dest.starts_with(&canonical_root) {
            return Err("Refusing to remove files outside the plugins directory".into());
        }
        fs::remove_dir_all(&canonical_dest)
            .map_err(|e| format!("Failed to delete plugin files: {e}"))?;
    }

    let _ = fs::remove_file(root.join(CONFIG_DIR).join(format!("{id}.json")));

    mutate_registry(&root, |reg| {
        reg.plugins.retain(|p| p.id != id);
        Ok(())
    })
}

pub fn read_plugin_file(app: &AppHandle, id: &str, rel: &str) -> Result<String, String> {
    let entry = get_entry(app, id)?;
    let base = app_dir(app)?.join(&entry.path);

    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        return Err("Path must be relative".into());
    }
    for comp in rel_path.components() {
        match comp {
            Component::Normal(_) | Component::CurDir => {}
            _ => return Err("Path must stay inside the plugin directory".into()),
        }
    }

    let full = base.join(rel_path);
    if let (Ok(canon_base), Ok(canon_full)) = (fs::canonicalize(&base), fs::canonicalize(&full)) {
        if !canon_full.starts_with(&canon_base) {
            return Err("Path must stay inside the plugin directory".into());
        }
    }
    fs::read_to_string(&full).map_err(|e| format!("Cannot read '{rel}': {e}"))
}

fn config_path(root: &Path, id: &str) -> PathBuf {
    root.join(CONFIG_DIR).join(format!("{id}.json"))
}

pub fn get_config(app: &AppHandle, id: &str) -> Result<serde_json::Value, String> {
    validate_plugin_id(id)?;
    let root = plugins_root(app)?;
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    match fs::read_to_string(config_path(&root, id)) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| format!("Invalid plugin config: {e}")),
        Err(_) => Ok(serde_json::Value::Object(serde_json::Map::new())),
    }
}

pub fn set_config(app: &AppHandle, id: &str, config: serde_json::Value) -> Result<(), String> {
    validate_plugin_id(id)?;
    if !config.is_object() {
        return Err("Plugin config must be an object".into());
    }
    let root = plugins_root(app)?;
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let path = config_path(&root, id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| format!("Failed to save plugin config: {e}"))
}

const AUDIO_EXTS: [&str; 10] = [
    "mp3", "m4a", "aac", "flac", "ogg", "opus", "wav", "webm", "wma", "mka",
];
const COVER_EXTS: [&str; 8] = ["png", "jpg", "jpeg", "webp", "avif", "gif", "bmp", "ico"];

pub fn validate_local_media(app: &AppHandle, path: &str, kind: &str) -> Result<String, String> {
    let canon = Path::new(path)
        .canonicalize()
        .map_err(|_| "file not found".to_string())?;
    let ext = canon
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match kind {
        "audio" if AUDIO_EXTS.contains(&ext.as_str()) => {}
        "cover" if COVER_EXTS.contains(&ext.as_str()) => {}
        "audio" | "cover" => return Err(format!("'.{ext}' is not allowed for {kind}")),
        _ => return Err("kind must be \"audio\" or \"cover\"".into()),
    }

    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(out) = crate::commands::output_dir_from_settings(app) {
        roots.push(out);
    }
    roots.push(crate::services::audio_cache::cache_dir());
    if let Ok(data) = app.path().app_data_dir() {
        roots.push(data);
    }
    if !roots.iter().any(|root| canon.starts_with(root)) {
        return Err("path is outside allowed folders".into());
    }

    let s = canon.to_string_lossy().into_owned();
    match s.strip_prefix(r"\\?\") {
        Some(p) => Ok(p.to_string()),
        None => Ok(s),
    }
}

// ---------------------------------------------------------------------------
// HTTP proxy (CORS-free fetch for plugins)
// ---------------------------------------------------------------------------

pub async fn http_fetch(
    url: String,
    opts: Option<PluginHttpOptions>,
) -> Result<PluginHttpResponse, String> {
    let parsed = reqwest::Url::parse(&url).map_err(|_| "Invalid URL".to_string())?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("Only http(s) URLs are allowed".into());
    }

    let opts = opts.unwrap_or(PluginHttpOptions {
        method: None,
        headers: None,
        body: None,
    });
    let method = opts
        .method
        .unwrap_or_else(|| "GET".to_string())
        .to_uppercase();
    if !["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"].contains(&method.as_str()) {
        return Err(format!("Method '{method}' is not allowed"));
    }

    let client = http_client()?;
    let mut req = client.request(
        reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| format!("Invalid method '{method}'"))?,
        parsed,
    );
    if let Some(headers) = opts.headers {
        for (k, v) in headers {
            req = req.header(k, v);
        }
    }
    if let Some(body) = opts.body {
        req = req.body(body);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;
    let status = resp.status().as_u16();
    let mut headers = std::collections::HashMap::new();
    for (k, v) in resp.headers().iter() {
        if let Ok(val) = v.to_str() {
            headers.insert(k.as_str().to_string(), val.to_string());
        }
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))?;
    if bytes.len() > MAX_HTTP_BODY_BYTES {
        return Err("Response body is too large (max 10MB)".into());
    }
    Ok(PluginHttpResponse {
        status,
        ok: (200..300).contains(&status),
        headers,
        text: String::from_utf8_lossy(&bytes).into_owned(),
    })
}

// ---------------------------------------------------------------------------
// Store catalog
// ---------------------------------------------------------------------------

fn parse_catalog(body: &str) -> Result<Vec<StorePlugin>, String> {
    let trimmed = body.trim();
    let value: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| format!("Invalid catalog JSON: {e}"))?;
    let arr = if let Some(arr) = value.as_array() {
        arr.clone()
    } else if let Some(arr) = value.get("plugins").and_then(|p| p.as_array()) {
        arr.clone()
    } else {
        return Err("Catalog must be a JSON array or an object with a 'plugins' array".into());
    };

    let mut out: Vec<StorePlugin> = Vec::new();
    let mut seen = HashSet::new();
    for item in arr {
        match serde_json::from_value::<StorePlugin>(item) {
            Ok(entry) => {
                if validate_plugin_id(&entry.id).is_err() || !seen.insert(entry.id.clone()) {
                    continue;
                }
                out.push(entry);
            }
            Err(_) => continue,
        }
    }
    Ok(out)
}

pub async fn fetch_store(
    app: &AppHandle,
    sources: Vec<String>,
) -> Result<PluginStoreResult, String> {
    if sources.is_empty() {
        return Err("No plugin store sources configured".into());
    }

    let root = plugins_root(app)?;
    let client = http_client()?.clone();

    let mut merged: Vec<StorePlugin> = Vec::new();
    let mut seen = HashSet::new();
    let mut errors: Vec<String> = Vec::new();
    let mut successes = 0usize;

    for source in &sources {
        let parsed = match reqwest::Url::parse(source) {
            Ok(u) if u.scheme() == "https" => u,
            _ => {
                errors.push("Invalid source URL".to_string());
                continue;
            }
        };
        match client
            .get(parsed)
            .timeout(Duration::from_secs(15))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => match resp.text().await {
                Ok(body) => match parse_catalog(&body) {
                    Ok(entries) => {
                        successes += 1;
                        for e in entries {
                            if seen.insert(e.id.clone()) {
                                merged.push(e);
                            }
                        }
                    }
                    Err(e) => errors.push(e),
                },
                Err(e) => errors.push(format!("Failed to read source: {e}")),
            },
            Ok(resp) => errors.push(format!("Source returned {}", resp.status())),
            Err(e) => errors.push(format!("Failed to fetch source: {e}")),
        }
    }

    if successes > 0 {
        // Cache the merged catalog for offline fallback.
        let cache = PluginStoreResult {
            entries: merged.clone(),
            source_error: None,
        };
        if let Ok(json) = serde_json::to_string_pretty(&cache) {
            let _ = fs::write(root.join(STORE_CACHE_FILE), json);
        }
        return Ok(PluginStoreResult {
            entries: merged,
            source_error: if errors.is_empty() {
                None
            } else {
                Some(errors.join("; "))
            },
        });
    }

    // Everything failed: fall back to the last cached catalog.
    if let Ok(raw) = fs::read_to_string(root.join(STORE_CACHE_FILE)) {
        if let Ok(cached) = serde_json::from_str::<PluginStoreResult>(&raw) {
            return Ok(PluginStoreResult {
                entries: cached.entries,
                source_error: Some(if errors.is_empty() {
                    "Store unreachable, showing cached catalog".into()
                } else {
                    format!("{} (showing cached catalog)", errors.join("; "))
                }),
            });
        }
    }

    Ok(PluginStoreResult {
        entries: Vec::new(),
        source_error: Some(if errors.is_empty() {
            "Store unreachable".into()
        } else {
            errors.join("; ")
        }),
    })
}
