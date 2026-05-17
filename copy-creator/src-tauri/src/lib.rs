mod clipboard;
mod db;
mod paste;
mod shortcut;
mod translator;
mod tray;

use tauri::Manager;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_process::init())
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

            db::init_db(app.handle())?;
            db::prune_old_records(app.handle()).ok();

            // Periodic pruning every hour
            let prune_handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
                db::prune_old_records(&prune_handle).ok();
            });

            clipboard::start_monitor(app.handle())?;
            tray::create_tray(app.handle())?;

            shortcut::install_mouse_hook(app.handle());

            // Create hidden radial menu popup window
            {
                use tauri::WebviewWindowBuilder;
                use tauri::WebviewUrl;
                let radial = WebviewWindowBuilder::new(
                    app,
                    "radial-menu",
                    WebviewUrl::App("index.html?radial=1".into()),
                )
                .title("")
                .inner_size(300.0, 420.0)
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .visible(false)
                .shadow(true)
                .skip_taskbar(true)
                .resizable(false)
                .build()?;
                let _ = radial.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
                #[cfg(target_os = "windows")]
                apply_backdrop_effect(&radial);
                log::info!("Radial menu popup window created");
            }

            if let Ok(key) = db::get_setting(app.handle().clone(), "shortcut_key".to_string()) {
                if !key.is_empty() {
                    if let Err(e) = shortcut::register_keyboard_shortcut(app.handle(), &key) {
                        log::warn!("Failed to register keyboard shortcut '{}': {}", key, e);
                    }
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
            db::set_setting,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
