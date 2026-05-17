use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU64, Ordering};
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

static RADIAL_MENU_ENABLED: AtomicBool = AtomicBool::new(true);

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::*;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::*;

#[cfg(target_os = "windows")]
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
#[cfg(target_os = "windows")]
static HOOK_HANDLE: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(core::ptr::null_mut());

static TOGGLING: AtomicBool = AtomicBool::new(false);

/// RAII guard that ensures TOGGLING is always reset, even on panic.
struct ToggleGuard;

impl Drop for ToggleGuard {
    fn drop(&mut self) {
        TOGGLING.store(false, Ordering::SeqCst);
    }
}

#[cfg(target_os = "windows")]
static RADIAL_RIGHT_DOWN: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static RADIAL_START_X: AtomicI32 = AtomicI32::new(0);
#[cfg(target_os = "windows")]
static RADIAL_START_Y: AtomicI32 = AtomicI32::new(0);
#[cfg(target_os = "windows")]
static LAST_MOVE_EMIT_MS: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "windows")]
static RADIAL_SHOW_MS: AtomicU64 = AtomicU64::new(0);

const MOVE_THROTTLE_MS: u64 = 16;
const RELEASE_CHECK_DELAY_MS: u64 = 200;

#[derive(serde::Serialize, Clone)]
struct RadialMenuPoint {
    x: i32,
    y: i32,
}

#[derive(serde::Serialize, Clone)]
struct RadialMenuDownPayload {
    x: i32,
    y: i32,
    theme: String,
}

