use tauri::AppHandle;

use super::{load_settings, user_data_dir};

#[tauri::command]
pub async fn export_backup(
    app: AppHandle,
) -> Result<Option<crate::services::export::ExportSummary>, String> {
    use tauri_plugin_dialog::DialogExt;

    let user_data = user_data_dir(&app)?;
    let settings = load_settings(&app)?;

    let (default_name, content, track_count, playlist_count) =
        tokio::task::spawn_blocking(move || {
            let backup = crate::services::export::build_backup(&settings, &user_data);
            let filename = crate::services::export::suggested_filename(&backup);
            let content = crate::services::export::render(&backup)?;
            Ok::<_, String>((filename, content, backup.track_count, backup.playlist_count))
        })
        .await
        .map_err(|e| format!("Export task failed: {e}"))??;

    let dialog_app = app.clone();
    let ext = crate::services::export::EXPORT_EXTENSION;
    let label = crate::services::export::EXPORT_LABEL;
    let picked = tokio::task::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .set_title("Export library backup")
            .set_file_name(&default_name)
            .add_filter(label, &[ext])
            .blocking_save_file()
    })
    .await
    .map_err(|e| format!("Export task failed: {e}"))?;

    let Some(picked) = picked else {
        return Ok(None);
    };

    let path = match picked {
        tauri_plugin_dialog::FilePath::Path(p) => p,
        tauri_plugin_dialog::FilePath::Url(u) => u
            .to_file_path()
            .map_err(|_| "Could not resolve the chosen location.".to_string())?,
    };
    let path_string = path.to_string_lossy().to_string();

    let bytes =
        tokio::task::spawn_blocking(move || crate::services::export::write_backup(&content, &path))
            .await
            .map_err(|e| format!("Export task failed: {e}"))??;

    Ok(Some(crate::services::export::ExportSummary {
        path: path_string,
        format: crate::services::export::EXPORT_EXTENSION.to_string(),
        track_count,
        playlist_count,
        bytes,
    }))
}

#[tauri::command]
pub async fn import_backup(
    app: AppHandle,
) -> Result<Option<crate::services::export::RestoredBackup>, String> {
    use tauri_plugin_dialog::DialogExt;

    let dialog_app = app.clone();
    let picked = tokio::task::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .set_title("Open Vynl backup")
            .add_filter("Vynl backup", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| format!("Import task failed: {e}"))?;

    let Some(picked) = picked else {
        return Ok(None);
    };

    let path = match picked {
        tauri_plugin_dialog::FilePath::Path(p) => p,
        tauri_plugin_dialog::FilePath::Url(u) => u
            .to_file_path()
            .map_err(|_| "Could not resolve the chosen file.".to_string())?,
    };

    let restored = tokio::task::spawn_blocking(move || {
        let backup = crate::services::export::read_backup_file(&path)?;
        Ok::<_, String>(crate::services::export::restore(&backup))
    })
    .await
    .map_err(|e| format!("Import task failed: {e}"))??;

    if restored.total == 0 {
        return Err("That backup doesn't contain any tracks.".into());
    }

    Ok(Some(restored))
}
