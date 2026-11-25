pub mod catalogue;
mod commands;
pub mod services;

use tauri::Manager;

#[cfg(target_os = "linux")]
mod gtk_log_filter {
    use std::ffi::CStr;
    use std::os::raw::{c_char, c_uint, c_void};

    type GLogFunc = extern "C" fn(*const c_char, c_uint, *const c_char, *mut c_void);

    unsafe extern "C" {
        fn g_log_set_handler(
            log_domain: *const c_char,
            log_levels: c_uint,
            log_func: GLogFunc,
            user_data: *mut c_void,
        ) -> c_uint;

        fn g_log_default_handler(
            log_domain: *const c_char,
            log_level: c_uint,
            message: *const c_char,
            user_data: *mut c_void,
        );
    }

    const G_LOG_LEVEL_CRITICAL: c_uint = 1 << 3;
    const G_LOG_FLAG_RECURSION: c_uint = 1 << 0;
    const G_LOG_FLAG_FATAL: c_uint = 1 << 1;

    extern "C" fn handler(
        log_domain: *const c_char,
        log_level: c_uint,
        message: *const c_char,
        user_data: *mut c_void,
    ) {
        if message.is_null() {
            return;
        }
        let text = unsafe { CStr::from_ptr(message) }.to_string_lossy();
        if text.contains("gtk_widget_get_scale_factor") && text.contains("GTK_IS_WIDGET") {
            return;
        }
        unsafe {
            g_log_default_handler(log_domain, log_level, message, user_data);
        }
    }

    pub fn install() {
        let domain = std::ffi::CString::new("Gtk").unwrap();
        unsafe {
            g_log_set_handler(
                domain.as_ptr(),
                G_LOG_LEVEL_CRITICAL | G_LOG_FLAG_RECURSION | G_LOG_FLAG_FATAL,
                handler,
                std::ptr::null_mut(),
            );
        }
    }
}

#[cfg(target_os = "windows")]
const APP_USER_MODEL_ID: &str = "vynl";

