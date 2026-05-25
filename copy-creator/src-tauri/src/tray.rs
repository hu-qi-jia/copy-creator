use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;

pub struct TrayState {
    pub tray: Mutex<Option<tauri::tray::TrayIcon>>,
}

fn build_tray_menu(app: &AppHandle, lang: &str) -> Result<tauri::menu::Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let (show_text, quit_text) = if lang == "en" {
        ("Show Window", "Quit")
    } else {
        ("显示窗口", "退出")
    };

    let show = MenuItemBuilder::with_id("show", show_text).build(app)?;
    let quit = MenuItemBuilder::with_id("quit", quit_text).build(app)?;
    MenuBuilder::new(app).item(&show).item(&quit).build().map_err(Into::into)
}

pub fn create_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let lang = crate::db::get_setting_sync(app, "language").unwrap_or_else(|| "zh-CN".to_string());
    let menu = build_tray_menu(app, &lang)?;

    #[cfg(target_os = "macos")]
    let icon_bytes = include_bytes!("../icons/32x32.png");
    #[cfg(not(target_os = "macos"))]
    let icon_bytes = include_bytes!("../icons/icon.png");
    let img = image::load_from_memory(icon_bytes)
        .expect("Failed to decode tray icon")
        .into_rgba8();
    let (w, h) = img.dimensions();
    let icon = tauri::image::Image::new_owned(img.into_raw(), w, h);

    let tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("Copy Creator")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    window.show().ok();
                    window.set_focus().ok();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click { button, button_state, .. } = event {
                if button_state != tauri::tray::MouseButtonState::Down {
                    return;
                }
                if button == tauri::tray::MouseButton::Left {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            window.hide().ok();
                        } else {
                            window.show().ok();
                            window.set_focus().ok();
                        }
                    }
                }
            }
        })
        .build(app)?;

    let state = app.state::<TrayState>();
    *state.tray.lock().unwrap() = Some(tray);

    Ok(())
}

#[tauri::command]
pub fn update_tray_language(app: AppHandle) -> Result<(), String> {
    let lang = crate::db::get_setting_sync(&app, "language").unwrap_or_else(|| "zh-CN".to_string());
    let menu = build_tray_menu(&app, &lang).map_err(|e| e.to_string())?;

    let state = app.state::<TrayState>();
    let tray_guard = state.tray.lock().map_err(|e| e.to_string())?;
    if let Some(tray) = tray_guard.as_ref() {
        tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    }

    Ok(())
}
