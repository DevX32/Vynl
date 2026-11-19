use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tauri::Emitter;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    phase: &'static str,
    done: usize,
    total: usize,
}

struct EmitterState {
    generation: u64,
    app: tauri::AppHandle,
}

static EMITTER: OnceLock<Mutex<Option<EmitterState>>> = OnceLock::new();
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

fn emitter() -> &'static Mutex<Option<EmitterState>> {
    EMITTER.get_or_init(|| Mutex::new(None))
}

pub(crate) fn set_emitter(app: Option<tauri::AppHandle>) -> u64 {
    let generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut guard) = emitter().lock() {
        match app {
            Some(app) => *guard = Some(EmitterState { generation, app }),
            None => *guard = None,
        }
    }
    generation
}

pub(crate) fn clear_emitter(generation: u64) {
    if let Ok(mut guard) = emitter().lock()
        && guard.as_ref().map(|state| state.generation) == Some(generation)
    {
        guard.take();
    }
}

pub(crate) fn emit(phase: &'static str, done: usize, total: usize) {
    let guard = emitter().lock();
    let Ok(guard) = guard else { return };
    let Some(state) = guard.as_ref() else { return };
    let _ = state
        .app
        .emit("vynl:import:progress", ProgressEvent { phase, done, total });
}
