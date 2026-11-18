use std::sync::{Mutex, OnceLock};

use tauri::Emitter;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    phase: &'static str,
    done: usize,
    total: usize,
}

static EMITTER: OnceLock<Mutex<Option<tauri::AppHandle>>> = OnceLock::new();

fn emitter() -> &'static Mutex<Option<tauri::AppHandle>> {
    EMITTER.get_or_init(|| Mutex::new(None))
}

pub(crate) fn set_emitter(app: Option<tauri::AppHandle>) {
    if let Ok(mut guard) = emitter().lock() {
        *guard = app;
    }
}

pub(crate) fn emit(phase: &'static str, done: usize, total: usize) {
    let guard = emitter().lock();
    let Ok(guard) = guard else { return };
    let Some(app) = guard.as_ref() else { return };
    let _ = app.emit("vynl:import:progress", ProgressEvent { phase, done, total });
}
