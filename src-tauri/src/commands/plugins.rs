use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use super::types::*;

#[tauri::command]
pub async fn plugins_list(app: AppHandle) -> Result<Vec<PluginEntry>, String> {
    crate::services::plugins::list(&app)
}

#[tauri::command]
pub async fn plugins_set_enabled(
    id: String,
    enabled: bool,
    app: AppHandle,
) -> Result<PluginEntry, String> {
    crate::services::plugins::set_enabled(&app, &id, enabled)
}

#[tauri::command]
pub async fn plugins_remove(id: String, app: AppHandle) -> Result<(), String> {
    crate::services::plugins::remove(&app, &id)
}

#[tauri::command]
pub async fn plugins_install_from_url(
    url: String,
    repo: Option<String>,
    app: AppHandle,
) -> Result<PluginEntry, String> {
    crate::services::plugins::install_from_url(app, url, repo).await
}

#[tauri::command]
pub async fn plugins_pick_folder(app: AppHandle) -> Result<Option<String>, String> {
    let result = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Select plugin folder")
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(result.map(|p| p.to_string()))
}

#[tauri::command]
pub async fn plugins_install_from_folder(
    path: String,
    app: AppHandle,
) -> Result<PluginEntry, String> {
    crate::services::plugins::install_from_folder(&app, &path)
}

#[tauri::command]
pub async fn plugins_pick_zip(app: AppHandle) -> Result<Option<String>, String> {
    let result = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Select plugin archive")
            .add_filter("Plugin archive", &["zip"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(result.map(|p| p.to_string()))
}

#[tauri::command]
pub async fn plugins_install_from_zip(path: String, app: AppHandle) -> Result<PluginEntry, String> {
    crate::services::plugins::install_from_zip(&app, &path)
}

#[tauri::command]
pub async fn plugins_reload_dev(id: String, app: AppHandle) -> Result<PluginEntry, String> {
    crate::services::plugins::reload_dev(&app, &id)
}

#[tauri::command]
pub async fn plugins_read_file(id: String, path: String, app: AppHandle) -> Result<String, String> {
    crate::services::plugins::read_plugin_file(&app, &id, &path)
}

#[tauri::command]
pub async fn plugin_get_config(id: String, app: AppHandle) -> Result<serde_json::Value, String> {
    crate::services::plugins::get_config(&app, &id)
}

#[tauri::command]
pub async fn plugin_set_config(
    id: String,
    config: serde_json::Value,
    app: AppHandle,
) -> Result<(), String> {
    crate::services::plugins::set_config(&app, &id, config)
}

#[tauri::command]
pub async fn plugin_http_fetch(
    url: String,
    opts: Option<PluginHttpOptions>,
) -> Result<PluginHttpResponse, String> {
    crate::services::plugins::http_fetch(url, opts).await
}

#[tauri::command]
pub async fn plugin_store_fetch(
    sources: Vec<String>,
    app: AppHandle,
) -> Result<PluginStoreResult, String> {
    crate::services::plugins::fetch_store(&app, sources).await
}

#[tauri::command]
pub async fn plugin_validate_local_media(
    path: String,
    kind: String,
    app: AppHandle,
) -> Result<String, String> {
    crate::services::plugins::validate_local_media(&app, &path, &kind)
}
