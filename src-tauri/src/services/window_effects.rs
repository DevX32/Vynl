#[cfg(target_os = "windows")]
static VIBRANCY_FALLBACK_LOGGED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "windows")]
fn note_vibrancy_fallback(layer: &str, error: impl std::fmt::Display) {
    use std::sync::atomic::Ordering;

    // Unsupported vibrancy is expected on some Windows builds; report the
    // first fallback at debug level instead of logging an error every time.
    if !VIBRANCY_FALLBACK_LOGGED.swap(true, Ordering::Relaxed) {
        log::debug!("{layer} unavailable, falling back: {error}");
    }
}

#[cfg(target_os = "windows")]
pub fn apply(window: &tauri::WebviewWindow) {
    use window_vibrancy::{apply_acrylic, apply_blur, apply_mica};

    if let Err(error) = apply_acrylic(window, Some((0, 0, 0, 0))) {
        note_vibrancy_fallback("acrylic", error);
        if let Err(error) = apply_mica(window, Some(true)) {
            note_vibrancy_fallback("mica", error);
            if let Err(error) = apply_blur(window, Some((0, 0, 0, 0))) {
                note_vibrancy_fallback("window blur", error);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn apply(_window: &tauri::WebviewWindow) {}
