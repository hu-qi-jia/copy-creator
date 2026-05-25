mod clipboard;
mod db;
mod paste;
mod shortcut;
mod translator;
mod tray;

use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

#[cfg(target_os = "windows")]
fn apply_backdrop_effect(window: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_WINDOW_CORNER_PREFERENCE};

    let hwnd = window.hwnd().unwrap_or_default();
    if hwnd.is_invalid() {
        return;
    }

    let hwnd = HWND(hwnd.0);

    let backdrop_type: i32 = 3;
    let result = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop_type as *const i32 as *const _,
            std::mem::size_of::<i32>() as u32,
        )
    };

    if let Err(e) = result {
        log::warn!("Failed to set DWM backdrop type: {:?}", e);
    }

    let corner_preference: i32 = 2; // DWMWCP_ROUND
    let result = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corner_preference as *const i32 as *const _,
            std::mem::size_of::<i32>() as u32,
        )
    };

    if let Err(e) = result {
        log::warn!("Failed to set DWM corner preference: {:?}", e);
    }
}

/// macOS: apply vibrancy (frosted glass) effect using NSVisualEffectView.
#[cfg(target_os = "macos")]
fn apply_vibrancy_effect(window: &tauri::WebviewWindow) {
    use objc::runtime::{Class, Object};
    use objc::{msg_send, sel, sel_impl};
    use cocoa::base::id;

    unsafe {
        // Get the native NSWindow handle via Tauri's ns_window() method
        let ns_window_raw = match window.ns_window() {
            Ok(h) => h as *mut Object,
            Err(e) => {
                log::warn!("apply_vibrancy_effect: failed to get ns_window: {}", e);
                return;
            }
        };

        // Get the window's content view
        let content_view: id = msg_send![ns_window_raw, contentView];
        if content_view.is_null() {
            log::warn!("apply_vibrancy_effect: content view is null");
            return;
        }

        // Create NSVisualEffectView
        let ns_vev_class = Class::get("NSVisualEffectView").unwrap();
        let frame: id = msg_send![content_view, bounds];
        let effect_view: id = msg_send![ns_vev_class, alloc];
        let effect_view: id = msg_send![effect_view, initWithFrame: frame];

        // NSVisualEffectBlendingModeBehindWindow = 0
        let _: () = msg_send![effect_view, setBlendingMode: 0usize];
        // NSVisualEffectMaterialHudWindow = 23 (suitable for floating tool windows)
        let _: () = msg_send![effect_view, setMaterial: 23usize];
        // NSVisualEffectStateActive = 1
        let _: () = msg_send![effect_view, setState: 1usize];
        // Auto-resize with superview (width + height flexible)
        let _: () = msg_send![effect_view, setAutoresizingMask: 18u64];

        // Make the effect view fill the content view and send it to back
        let _: () = msg_send![content_view, addSubview: effect_view positioned: -1i64 relativeTo: std::ptr::null::<Object>()];

        log::info!("apply_vibrancy_effect: NSVisualEffectView applied");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        shortcut::toggle_window(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            #[cfg(target_os = "windows")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
                    apply_backdrop_effect(&window);
                }
            }

            #[cfg(target_os = "macos")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
                    apply_vibrancy_effect(&window);
                }
            }

            let is_autostart = std::env::args().any(|a| a == "--hidden");

            db::init_db(app.handle())?;
            db::prune_old_records(app.handle()).ok();

            // Always start with light theme
            let _ = db::set_setting(app.handle().clone(), "theme".to_string(), "light".to_string());

            // Repair autostart registry entry to ensure --hidden arg is present
            let autostart = app.autolaunch();
            if autostart.is_enabled().unwrap_or(false) {
                let _ = autostart.enable();
            }

            // Periodic pruning every hour
            let prune_handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
                db::prune_old_records(&prune_handle).ok();
            });

            clipboard::start_monitor(app.handle())?;

            app.handle().manage(tray::TrayState { tray: std::sync::Mutex::new(None) });
            tray::create_tray(app.handle())?;

            shortcut::install_mouse_hook(app.handle());

            // Configure radial menu popup window (defined in tauri.conf.json)
            {
                let radial = app.get_webview_window("radial-menu").unwrap();
                let _ = radial.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
                #[cfg(target_os = "windows")]
                apply_backdrop_effect(&radial);
                #[cfg(target_os = "macos")]
                apply_vibrancy_effect(&radial);
                log::info!("Radial menu popup window created");
            }

            if let Ok(key) = db::get_setting(app.handle().clone(), "shortcut_key".to_string()) {
                if !key.is_empty() {
                    if let Err(e) = shortcut::register_keyboard_shortcut(app.handle(), &key) {
                        log::warn!("Failed to register keyboard shortcut '{}': {}", key, e);
                    }
                }
            }

            // Show main window when not auto-started (after all init is done)
            if !is_autostart {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            db::get_clipboard_records,
            db::delete_clipboard_record,
            db::get_phrase_groups,
            db::create_phrase_group,
            db::update_phrase_group,
            db::delete_phrase_group,
            db::get_phrases,
            db::create_phrase,
            db::update_phrase,
            db::delete_phrase,
            db::get_translation_history,
            db::clear_translation_history,
            db::get_setting,
            db::get_all_settings,
            db::set_setting,
            db::set_settings_batch,
            paste::paste_text,
            paste::paste_image,
            paste::paste_file,
            db::get_image_base64,
            db::get_image_thumbnail,
            db::ensure_thumbnail,
            db::get_storage_path,
            db::select_storage_folder,
            translator::translate,
            shortcut::update_shortcut,
            shortcut::set_radial_menu_enabled,
            tray::update_tray_language,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
