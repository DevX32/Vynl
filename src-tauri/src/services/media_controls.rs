use crate::commands::types::{MediaControlAction, MediaUpdate};
use std::sync::Mutex;
use std::sync::mpsc::{self, Sender};
use tauri::{AppHandle, Emitter, Manager};

enum MediaCommand {
    Update(Option<MediaUpdate>),
    Shutdown,
}

pub struct MediaController {
    tx: Option<Mutex<Sender<MediaCommand>>>,
}

impl MediaController {
    pub fn update(&self, update: Option<MediaUpdate>) {
        self.send(MediaCommand::Update(update));
    }

    fn send(&self, command: MediaCommand) {
        let Some(tx) = &self.tx else {
            return;
        };
        if let Ok(tx) = tx.lock() {
            let _ = tx.send(command);
        }
    }

    fn disabled() -> Self {
        Self { tx: None }
    }
}

impl Drop for MediaController {
    fn drop(&mut self) {
        self.send(MediaCommand::Shutdown);
    }
}

pub fn start(app: AppHandle) -> MediaController {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        #[cfg(target_os = "windows")]
        let hwnd = app
            .get_webview_window("main")
            .and_then(|window| window.hwnd().ok())
            .map(|handle| handle.0 as usize);
        #[cfg(not(target_os = "windows"))]
        let hwnd: Option<usize> = None;

        let (tx, rx) = mpsc::channel::<MediaCommand>();
        let result = std::thread::Builder::new()
            .name("vynl-media-controls".to_string())
            .spawn(move || {
                #[cfg(target_os = "windows")]
                init_com();

                let mut controls = match create_controls(hwnd) {
                    Ok(controls) => controls,
                    Err(error) => {
                        eprintln!("media controls unavailable: {error}");
                        return;
                    }
                };

                let event_app = app.clone();
                if let Err(error) = controls.attach(move |event| {
                    handle_event(&event_app, event);
                }) {
                    eprintln!("media controls attach failed: {error}");
                    return;
                }

                while let Ok(command) = rx.recv() {
                    match command {
                        MediaCommand::Update(update) => apply_update(&mut controls, update),
                        MediaCommand::Shutdown => break,
                    }
                }
            });

        if let Err(error) = result {
            eprintln!("media controls thread failed: {error}");
            return MediaController::disabled();
        }

        MediaController {
            tx: Some(Mutex::new(tx)),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = app;
        MediaController::disabled()
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn create_controls(hwnd: Option<usize>) -> Result<souvlaki::MediaControls, String> {
    let config = souvlaki::PlatformConfig {
        display_name: "Vynl",
        dbus_name: "vynl",
        hwnd: {
            #[cfg(target_os = "windows")]
            {
                Some(hwnd.ok_or_else(|| "missing window handle".to_string())?
                    as *mut std::ffi::c_void)
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = hwnd;
                None
            }
        },
    };
    souvlaki::MediaControls::new(config).map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
fn init_com() {
    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoInitializeEx(reserved: *mut std::ffi::c_void, co_init: u32) -> i32;
    }

    unsafe {
        CoInitializeEx(std::ptr::null_mut(), 0);
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn build_metadata<'a>(
    update: &'a MediaUpdate,
    album: Option<&'a str>,
    artist: Option<&'a str>,
    cover_url: Option<&'a str>,
    duration: Option<std::time::Duration>,
) -> souvlaki::MediaMetadata<'a> {
    souvlaki::MediaMetadata {
        title: Some(update.title.as_str()),
        album,
        artist,
        cover_url,
        duration,
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn apply_update(controls: &mut souvlaki::MediaControls, update: Option<MediaUpdate>) {
    let Some(update) = update else {
        let _ = controls.set_metadata(cleared_metadata());
        let _ = controls.set_playback(souvlaki::MediaPlayback::Stopped);
        return;
    };

    let cover = update.cover.as_deref().map(cover_url);
    let duration =
        (update.duration > 0.0).then(|| std::time::Duration::from_secs_f64(update.duration));
    let album = if cfg!(target_os = "windows") {
        Some(update.album.as_str())
    } else {
        non_empty(&update.album)
    };
    let artist = if cfg!(target_os = "windows") {
        Some(update.artist.as_str())
    } else {
        non_empty(&update.artist)
    };
    if controls
        .set_metadata(build_metadata(
            &update,
            album,
            artist,
            cover.as_deref(),
            duration,
        ))
        .is_err()
    {
        let _ = controls.set_metadata(build_metadata(&update, album, artist, None, duration));
    }

    let progress = Some(souvlaki::MediaPosition(std::time::Duration::from_secs_f64(
        update.position.max(0.0),
    )));
    let playback = if update.title.trim().is_empty() {
        souvlaki::MediaPlayback::Stopped
    } else if update.playing {
        souvlaki::MediaPlayback::Playing { progress }
    } else {
        souvlaki::MediaPlayback::Paused { progress }
    };
    let _ = controls.set_playback(playback);
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn cleared_metadata() -> souvlaki::MediaMetadata<'static> {
    #[cfg(target_os = "windows")]
    {
        souvlaki::MediaMetadata {
            title: Some(""),
            album: Some(""),
            artist: Some(""),
            cover_url: None,
            duration: None,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        souvlaki::MediaMetadata::default()
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn non_empty(value: &str) -> Option<&str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn cover_url(path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }

    #[cfg(target_os = "windows")]
    {
        let path = path.strip_prefix("file://").unwrap_or(path);
        let mut native = path.replace('/', "\\");
        if !native.starts_with("\\\\") {
            native = native.trim_start_matches('\\').to_string();
        }
        format!("file://{native}")
    }

    #[cfg(not(target_os = "windows"))]
    {
        let normalized = path.replace('\\', "/");
        let path = normalized.strip_prefix("file://").unwrap_or(&normalized);
        format!("file://{path}")
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn signed_offset(direction: souvlaki::SeekDirection, seconds: f64) -> f64 {
    match direction {
        souvlaki::SeekDirection::Forward => seconds,
        souvlaki::SeekDirection::Backward => -seconds,
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn handle_event(app: &AppHandle, event: souvlaki::MediaControlEvent) {
    use souvlaki::MediaControlEvent as Event;

    const SEEK_STEP_SECONDS: f64 = 10.0;

    let action = match event {
        Event::Raise => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            None
        }
        Event::Quit => {
            app.exit(0);
            None
        }
        Event::Play => Some(MediaControlAction::Play),
        Event::Pause => Some(MediaControlAction::Pause),
        Event::Toggle => Some(MediaControlAction::Toggle),
        Event::Next => Some(MediaControlAction::Next),
        Event::Previous => Some(MediaControlAction::Previous),
        Event::Stop => Some(MediaControlAction::Stop),
        Event::Seek(direction) => Some(MediaControlAction::SeekBy {
            offset: signed_offset(direction, SEEK_STEP_SECONDS),
        }),
        Event::SeekBy(direction, duration) => Some(MediaControlAction::SeekBy {
            offset: signed_offset(direction, duration.as_secs_f64()),
        }),
        Event::SetPosition(position) => Some(MediaControlAction::Seek {
            position: position.0.as_secs_f64(),
        }),
        Event::SetVolume(volume) => Some(MediaControlAction::SetVolume { volume }),
        Event::OpenUri(_) => None,
    };

    if let Some(action) = action {
        emit(app, action);
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn emit(app: &AppHandle, action: MediaControlAction) {
    let _ = app.emit("vynl:media-control", action);
}
