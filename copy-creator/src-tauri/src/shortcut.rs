use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU64, Ordering};
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

#[cfg(target_os = "macos")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

#[cfg(target_os = "macos")]
fn is_accessibility_permitted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

#[cfg(not(target_os = "macos"))]
fn is_accessibility_permitted() -> bool {
    true
}

// ---------------------------------------------------------------------------
// macOS Accessibility API — read selected text directly without touching the
// clipboard or simulating keystrokes (same approach as Easydict).
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod mac_accessibility {
    use std::ffi::CString;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateSystemWide() -> *const std::ffi::c_void;
        fn AXUIElementCopyAttributeValue(
            element: *const std::ffi::c_void,
            attribute: *const std::ffi::c_void,
            value: *mut *const std::ffi::c_void,
        ) -> i32;
        fn CFRelease(cf: *const std::ffi::c_void);
        fn CFStringCreateWithCString(
            alloc: *const std::ffi::c_void,
            c_str: *const std::os::raw::c_char,
            encoding: u32,
        ) -> *const std::ffi::c_void;
    }

    const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

    fn cf_string(s: &str) -> *const std::ffi::c_void {
        let c = CString::new(s).unwrap();
        unsafe {
            CFStringCreateWithCString(
                std::ptr::null(),
                c.as_ptr(),
                K_CF_STRING_ENCODING_UTF8,
            )
        }
    }

    /// Read the currently selected text in the frontmost application using the
    /// macOS Accessibility API (AXSelectedText). Returns None when the focused
    /// element doesn't expose a text selection or the API is unavailable.
    pub fn get_selected_text() -> Option<String> {
        unsafe {
            let system = AXUIElementCreateSystemWide();
            if system.is_null() {
                return None;
            }

            // --- get focused application ---
            let attr = cf_string("AXFocusedApplication");
            let mut app: *const std::ffi::c_void = std::ptr::null();
            let ok = AXUIElementCopyAttributeValue(
                system,
                attr,
                &mut app as *mut _ as *mut *const std::ffi::c_void,
            ) == 0
                && !app.is_null();
            CFRelease(attr);
            if !ok {
                CFRelease(system);
                return None;
            }

            // --- get focused element inside that app ---
            let attr = cf_string("AXFocusedUIElement");
            let mut el: *const std::ffi::c_void = std::ptr::null();
            let ok = AXUIElementCopyAttributeValue(
                app,
                attr,
                &mut el as *mut _ as *mut *const std::ffi::c_void,
            ) == 0
                && !el.is_null();
            CFRelease(attr);
            CFRelease(app);
            if !ok {
                CFRelease(system);
                return None;
            }

            // --- read selected text ---
            let attr = cf_string("AXSelectedText");
            let mut text: *const std::ffi::c_void = std::ptr::null();
            let ok = AXUIElementCopyAttributeValue(
                el,
                attr,
                &mut text as *mut _ as *mut *const std::ffi::c_void,
            ) == 0
                && !text.is_null();
            CFRelease(attr);
            CFRelease(el);
            CFRelease(system);
            if !ok {
                return None;
            }

            // CFString is toll-free bridged to NSString
            use objc::runtime::Object;
            let ns_str = text as *mut Object;
            let c_str: *const std::os::raw::c_char = objc::msg_send![ns_str, UTF8String];
            let result = if c_str.is_null() {
                String::new()
            } else {
                std::ffi::CStr::from_ptr(c_str).to_string_lossy().into_owned()
            };
            CFRelease(text);

            if result.trim().is_empty() {
                None
            } else {
                Some(result)
            }
        }
    }
}

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