#[cfg(target_os = "windows")]
fn register_aumid() {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    unsafe extern "system" {
        fn SetCurrentProcessExplicitAppUserModelID(app_id: *const u16) -> i32;
    }

    let wide: Vec<u16> = OsStr::new(APP_USER_MODEL_ID)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        SetCurrentProcessExplicitAppUserModelID(wide.as_ptr());
    }

    {
        use winreg::RegKey;
        use winreg::enums::*;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok((key, _)) = hkcu.create_subkey_with_flags(
            format!("Software\\Classes\\AppUserModelId\\{APP_USER_MODEL_ID}"),
            KEY_ALL_ACCESS | KEY_CREATE_SUB_KEY,
        ) {
            let _ = key.set_value("DisplayName", &"Vynl".to_string());
            if let Some(dir) = dirs::data_dir() {
                let dir = dir.join("vynl");
                if std::fs::create_dir_all(&dir).is_ok() {
                    let icon = dir.join("vynl.ico");
                    if std::fs::write(&icon, include_bytes!("../icons/icon.ico")).is_ok() {
                        let _ = key.set_value("IconUri", &icon.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
}

pub fn sync_launch_at_startup(app: &tauri::AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;

    let autolaunch = app.autolaunch();
    let result = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    if let Err(error) = result {
        eprintln!("could not update autostart: {error}");
    }
}

#[cfg(target_os = "windows")]
fn apply_window_effects(window: &tauri::WebviewWindow) {
    use window_vibrancy::{apply_acrylic, apply_blur, apply_mica};

    if let Err(error) = apply_acrylic(window, Some((0, 0, 0, 0))) {
        eprintln!("acrylic unavailable: {error}");
        if let Err(error) = apply_mica(window, Some(true)) {
            eprintln!("mica unavailable: {error}");
            if let Err(error) = apply_blur(window, Some((0, 0, 0, 0))) {
                eprintln!("window blur unavailable: {error}");
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn apply_window_effects(_window: &tauri::WebviewWindow) {}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn register_global_media_keys(app: tauri::AppHandle) {
    use crate::commands::types::MediaControlAction;
    use tauri::Emitter;
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let shortcuts: &[(&str, MediaControlAction)] = &[
        ("MediaPlayPause", MediaControlAction::Toggle),
        ("MediaPlay", MediaControlAction::Play),
        ("MediaPause", MediaControlAction::Pause),
        ("MediaStop", MediaControlAction::Stop),
        ("MediaTrackNext", MediaControlAction::Next),
        ("MediaTrackPrevious", MediaControlAction::Previous),
        ("AudioVolumeUp", MediaControlAction::VolumeUp),
        ("AudioVolumeDown", MediaControlAction::VolumeDown),
        ("AudioVolumeMute", MediaControlAction::Mute),
    ];

    let gs = app.global_shortcut();
    for (accel, action) in shortcuts {
        let app_handle = app.clone();
        let action = action.clone();
        let accel_str = *accel;
        if let Err(error) = gs.on_shortcut(accel_str, move |_app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            let _ = app_handle.emit("vynl:media-control", action.clone());
        }) {
            eprintln!("[vynl] global shortcut {accel_str} unavailable: {error}");
        }
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn unregister_global_media_keys(app: tauri::AppHandle) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let _ = app.global_shortcut().unregister_all();
}

pub fn run() {
    #[cfg(target_os = "windows")]
    register_aumid();

    #[cfg(target_os = "linux")]
    gtk_log_filter::install();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
            let state = app.state::<commands::AppState>();
            let mut pending = state
                .pending_confirm
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(url) = _argv.iter().find(|arg| arg.starts_with("vynl://")) {
                *pending = Some(url.clone());
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            app.manage(commands::AppState::default());
            {
                let state = app.state::<commands::AppState>();
                let mut pending = state
                    .pending_confirm
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                if let Some(url) = std::env::args().find(|arg| arg.starts_with("vynl://")) {
                    *pending = Some(url);
                }
            }
            #[cfg(any(target_os = "linux", all(debug_assertions, target_os = "windows")))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if let Err(error) = app.deep_link().register_all() {
                    eprintln!("could not register deep link schemes: {error}");
                }
            }

            app.manage(crate::services::update::UpdateState::default());

            app.manage(crate::services::media_controls::start(app.handle().clone()));

            {
                let app_handle = app.handle().clone();
                let _ = ctrlc::set_handler(move || {
                    if let Some(win) = app_handle.get_webview_window("main") {
                        let _ = win.destroy();
                    }
                    if let Ok(rt) = tokio::runtime::Runtime::new() {
                        rt.block_on(crate::services::downloader::cancel_download());
                    }
                    app_handle.exit(0);
                });
            }

            let user_data = app.path().app_data_dir().unwrap_or_default();
            let settings = crate::services::settings::get_settings(&user_data);
            {
                let allow = |path: &std::path::Path| {
                    if let Err(error) = app.asset_protocol_scope().allow_directory(path, true) {
                        eprintln!(
                            "could not allow asset access to {}: {error}",
                            path.display()
                        );
                    }
                };
                allow(&user_data);
                let output = std::path::PathBuf::from(&settings.output_dir);
                if output.is_dir() {
                    allow(&output);
                }
                if let Some(cache) = dirs::cache_dir() {
                    allow(&cache);
                }
            }

            if let Some(win) = app.get_webview_window("main")
                && let Ok(icon) =
                    tauri::image::Image::from_bytes(include_bytes!("../icons/icon.ico"))
            {
                let _ = win.set_icon(icon);
            }

            if let Some(win) = app.get_webview_window("main") {
                apply_window_effects(&win);
                let fallback = win.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    let _ = fallback.show();
                });
            }

            crate::services::tools::remove_unused_binaries(&user_data);

            let catalogued = crate::services::catalogue::init(&user_data);
            if let Err(error) = &catalogued {
                eprintln!("catalogue unavailable: {error}");
            }

            sync_launch_at_startup(app.handle(), settings.launch_at_startup);

            {
                let ud = user_data.clone();
                let out = std::path::PathBuf::from(&settings.output_dir);
                let st = settings.clone();
                tauri::async_runtime::spawn(async move {
                    if crate::services::sync::status(&ud).signed_in {
                        match crate::services::sync::pull_sync_state(&ud, &out).await {
                            Ok(_) => {}
                            Err(e) if e.contains("No cloud sync state found") => {}
                            Err(e) => eprintln!("[Vynl sync] startup pull failed: {e}"),
                        }
                    }
                    if crate::services::sync::is_sync_dirty(&ud)
                        && let Err(error) =
                            crate::services::sync::push_sync_state(&ud, &out, &st).await
                    {
                        eprintln!("[Vynl sync] retry of pending push failed: {error}");
                    }
                });
            }

            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            if settings.global_media_keys {
                register_global_media_keys(app.handle().clone());
            }
            crate::services::player::set_app_handle(app.handle());
            crate::services::player::set_eq(settings.eq_enabled, &settings.eq_bands);

            {
                let ud = user_data.clone();
                tauri::async_runtime::spawn(async move {
                    crate::services::mobile_sync::ensure_started_on_boot(&ud).await;
                });
            }

            tauri::async_runtime::spawn(async move {
                let _ = crate::services::internal::warm_session().await;
            });

            {
                let state = app.state::<commands::AppState>();
                let mut guard = state.settings.lock().unwrap_or_else(|e| e.into_inner());
                *guard = Some(settings);
            }

            let show_item = tauri::menu::MenuBuilder::new(app)
                .item(
                    &tauri::menu::MenuItemBuilder::new("Show Vynl")
                        .id("show")
                        .build(app)?,
                )
                .item(
                    &tauri::menu::MenuItemBuilder::new("Quit")
                        .id("quit")
                        .build(app)?,
                )
                .build()?;

            let icon = app.default_window_icon().cloned();
            let mut tray_builder = tauri::tray::TrayIconBuilder::new();
            if let Some(icon) = icon {
                tray_builder = tray_builder.icon(icon);
            }
            let _tray = tray_builder
                .menu(&show_item)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "quit" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.destroy();
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
            {
                let app = window.app_handle();
                let should_minimize = {
                    let state = app.state::<commands::AppState>();
                    let guard = state.settings.lock().unwrap_or_else(|e| e.into_inner());
                    guard.as_ref().map(|s| s.minimize_to_tray).unwrap_or(false)
                };
                if should_minimize {
                    let _ = window.hide();
                    api.prevent_close();
                } else {
                    app.exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::resolve,
            commands::search_tracks,
            crate::services::catalogue::catalogue_search,
            crate::services::catalogue::catalogue_cover,
            crate::services::catalogue::catalogue_album,
            crate::services::catalogue::catalogue_artist,
            crate::services::catalogue::catalogue_playlist,
            crate::services::catalogue::catalogue_playlist_page,
            crate::services::catalogue::catalogue_prebuffer,
            crate::services::catalogue::catalogue_discard_prebuffer,
            crate::services::catalogue::catalogue_stream,
            crate::services::catalogue::catalogue_crossfade_stream,
            crate::services::catalogue::catalogue_seek_stream,
            crate::services::catalogue::catalogue_add_to_playlist,
            crate::services::catalogue::catalogue_save_to_playlist,
            crate::services::catalogue::catalogue_taste,
            crate::services::importer::import_stream_playlist,
            commands::cancel_stream_import,
            commands::downloaded_paths,
            commands::start_download,
            commands::cancel_download,
            commands::export_backup,
            commands::import_backup,
            commands::matches,
            commands::sync_done,
            commands::get_settings,
            commands::set_settings,
            commands::get_eq_design,
            commands::get_mobile_sync_status,
            commands::set_mobile_sync_enabled,
            commands::rotate_mobile_sync_pin,
            commands::taste_load,
            commands::taste_save,
            commands::get_tools,
            commands::check_tool_updates,
            commands::install_tool,
            commands::delete_library_track,
            commands::delete_all_library_tracks,
            commands::get_library,
            commands::list_playlists,
            commands::get_playlist,
            commands::create_playlist,
            commands::reorder_playlists,
            commands::rename_playlist,
            commands::delete_playlist,
            commands::add_to_playlist,
            commands::remove_from_playlist,
            commands::move_in_playlist,
            commands::set_playlist_cover,
            commands::save_playlist_cover,
            commands::sync_status,
            commands::sync_pull,
            commands::sync_push,
            commands::sync_now,
            commands::sync_mark_dirty,
            commands::sync_resolve_missing_source,
            commands::sync_sign_in,
            commands::sync_sign_up,
            commands::sync_sign_out,
            commands::sync_confirm_email,
            commands::take_pending_confirm_url,
            commands::lyrics_local,
            commands::lyrics_fetch,
            commands::lyrics_embed,
            commands::lyrics_save,
            commands::lyrics_search,
            commands::rpc_update,
            commands::media_update,
            commands::save_now_playing,
            commands::load_now_playing,
            commands::clear_now_playing,
            commands::check_app_update,
            commands::install_app_update,
            commands::restart_app,
            commands::player_play,
            commands::player_crossfade,
            commands::player_stop,
            commands::player_pause,
            commands::player_resume,
            commands::player_seek,
            commands::player_set_volume,
            commands::player_set_speed,
            commands::player_set_replay_gain,
            commands::library_read_tags,
            commands::library_write_tags,
            commands::player_get_position,
            commands::player_is_playing,
            commands::cache_remote_audio,
            commands::clear_audio_cache,
            commands::clear_cache,
            commands::downloads_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
