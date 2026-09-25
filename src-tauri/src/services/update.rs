use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::commands::types::UpdateStatus;

pub struct UpdateState {
    pub pending: Mutex<Option<Update>>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            pending: Mutex::new(None),
        }
    }
}

pub async fn check_for_update(app: &AppHandle) -> Result<Option<Update>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let check = updater.check().await.map_err(|e| e.to_string())?;
    Ok(check)
}

pub async fn emit_status(app: &AppHandle, status: &UpdateStatus) {
    let _ = app.emit("vynl:update:status", status);
}

pub async fn download_and_install(app: &AppHandle, update: Update) -> Result<(), String> {
    let mut downloaded: u64 = 0;
    let mut content_length: u64 = 0;

    update
        .download_and_install(
            |chunk_length, total| {
                downloaded += chunk_length as u64;
                if let Some(t) = total {
                    content_length = t;
                }
                let progress = if content_length > 0 {
                    (downloaded as f64 / content_length as f64) * 100.0
                } else {
                    0.0
                };
                let status = UpdateStatus {
                    available: true,
                    version: None,
                    current_version: env!("CARGO_PKG_VERSION").to_string(),
                    downloading: Some(true),
                    progress: Some(progress),
                    ready: None,
                    error: None,
                };
                let _ = app.emit("vynl:update:status", &status);
            },
            || {
                let status = UpdateStatus {
                    available: true,
                    version: None,
                    current_version: env!("CARGO_PKG_VERSION").to_string(),
                    downloading: Some(false),
                    progress: Some(100.0),
                    ready: Some(true),
                    error: None,
                };
                let _ = app.emit("vynl:update:status", &status);
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