const MOVE_THROTTLE_MS: u64 = 16;

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
        log::info!("[toggle_window] skipped (re-entrant)");
        return;
    }
    let _guard = ToggleGuard;

    if let Some(window) = app.get_webview_window("main") {
        let visible = window.is_visible().unwrap_or(false);
        log::info!("[toggle_window] visible={}", visible);

        if visible {
            log::info!("[toggle_window] hiding window");
            let _ = window.hide();
        } else {
            #[cfg(target_os = "windows")]
            {
                crate::paste::save_foreground_window();
                // Allow our own process (or any process) to call SetForegroundWindow.
                // The thread has temporary foreground permission from the hotkey / hook
                // input, so this ASFW call makes SetForegroundWindow bulletproof.
                unsafe {
                    use windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;
                    let _ = AllowSetForegroundWindow(0xFFFFFFFF);
                }
            }

            #[cfg(target_os = "macos")]
            {
                crate::paste::save_foreground_window();
            }

            log::info!("[toggle_window] showing window");
            let _ = window.show();
            let _ = window.set_focus();
        }
    } else {
        log::warn!("[toggle_window] main window not found");
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

pub fn install_mouse_hook(_app: &AppHandle) {
    #[cfg(target_os = "windows")]
    {
        let app = _app;
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
pub fn show_radial_menu(app: AppHandle) -> Result<(), String> {
    if !RADIAL_MENU_ENABLED.load(Ordering::SeqCst) {
        return Ok(());
    }

    if let Some(window) = app.get_webview_window("radial-menu") {
        crate::paste::save_foreground_window();

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
            use windows::Win32::Foundation::POINT;
            let mut point = POINT { x: 0, y: 0 };
            unsafe { GetCursorPos(&mut point).ok().unwrap_or_default(); }
            let scale = window.scale_factor().unwrap_or(1.0);
            let half_w = (150.0 * scale) as i32;
            let top_off = (30.0 * scale) as i32;
            let _ = window.set_position(tauri::Position::Physical(
                tauri::PhysicalPosition::new(point.x - half_w, point.y - top_off),
            ));
        }

        #[cfg(target_os = "macos")]
        {
            use core_graphics::event::CGEvent;
            use core_graphics::event_source::CGEventSource;
            use core_graphics::event_source::CGEventSourceStateID::HIDSystemState;

            let source = CGEventSource::new(HIDSystemState).ok();
            let event = source.and_then(|s| CGEvent::new(s).ok());
            if let Some(ev) = event {
                let loc = ev.location();
                let scale = window.scale_factor().unwrap_or(2.0);
                let half_w = (150.0 * scale) as f64;
                let top_off = (30.0 * scale) as f64;
                let x = (loc.x - half_w) / scale as f64;
                let y = (loc.y - top_off) / scale as f64;
                let _ = window.set_position(tauri::Position::Physical(
                    tauri::PhysicalPosition::new(x as i32, y as i32),
                ));
            }
        }

        let scale = window.scale_factor().unwrap_or(1.0);
        let css_x = ((150.0 * scale) / scale).round() as i32;
        let css_y = ((30.0 * scale) / scale).round() as i32;

        let theme = crate::db::get_setting(app.clone(), "theme".to_string())
            .unwrap_or_else(|_| "light".to_string());

        let _ = app.emit(
            "radial-menu-down",
            RadialMenuDownPayload { x: css_x, y: css_y, theme },
        );

        let _ = window.show();
        let _ = window.set_focus();
    }

    Ok(())
}

#[tauri::command]
pub fn update_radial_keyboard_shortcut(
    app: AppHandle,
    old_shortcut: String,
    new_shortcut: String,
) -> Result<(), String> {
    if !old_shortcut.is_empty() {
        let _ = unregister_keyboard_shortcut(&app, &old_shortcut);
    }
    if !new_shortcut.is_empty() {
        register_keyboard_shortcut(&app, &new_shortcut)
            .map_err(|e| format!("Failed to register radial keyboard shortcut: {}", e))?;
    }
    Ok(())
}

/// Capture selected text using a multi-level strategy (inspired by Easydict):
///   1. macOS Accessibility API (AXSelectedText)    — no side effects
///   2. Simulate Cmd+C / Ctrl+C with foreground restore — fallback
///   3. Read clipboard directly                      — last resort
fn capture_selected_text(app: &AppHandle) -> String {
    use tauri_plugin_clipboard_manager::ClipboardExt;

    static ACCESSIBILITY_OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let trusted = *ACCESSIBILITY_OK.get_or_init(|| is_accessibility_permitted());

    // --- Level 1: macOS Accessibility API (no clipboard touch, no keystrokes) ---
    #[cfg(target_os = "macos")]
    if trusted {
        if let Some(text) = mac_accessibility::get_selected_text() {
            log::info!(
                "[capture_selected_text] got via AX: {} chars",
                text.len()
            );
            return text;
        }
        log::info!("[capture_selected_text] AX returned nothing, falling back to Cmd+C");
    }

    if !trusted {
        log::info!("[capture_selected_text] accessibility not trusted, falling back to clipboard");
        return app
            .clipboard()
            .read_text()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
    }

    // --- Level 2: simulate copy keystroke ---
    // Save current clipboard so we can restore it afterwards.
    let saved = app.clipboard().read_text().unwrap_or_default();

    // Restore focus to the original foreground app so Cmd+C targets the right window.
    #[cfg(target_os = "macos")]
    {
        crate::paste::restore_foreground_app();
        // Brief settle time after activating the target app
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(e) => e,
        Err(_) => return String::new(),
    };

    #[cfg(target_os = "macos")]
    {
        let _ = enigo.key(Key::Meta, Direction::Press);
        std::thread::sleep(std::time::Duration::from_millis(30));
        let _ = enigo.key(Key::Unicode('c'), Direction::Click);
        std::thread::sleep(std::time::Duration::from_millis(30));
        let _ = enigo.key(Key::Meta, Direction::Release);
    }

    #[cfg(target_os = "windows")]
    {
        let _ = enigo.key(Key::Control, Direction::Press);
        std::thread::sleep(std::time::Duration::from_millis(20));
        let _ = enigo.key(Key::C, Direction::Click);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _ = enigo.key(Key::Control, Direction::Release);
    }

    // Wait for clipboard to receive the copied text
    std::thread::sleep(std::time::Duration::from_millis(150));

    let captured = app
        .clipboard()
        .read_text()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    // Restore original clipboard if we captured something different
    if !captured.is_empty() && captured != saved {
        let _ = app.clipboard().write_text(&saved);
    }

    log::info!(
        "[capture_selected_text] Cmd+C captured: {} chars",
        captured.len()
    );

    captured
}

/// Clamp proposed window position so the window stays fully within the containing monitor.
fn clamp_to_monitor_bounds(
    window: &tauri::WebviewWindow,
    proposed_x: i32,
    proposed_y: i32,
) -> (i32, i32) {
    const MARGIN: i32 = 8;

    let win_size = match window.outer_size() {
        Ok(s) => s,
        Err(_) => return (proposed_x, proposed_y),
    };
    let win_w = win_size.width as i32;
    let win_h = win_size.height as i32;

    let monitors = match window.available_monitors() {
        Ok(m) => m,
        Err(_) => return (proposed_x, proposed_y),
    };

    for monitor in monitors {
        let pos = monitor.position();
        let size = monitor.size();
        let mon_x = pos.x;
        let mon_y = pos.y;
        let mon_w = size.width as i32;
        let mon_h = size.height as i32;

        if proposed_x >= mon_x
            && proposed_x < mon_x + mon_w
            && proposed_y >= mon_y
            && proposed_y < mon_y + mon_h
        {
            let min_x = mon_x + MARGIN;
            let max_x = mon_x + mon_w - win_w - MARGIN;
            let min_y = mon_y + MARGIN;
            let max_y = mon_y + mon_h - win_h - MARGIN;

            return (
                proposed_x.clamp(min_x, max_x.max(min_x)),
                proposed_y.clamp(min_y, max_y.max(min_y)),
            );
        }
    }

    (proposed_x, proposed_y)
}

#[tauri::command]
pub fn show_translate_popup(app: AppHandle) -> Result<(), String> {
    // Save the foreground window NOW while the user's app is still frontmost.
    // The spawned thread will restore focus before simulating Cmd+C.
    crate::paste::save_foreground_window();

    // Spawn on a background thread so the shortcut handler returns immediately.
    // This allows the main run loop to process key-up events, so modifier keys
    // from the shortcut are released before we try to capture text.
    std::thread::spawn(move || {
        // Give the user time to release the shortcut modifier keys
        std::thread::sleep(std::time::Duration::from_millis(200));

        use tauri_plugin_clipboard_manager::ClipboardExt;

        // First try to capture currently selected text, fall back to clipboard
        let selected_text = capture_selected_text(&app);
        let text_to_translate = if !selected_text.is_empty() {
            log::info!("[show_translate_popup] using captured selected text: {} chars", selected_text.len());
            selected_text
        } else {
            let clipboard_text = app.clipboard().read_text()
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            log::info!("[show_translate_popup] falling back to clipboard: {} chars", clipboard_text.len());
            clipboard_text
        };

        if text_to_translate.is_empty() {
            log::info!("[show_translate_popup] no text to translate");
            return;
        }

        if let Some(window) = app.get_webview_window("translate-popup") {

            #[cfg(target_os = "windows")]
            {
                use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
                use windows::Win32::Foundation::POINT;
                let mut point = POINT { x: 0, y: 0 };
                unsafe { GetCursorPos(&mut point).ok().unwrap_or_default(); }
                let scale = window.scale_factor().unwrap_or(1.0);
                let half_w = (190.0 * scale) as i32;
                let top_off = (10.0 * scale) as i32;
                let raw_x = point.x - half_w;
                let raw_y = point.y - top_off;
                let (x, y) = clamp_to_monitor_bounds(&window, raw_x, raw_y);
                let _ = window.set_position(tauri::Position::Physical(
                    tauri::PhysicalPosition::new(x, y),
                ));
            }

            #[cfg(target_os = "macos")]
            {
                use core_graphics::event::CGEvent;
                use core_graphics::event_source::CGEventSource;
                use core_graphics::event_source::CGEventSourceStateID::HIDSystemState;

                let source = CGEventSource::new(HIDSystemState).ok();
                let event = source.and_then(|s| CGEvent::new(s).ok());
                if let Some(ev) = event {
                    let loc = ev.location();
                    let scale = window.scale_factor().unwrap_or(2.0);
                    let half_w = (190.0 * scale) as f64;
                    let top_off = (10.0 * scale) as f64;
                    // CGEvent location is in logical points; multiply by scale for physical pixels
                    let raw_x = (loc.x * scale - half_w) as i32;
                    let raw_y = (loc.y * scale - top_off) as i32;
                    let (x, y) = clamp_to_monitor_bounds(&window, raw_x, raw_y);
                    let _ = window.set_position(tauri::Position::Physical(
                        tauri::PhysicalPosition::new(x, y),
                    ));
                }
            }

            let _ = app.emit("translate-popup-text", &text_to_translate);

            let _ = window.show();
            let _ = window.set_focus();
        }
    });

    Ok(())
}

#[tauri::command]
pub fn update_translate_shortcut(
    app: AppHandle,
    old_shortcut: String,
    new_shortcut: String,
) -> Result<(), String> {
    if !old_shortcut.is_empty() {
        let _ = unregister_keyboard_shortcut(&app, &old_shortcut);
    }
    if !new_shortcut.is_empty() {
        register_keyboard_shortcut(&app, &new_shortcut)
            .map_err(|e| format!("Failed to register translate shortcut: {}", e))?;
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

