use tauri::{AppHandle, Manager};

use super::types::*;

pub fn clear_pending_update(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<crate::services::update::UpdateState>();
    let mut guard = state.pending.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}

pub fn update_error_status(version: Option<String>, error: &str) -> UpdateStatus {
    UpdateStatus {
        available: version.is_some(),
        version,
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        downloading: Some(false),
        progress: None,
        ready: Some(false),
        error: Some(error.to_string()),
    }
}

#[tauri::command]
pub async fn check_app_update(app: AppHandle) -> Result<UpdateStatus, String> {
    use crate::services::update as update_svc;
    match update_svc::check_for_update(&app).await {
        Ok(Some(update)) => {
            if update.version == env!("CARGO_PKG_VERSION") {
                clear_pending_update(&app)?;
                let status = UpdateStatus {
                    available: false,
                    version: None,
                    current_version: env!("CARGO_PKG_VERSION").to_string(),
                    downloading: None,
                    progress: None,
                    ready: None,
                    error: None,
                };
                update_svc::emit_status(&app, &status);
                return Ok(status);
            }
            let status = UpdateStatus {
                available: true,
                version: Some(update.version.clone()),
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                downloading: None,
                progress: None,
                ready: None,
                error: None,
            };
            {
                let state = app.state::<crate::services::update::UpdateState>();
                let mut guard = state.pending.lock().map_err(|e| e.to_string())?;
                *guard = Some(update);
            }
            update_svc::emit_status(&app, &status);
            Ok(status)
        }
        Ok(None) => {
            clear_pending_update(&app)?;
            let status = UpdateStatus {
                available: false,
                version: None,
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                downloading: None,
                progress: None,
                ready: None,
                error: None,
            };
            update_svc::emit_status(&app, &status);
            Ok(status)
        }
        Err(error) => {
            log::error!("update check failed: {error}");
            let _ = clear_pending_update(&app);
            let status = UpdateStatus {
                available: false,
                version: None,
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                downloading: None,
                progress: None,
                ready: None,
                error: Some(error),
            };
            update_svc::emit_status(&app, &status);
            Ok(status)
        }
    }
}

#[tauri::command]
pub async fn install_app_update(app: AppHandle) -> Result<(), String> {
    use crate::services::update as update_svc;

    let update = {
        let state = app.state::<crate::services::update::UpdateState>();
        let mut guard = state.pending.lock().map_err(|e| e.to_string())?;
        guard.take()
    };

    let update = match update {
        Some(u) => u,
        None => match update_svc::check_for_update(&app).await {
            Ok(Some(u)) => u,
            Ok(None) => {
                let error = "No update available";
                let status = update_error_status(None, error);
                update_svc::emit_status(&app, &status);
                return Err(error.to_string());
            }
            Err(error) => {
                let status = update_error_status(None, &error);
                update_svc::emit_status(&app, &status);
                return Err(error);
            }
        },
    };

    let version = update.version.clone();

    let downloading_status = UpdateStatus {
        available: true,
        version: Some(version.clone()),
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        downloading: Some(true),
        progress: Some(0.0),
        ready: None,
        error: None,
    };
    update_svc::emit_status(&app, &downloading_status);

    if let Err(e) = update_svc::download_and_install(&app, update).await {
        let error_status = UpdateStatus {
            available: true,
            version: Some(version),
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            downloading: Some(false),
            progress: None,
            ready: None,
            error: Some(e.clone()),
        };
        update_svc::emit_status(&app, &error_status);
        return Err(e);
    }

    Ok(())
}
