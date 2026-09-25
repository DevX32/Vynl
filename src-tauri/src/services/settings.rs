use std::fs;
use std::path::Path;

use crate::commands::types::Settings;
use crate::services::util;

const DEFAULT_ACCENT_COLOR: &str = "#a894e8";

crate::declare_file_mutex!();

fn is_valid_hex_color(s: &str) -> bool {
    let hex = match s.strip_prefix('#') {
        Some(rest) => rest,
        None => return false,
    };
    matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

fn defaults() -> Settings {
    let music = dirs::audio_dir().unwrap_or_default();
    let dir = if music.exists() {
        let vynl = music.join("Vynl");
        let _ = fs::create_dir_all(&vynl);
        vynl
    } else {
        let _ = fs::create_dir_all(&music);
        let vynl = music.join("Vynl");
        let _ = fs::create_dir_all(&vynl);
        vynl
    };
    let dir = dir.canonicalize().unwrap_or(dir);
    Settings {
        output_dir: dir.to_string_lossy().to_string(),
        format: crate::commands::types::AudioFormat::Mp3,
        bitrate: None,
        filename_pattern: "{track} - {title}".into(),
        overwrite: false,
        discord_rpc: true,
        confirm_matches: false,
        minimize_to_tray: false,
        accent_color: DEFAULT_ACCENT_COLOR.into(),
        dynamic_accent: false,
        hardware_acceleration: true,
        launch_at_startup: false,
        display_name: String::new(),
        eq_enabled: false,
        eq_bands: crate::services::player::normalize_eq_bands(&[]),
        plugins_auto_update: true,
    }
}

fn settings_file(user_data_dir: &Path) -> std::path::PathBuf {
    user_data_dir.join("settings.json")
}

pub fn get_settings(user_data_dir: &Path) -> Settings {
    let _lock = file_mutex().lock().unwrap_or_else(|e| e.into_inner());
    get_settings_unlocked(user_data_dir)
}

pub fn set_settings(user_data_dir: &Path, patch: Settings) -> Result<Settings, String> {
    let _lock = file_mutex().lock().map_err(|e| e.to_string())?;
    let current = get_settings_unlocked(user_data_dir);
    let is_lossless = patch.format == crate::commands::types::AudioFormat::Flac
        || patch.format == crate::commands::types::AudioFormat::Wav;

    let output_dir = if patch.output_dir.is_empty() {
        current.output_dir
    } else {
        let abs = std::path::PathBuf::from(&patch.output_dir);
        if !abs.is_absolute() {
            return Err("output directory must be an absolute path".into());
        }
        let canonical = fs::canonicalize(&abs).unwrap_or(abs);
        canonical.to_string_lossy().to_string()
    };

    let next = Settings {
        output_dir,
        format: patch.format,
        bitrate: if is_lossless {
            None
        } else {
            patch.bitrate.or(current.bitrate)
        },
        filename_pattern: if patch.filename_pattern.is_empty() {
            current.filename_pattern
        } else {
            patch.filename_pattern
        },
        overwrite: patch.overwrite,
        discord_rpc: patch.discord_rpc,
        confirm_matches: patch.confirm_matches,
        minimize_to_tray: patch.minimize_to_tray,
        accent_color: if is_valid_hex_color(&patch.accent_color) {
            patch.accent_color
        } else {
            current.accent_color
        },
        dynamic_accent: patch.dynamic_accent,
        hardware_acceleration: patch.hardware_acceleration,
        launch_at_startup: patch.launch_at_startup,
        display_name: patch.display_name,
        eq_enabled: patch.eq_enabled,
        eq_bands: crate::services::player::normalize_eq_bands(&patch.eq_bands),
        plugins_auto_update: patch.plugins_auto_update,
    };

    util::write_json(&settings_file(user_data_dir), &next)?;
    Ok(next)
}

/// Reads settings while the caller already holds the file mutex.
fn get_settings_unlocked(user_data_dir: &Path) -> Settings {
    let Some(mut loaded) = util::read_json::<Settings>(&settings_file(user_data_dir)) else {
        return defaults();
    };
    let def = defaults();
    if loaded.output_dir.is_empty() {
        loaded.output_dir = def.output_dir;
    }
    if loaded.filename_pattern.is_empty() {
        loaded.filename_pattern = def.filename_pattern;
    }
    if !is_valid_hex_color(&loaded.accent_color) {
        loaded.accent_color = def.accent_color;
    }
    loaded.eq_bands = crate::services::player::normalize_eq_bands(&loaded.eq_bands);
    loaded
}
