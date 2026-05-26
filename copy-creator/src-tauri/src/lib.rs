mod clipboard;
mod db;
mod paste;
mod shortcut;
mod translator;
mod tray;

use std::str::FromStr;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::Shortcut as GShortcut;

/// macOS: set app activation policy to show/hide from Dock at runtime.
#[cfg(target_os = "macos")]
fn set_activation_policy(accessory: bool) {
    use objc::runtime::{Class, Object};
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let ns_app_class = Class::get("NSApplication").unwrap();
        let app: *mut Object = msg_send![ns_app_class, sharedApplication];
        // NSApplicationActivationPolicyAccessory = 1, Regular = 0
        let policy: usize = if accessory { 1 } else { 0 };
        let _: bool = msg_send![app, setActivationPolicy: policy];
    }
}

#[cfg(not(target_os = "macos"))]
fn set_activation_policy(_accessory: bool) {}

#[tauri::command]
fn set_hide_dock_icon(app: tauri::AppHandle, hide: bool) -> Result<(), String> {
    set_activation_policy(hide);
    db::set_setting(app, "hide_dock_icon".to_string(), if hide { "1".to_string() } else { "0".to_string() })
        .map_err(|e| e.to_string())?;
    Ok(())
}

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
    use cocoa::foundation::{NSRect, NSPoint, NSSize};

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

        // Apply rounded corners to the window
        let _: () = msg_send![content_view, setWantsLayer: 1i8];
        let layer: id = msg_send![content_view, layer];
        if !layer.is_null() {
            let _: () = msg_send![layer, setCornerRadius: 10.0f64];
            let _: () = msg_send![layer, setMasksToBounds: 1i8];
        }

        // Create NSVisualEffectView
        if let Some(ns_vev_class) = Class::get("NSVisualEffectView") {
            // Get bounds as proper NSRect struct — returning as `id` corrupts the value
            let bounds: NSRect = msg_send![content_view, bounds];
            // Guard against NaN/Inf/zero-sized frames that crash AppKit
            let frame = if bounds.size.width.is_finite()
                && bounds.size.height.is_finite()
                && bounds.size.width > 0.0
                && bounds.size.height > 0.0
            {
                bounds
            } else {
                log::warn!(
                    "apply_vibrancy_effect: invalid bounds ({:?} x {:?}), falling back to zero rect",
                    bounds.size.width, bounds.size.height
                );
                NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0))
            };
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

            // Insert behind the first subview (webview) so it doesn't block content
            let subviews: id = msg_send![content_view, subviews];
            let subview_count: usize = msg_send![subviews, count];
            if subview_count > 0 {
                let webview: id = msg_send![subviews, objectAtIndex: 0];
                let _: () = msg_send![content_view, addSubview: effect_view positioned: -1i64 relativeTo: webview];
            } else {
                let _: () = msg_send![content_view, addSubview: effect_view];
            }

            log::info!("apply_vibrancy_effect: NSVisualEffectView applied");
        }
    }
}

/// Compare a stored shortcut string (from DB) against the pressed shortcut.
/// Uses structured HotKey comparison to handle format differences:
/// frontend stores "Shift+D", but HotKey::to_string() outputs "shift+KeyD".
fn shortcut_matches(stored: &str, pressed: &GShortcut) -> bool {
    if stored.is_empty() {
        return false;
    }
    if let Ok(parsed) = GShortcut::from_str(stored) {
        return parsed == *pressed;
    }
    // Fallback to string comparison for legacy or unusual formats
    stored == pressed.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        // Check translate popup shortcut
                        let translate_key = db::get_setting(app.clone(), "translate_shortcut_key".to_string())
                            .unwrap_or_default();
                        if shortcut_matches(&translate_key, shortcut) {
                            shortcut::show_translate_popup(app.clone())
                                .unwrap_or_else(|e| log::warn!("show_translate_popup error: {}", e));
                            return;
                        }

                        // Check radial menu keyboard shortcut
                        let radial_key = db::get_setting(app.clone(), "radial_keyboard_shortcut".to_string())
                            .unwrap_or_default();
                        if shortcut_matches(&radial_key, shortcut) {
                            shortcut::show_radial_menu(app.clone())
                                .unwrap_or_else(|e| log::warn!("show_radial_menu error: {}", e));
                            return;
                        }

                        // Default: toggle main window
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

            // Apply hide-from-Dock setting on startup
            if let Ok(val) = db::get_setting(app.handle().clone(), "hide_dock_icon".to_string()) {
                if val == "1" {
                    set_activation_policy(true);
                }
            }

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

            // Configure translate popup window
            {
                let popup = app.get_webview_window("translate-popup").unwrap();
                let _ = popup.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
                #[cfg(target_os = "windows")]
                apply_backdrop_effect(&popup);
                #[cfg(target_os = "macos")]
                apply_vibrancy_effect(&popup);
                log::info!("Translate popup window created");
            }

            if let Ok(key) = db::get_setting(app.handle().clone(), "shortcut_key".to_string()) {
                if !key.is_empty() {
                    if let Err(e) = shortcut::register_keyboard_shortcut(app.handle(), &key) {
                        log::warn!("Failed to register keyboard shortcut '{}': {}", key, e);
                    }
                }
            }

            // Register radial menu keyboard shortcut (alternative to mouse)
            if let Ok(key) = db::get_setting(app.handle().clone(), "radial_keyboard_shortcut".to_string()) {
                if !key.is_empty() {
                    if let Err(e) = shortcut::register_keyboard_shortcut(app.handle(), &key) {
                        log::warn!("Failed to register radial keyboard shortcut '{}': {}", key, e);
                    }
                }
            }

            // Register translate popup shortcut
            if let Ok(key) = db::get_setting(app.handle().clone(), "translate_shortcut_key".to_string()) {
                if !key.is_empty() {
                    if let Err(e) = shortcut::register_keyboard_shortcut(app.handle(), &key) {
                        log::warn!("Failed to register translate shortcut '{}': {}", key, e);
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
            shortcut::show_radial_menu,
            shortcut::update_radial_keyboard_shortcut,
            shortcut::show_translate_popup,
            shortcut::update_translate_shortcut,
            tray::update_tray_language,
            set_hide_dock_icon,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