pub fn toggle_window(app: &AppHandle) {
    if TOGGLING.swap(true, Ordering::SeqCst) {
        return;
    }
    let _guard = ToggleGuard;

    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            #[cfg(target_os = "windows")]
            crate::paste::save_foreground_window();

            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

#[cfg(target_os = "windows")]
fn screen_to_css(window: &tauri::WebviewWindow, screen_x: i32, screen_y: i32) -> Option<(i32, i32)> {
    let win_pos = window.outer_position().ok()?;
    let scale = window.scale_factor().ok().unwrap_or(1.0);
    let rel_x = ((screen_x - win_pos.x) as f64 / scale).round() as i32;
    let rel_y = ((screen_y - win_pos.y) as f64 / scale).round() as i32;
    Some((rel_x, rel_y))
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn mouse_hook_callback(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let msg = w_param.0 as u32;

        if msg == WM_RBUTTONDOWN {
            let ctrl = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16) & 0x8000 != 0;
            let shift = (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16) & 0x8000 != 0;
            let alt = (GetAsyncKeyState(VK_MENU.0 as i32) as u16) & 0x8000 != 0;

            if ctrl && shift {
                if let Some(app) = APP_HANDLE.get() {
                    toggle_window(app);
                }
                return LRESULT(1);
            }

            if ctrl && alt && !shift {
                if !RADIAL_MENU_ENABLED.load(Ordering::SeqCst) {
                    let hook = HHOOK(HOOK_HANDLE.load(Ordering::SeqCst));
                    return unsafe { CallNextHookEx(hook, n_code, w_param, l_param) };
                }
                if let Some(app) = APP_HANDLE.get() {
                    if let Some(window) = app.get_webview_window("radial-menu") {
                        crate::paste::save_foreground_window();

                        let hook_struct = &*(l_param.0 as *const MSLLHOOKSTRUCT);
                        let sx = hook_struct.pt.x;
                        let sy = hook_struct.pt.y;

                        let scale = window.scale_factor().unwrap_or(1.0);
                        let half_w = (150.0 * scale) as i32;
                        let top_off = (30.0 * scale) as i32;

                        // Pre-calc CSS coords before positioning (avoids stale outer_position)
                        let css_x = ((half_w as f64) / scale).round() as i32;
                        let css_y = ((top_off as f64) / scale).round() as i32;

                        let _ = window.set_position(tauri::Position::Physical(
                            tauri::PhysicalPosition::new(sx - half_w, sy - top_off),
                        ));

                        RADIAL_RIGHT_DOWN.store(true, Ordering::SeqCst);
                        RADIAL_START_X.store(sx, Ordering::SeqCst);
                        RADIAL_START_Y.store(sy, Ordering::SeqCst);

                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        RADIAL_SHOW_MS.store(now_ms, Ordering::SeqCst);

                        let theme = crate::db::get_setting(app.clone(), "theme".to_string())
                            .unwrap_or_else(|_| "light".to_string());

                        log::info!("radial-menu-down: screen=({}, {}), css=({}, {}), theme={}", sx, sy, css_x, css_y, theme);
                        let _ = app.emit(
                            "radial-menu-down",
                            RadialMenuDownPayload { x: css_x, y: css_y, theme },
                        );

                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                return LRESULT(1);
            }
        }

        if msg == WM_MOUSEMOVE && RADIAL_RIGHT_DOWN.load(Ordering::SeqCst) {
            // Close window if user released Ctrl or Alt while holding right button.
            // Skip this check within RELEASE_CHECK_DELAY_MS of initial show to avoid
            // spurious aborts from keyboard state jitter during focus transition.
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let show_ms = RADIAL_SHOW_MS.load(Ordering::SeqCst);
            if now_ms.saturating_sub(show_ms) >= RELEASE_CHECK_DELAY_MS {
                let ctrl = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16) & 0x8000 != 0;
                let alt = (GetAsyncKeyState(VK_MENU.0 as i32) as u16) & 0x8000 != 0;
                if !ctrl || !alt {
                    RADIAL_RIGHT_DOWN.store(false, Ordering::SeqCst);
                    if let Some(app) = APP_HANDLE.get() {
                        let _ = app.emit("radial-menu-up", ());
                    }
                    let hook = HHOOK(HOOK_HANDLE.load(Ordering::SeqCst));
                    return unsafe { CallNextHookEx(hook, n_code, w_param, l_param) };
                }
            }

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let last = LAST_MOVE_EMIT_MS.load(Ordering::SeqCst);
            if now.saturating_sub(last) >= MOVE_THROTTLE_MS {
                LAST_MOVE_EMIT_MS.store(now, Ordering::SeqCst);

                if let Some(app) = APP_HANDLE.get() {
                    if let Some(window) = app.get_webview_window("radial-menu") {
                        let hook_struct = &*(l_param.0 as *const MSLLHOOKSTRUCT);
                        let sx = hook_struct.pt.x;
                        let sy = hook_struct.pt.y;

                        if let Some((cx, cy)) = screen_to_css(&window, sx, sy) {
                            let _ = app.emit(
                                "radial-menu-move",
                                RadialMenuPoint { x: cx, y: cy },
                            );
                        }
                    }
                }
            }
        }

        if msg == WM_RBUTTONUP && RADIAL_RIGHT_DOWN.load(Ordering::SeqCst) {
            RADIAL_RIGHT_DOWN.store(false, Ordering::SeqCst);
            log::info!("radial-menu-up");

            if let Some(app) = APP_HANDLE.get() {
                let _ = app.emit("radial-menu-up", ());
            }
            return LRESULT(1);
        }
    }

    let hook = HHOOK(HOOK_HANDLE.load(Ordering::SeqCst));
    unsafe { CallNextHookEx(hook, n_code, w_param, l_param) }
}

pub fn install_mouse_hook(app: &AppHandle) {
    #[cfg(target_os = "windows")]
    {
        // Restore persisted radial menu enabled state
        if let Ok(val) = crate::db::get_setting(app.clone(), "radial_menu_enabled".to_string()) {
            RADIAL_MENU_ENABLED.store(val == "1", Ordering::SeqCst);
        }

        APP_HANDLE.set(app.clone()).ok();
        let hook = unsafe {
            SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_callback), None, 0)
        };
        if let Ok(h) = hook {
            HOOK_HANDLE.store(h.0, Ordering::SeqCst);
            log::info!("Global mouse hook installed (Ctrl+Shift+RightClick / Ctrl+Alt+RightClick)");
        } else {
            log::warn!("Failed to install mouse hook");
        }
    }
}

pub fn register_keyboard_shortcut(
    app: &AppHandle,
    shortcut: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if shortcut.is_empty() {
        return Ok(());
    }
    app.global_shortcut().register(shortcut)?;
    Ok(())
}

pub fn unregister_keyboard_shortcut(
    app: &AppHandle,
    shortcut: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if shortcut.is_empty() {
        return Ok(());
    }
    let _ = app.global_shortcut().unregister(shortcut);
    Ok(())
}

#[tauri::command]
pub fn update_shortcut(
    app: AppHandle,
    old_shortcut: String,
    new_shortcut: String,
) -> Result<(), String> {
    if !old_shortcut.is_empty() {
        let _ = unregister_keyboard_shortcut(&app, &old_shortcut);
    }
    if !new_shortcut.is_empty() {
        register_keyboard_shortcut(&app, &new_shortcut)
            .map_err(|e| format!("Failed to register shortcut: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_radial_menu_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    RADIAL_MENU_ENABLED.store(enabled, Ordering::SeqCst);
    let state = app.state::<crate::db::DbState>();
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('radial_menu_enabled', ?1) ON CONFLICT(key) DO UPDATE SET value = ?1",
        rusqlite::params![if enabled { "1" } else { "0" }],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

