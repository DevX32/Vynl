mod commands;
pub mod services;

use tauri::Manager;

#[cfg(target_os = "linux")]
mod gtk_log_filter {
    use std::ffi::CStr;
    use std::os::raw::{c_char, c_uint, c_void};

    type GLogFunc = extern "C" fn(*const c_char, c_uint, *const c_char, *mut c_void);

    extern "C" {
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

    extern "system" {
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
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok((key, _)) = hkcu.create_subkey_with_flags(
            format!("Software\\Classes\\AppUserModelId\\{APP_USER_MODEL_ID}"),
            KEY_ALL_ACCESS | KEY_CREATE_SUB_KEY,
        ) {
            let _ = key.set_value("DisplayName", &"Vynl".to_string());
        }
    }
}

#[cfg(target_os = "windows")]
pub fn sync_launch_at_startup(enabled: bool) {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        KEY_ALL_ACCESS,
    );
    if let Ok(key) = run_key {
        if enabled {
            if let Ok(exe) = std::env::current_exe() {
                let _ = key.set_value("Vynl", &format!("\"{}\"", exe.to_string_lossy()));
            }
        } else {
            let _ = key.delete_value("Vynl");
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn sync_launch_at_startup(_enabled: bool) {}

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
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            app.manage(commands::AppState::default());
            app.manage(crate::services::update::UpdateState::default());

            {
                let app_handle = app.handle().clone();
                let _ = ctrlc::set_handler(move || {
                    if let Some(win) = app_handle.get_webview_window("main") {
                        let _ = win.destroy();
                    }
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(crate::services::downloader::cancel_download());
                    app_handle.exit(0);
                });
            }

            let user_data = app.path().app_data_dir().unwrap_or_default();
            let settings = crate::services::settings::get_settings(&user_data);

            sync_launch_at_startup(settings.launch_at_startup);
            crate::services::player::set_app_handle(app.handle());
            crate::services::player::set_eq(settings.eq_enabled, &settings.eq_bands);

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
                .tooltip("Vynl")
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
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let app = window.app_handle();
                    let should_minimize = {
                        let state = app.state::<commands::AppState>();
                        let guard = state.settings.lock().unwrap_or_else(|e| e.into_inner());
                        guard.as_ref().map(|s| s.minimize_to_tray).unwrap_or(false)
                    };
                    if should_minimize {
                        let _ = window.hide();
                        api.prevent_close();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::resolve,
            commands::search_spotify,
            commands::downloaded_paths,
            commands::start_download,
            commands::cancel_download,
            commands::matches,
            commands::sync_done,
            commands::get_settings,
            commands::set_settings,
            commands::get_tools,
            commands::check_tool_updates,
            commands::install_tool,
            commands::update_tool,
            commands::delete_library_track,
            commands::get_library,
            commands::list_playlists,
            commands::get_playlist,
            commands::create_playlist,
            commands::rename_playlist,
            commands::delete_playlist,
            commands::add_to_playlist,
            commands::remove_from_playlist,
            commands::move_in_playlist,
            commands::set_playlist_cover,
            commands::save_playlist_cover,
            commands::lyrics_local,
            commands::lyrics_fetch,
            commands::lyrics_embed,
            commands::lyrics_search,
            commands::lyrics_export,
            commands::rpc_update,
            commands::save_now_playing,
            commands::load_now_playing,
            commands::clear_now_playing,
            commands::check_app_update,
            commands::install_app_update,
            commands::restart_app,
            commands::player_play,
            commands::player_stop,
            commands::player_pause,
            commands::player_resume,
            commands::player_seek,
            commands::player_set_volume,
            commands::player_get_position,
            commands::player_is_playing,
            commands::player_check_finished,
            commands::cache_remote_audio,
            commands::clear_audio_cache,
            commands::artist::fetch_artist_info,
            commands::plugins::plugins_list,
            commands::plugins::plugins_set_enabled,
            commands::plugins::plugins_remove,
            commands::plugins::plugins_install_from_url,
            commands::plugins::plugins_pick_folder,
            commands::plugins::plugins_install_from_folder,
            commands::plugins::plugins_pick_zip,
            commands::plugins::plugins_install_from_zip,
            commands::plugins::plugins_reload_dev,
            commands::plugins::plugins_read_file,
            commands::plugins::plugin_get_config,
            commands::plugins::plugin_set_config,
            commands::plugins::plugin_http_fetch,
            commands::plugins::plugin_store_fetch,
            commands::plugins::plugin_validate_local_media,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
