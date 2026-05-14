use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

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

pub fn toggle_window(app: &AppHandle) {
    if TOGGLING.swap(true, Ordering::SeqCst) {
        return;
    }

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

    TOGGLING.store(false, Ordering::SeqCst);
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
            if ctrl && shift {
                if let Some(app) = APP_HANDLE.get() {
                    toggle_window(app);
                }
                return LRESULT(1);
            }
        }
    }

    let hook = HHOOK(HOOK_HANDLE.load(Ordering::SeqCst));
    unsafe {
        CallNextHookEx(hook, n_code, w_param, l_param)
    }
}

pub fn install_mouse_hook(app: &AppHandle) {
    #[cfg(target_os = "windows")]
    {
        APP_HANDLE.set(app.clone()).ok();
        let hook = unsafe {
            SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_callback), None, 0)
        };
        if let Ok(h) = hook {
            HOOK_HANDLE.store(h.0, Ordering::SeqCst);
            log::info!("Ctrl+Shift+RightClick mouse hook installed");
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
