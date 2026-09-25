#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    apply_hardware_acceleration();
    vynl_lib::run()
}

fn apply_hardware_acceleration() {
    let base = match dirs::data_dir() {
        Some(d) => d,
        None => return,
    };

    let paths = [
        base.join("dev.vynl").join("settings.json"),
        base.join("Vynl").join("settings.json"),
        base.join("vynl").join("settings.json"),
    ];

    for path in &paths {
        if !path.exists() {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };
        if val.get("hardwareAcceleration").and_then(|v| v.as_bool()) == Some(false) {
            #[cfg(target_os = "windows")]
            std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGS", "--disable-gpu");
            #[cfg(target_os = "linux")]
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        } else {
            #[cfg(target_os = "windows")]
            std::env::remove_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGS");
            #[cfg(target_os = "linux")]
            std::env::remove_var("WEBKIT_DISABLE_COMPOSITING_MODE");
        }
        return;
    }
}
