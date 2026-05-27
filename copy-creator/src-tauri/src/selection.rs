/// macOS text selection capture module.
///
/// Implements a multi-level fallback strategy (inspired by Easydict) to read
/// the currently selected text from any foreground application:
///
/// **Browser path**:  AppleScript → AX API → Cmd+C → stale clipboard
/// **Native path**:   AX API → Cmd+C → stale clipboard

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Accessibility API FFI
// ---------------------------------------------------------------------------

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
// macOS Accessibility API — read selected text via AXSelectedText
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod mac_accessibility_ffi {
    use std::ffi::{c_void, CString};

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        pub fn AXUIElementCreateSystemWide() -> *const c_void;
        pub fn AXUIElementCopyAttributeValue(
            element: *const c_void,
            attribute: *const c_void,
            value: *mut *const c_void,
        ) -> i32;
        pub fn CFRelease(cf: *const c_void);
        pub fn CFStringCreateWithCString(
            alloc: *const c_void,
            c_str: *const std::os::raw::c_char,
            encoding: u32,
        ) -> *const c_void;
    }

    const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

    pub fn cf_string(s: &str) -> *const c_void {
        let c = CString::new(s).unwrap();
        unsafe {
            CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), K_CF_STRING_ENCODING_UTF8)
        }
    }
}

#[cfg(target_os = "macos")]
fn get_selected_text_via_ax() -> Option<String> {
    use mac_accessibility_ffi::*;

    unsafe {
        let system = AXUIElementCreateSystemWide();
        if system.is_null() {
            log::info!("[selection] AX: system element is null");
            return None;
        }

        // Get focused application
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
            log::info!("[selection] AX: failed to get focused application");
            return None;
        }

        // Get focused UI element inside that app
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
            log::info!("[selection] AX: failed to get focused UI element");
            return None;
        }

        // Read selected text
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
            log::info!("[selection] AX: no selected text attribute");
            return None;
        }

        // CFString → Rust String (toll-free bridged to NSString)
        use objc::runtime::Object;
        let ns_str = text as *mut Object;
        let c_str: *const std::os::raw::c_char = msg_send![ns_str, UTF8String];
        let result = if c_str.is_null() {
            String::new()
        } else {
            std::ffi::CStr::from_ptr(c_str).to_string_lossy().into_owned()
        };
        CFRelease(text);

        if result.trim().is_empty() {
            log::info!("[selection] AX: selected text is empty");
            None
        } else {
            log::info!("[selection] AX: got {} chars", result.len());
            Some(result)
        }
    }
}

// ---------------------------------------------------------------------------
// Browser detection
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
enum ApplescriptSyntax {
    Safari,
    Chromium,
    Skip,
}

#[cfg(target_os = "macos")]
static BROWSER_WHITELIST: &[(&str, ApplescriptSyntax)] = &[
    // Safari syntax
    ("com.apple.Safari", ApplescriptSyntax::Safari),
    ("com.kagi.Kagi", ApplescriptSyntax::Safari), // Orion
    // Chromium syntax
    ("com.google.Chrome", ApplescriptSyntax::Chromium),
    ("com.microsoft.edgemac", ApplescriptSyntax::Chromium),
    ("com.microsoft.edgemac.Beta", ApplescriptSyntax::Chromium),
    ("com.microsoft.edgemac.Dev", ApplescriptSyntax::Chromium),
    ("com.microsoft.edgemac.Canary", ApplescriptSyntax::Chromium),
    ("com.brave.Browser", ApplescriptSyntax::Chromium),
    ("company.thebrowser.Browser", ApplescriptSyntax::Chromium), // Arc
    ("com.vivaldi.Vivaldi", ApplescriptSyntax::Chromium),
    ("com.operasoftware.Opera", ApplescriptSyntax::Chromium),
    ("com.duckduckgo.macos.browser", ApplescriptSyntax::Chromium),
    ("com.microsoft.msedge", ApplescriptSyntax::Chromium),
    ("app.zen-browser.zen", ApplescriptSyntax::Chromium),
    // Firefox: does not support AppleScript JS injection
    ("org.mozilla.firefox", ApplescriptSyntax::Skip),
];

#[cfg(target_os = "macos")]
fn get_frontmost_bundle_id() -> Option<String> {
    use objc::runtime::{Class, Object};

    unsafe {
        let ws_class = Class::get("NSWorkspace")?;
        let shared: *mut Object = msg_send![ws_class, sharedWorkspace];
        let app: *mut Object = msg_send![shared, frontmostApplication];
        if app.is_null() {
            return None;
        }
        let bundle_id: *mut Object = msg_send![app, bundleIdentifier];
        if bundle_id.is_null() {
            return None;
        }
        let c_str: *const std::os::raw::c_char = msg_send![bundle_id, UTF8String];
        if c_str.is_null() {
            return None;
        }
        Some(std::ffi::CStr::from_ptr(c_str).to_string_lossy().into_owned())
    }
}

#[cfg(target_os = "macos")]
fn is_browser(bundle_id: &str) -> Option<ApplescriptSyntax> {
    // 1. Whitelist lookup
    for (id, syntax) in BROWSER_WHITELIST.iter() {
        if bundle_id == *id {
            return Some(*syntax);
        }
    }
    // 2. Scheme sniffing fallback
    scheme_sniff_browser(bundle_id)
}

/// Check if the app registers http/https URL schemes via its Info.plist.
/// If so, treat it as a Chromium-syntax browser.
#[cfg(target_os = "macos")]
fn scheme_sniff_browser(bundle_id: &str) -> Option<ApplescriptSyntax> {
    use objc::runtime::{Class, Object};

    unsafe {
        let ws_class = Class::get("NSWorkspace")?;
        let shared: *mut Object = msg_send![ws_class, sharedWorkspace];

        // Get app path from bundle ID
        let ns_string_class = Class::get("NSString")?;
        let bid: *mut Object = msg_send![ns_string_class,
            stringWithUTF8String: bundle_id.as_ptr() as *const std::os::raw::c_char
        ];
        let app_path: *mut Object = msg_send![shared, absolutePathForAppBundleWithIdentifier: bid];
        if app_path.is_null() {
            return None;
        }

        // Construct Info.plist path
        let c_path: *const std::os::raw::c_char = msg_send![app_path, UTF8String];
        if c_path.is_null() {
            return None;
        }
        let path_str = std::ffi::CStr::from_ptr(c_path).to_string_lossy().into_owned();
        let plist_path = format!("{}/Contents/Info.plist", path_str);

        // Read plist via NSDictionary
        let ns_dict_class = Class::get("NSDictionary")?;
        let plist_path_str: *mut Object = msg_send![ns_string_class,
            stringWithUTF8String: plist_path.as_ptr() as *const std::os::raw::c_char
        ];
        let dict: *mut Object = msg_send![ns_dict_class, dictionaryWithContentsOfFile: plist_path_str];
        if dict.is_null() {
            return None;
        }

        // Look up CFBundleURLTypes
        let key: *mut Object = msg_send![ns_string_class,
            stringWithUTF8String: "CFBundleURLTypes\0".as_ptr() as *const std::os::raw::c_char
        ];
        let url_types: *mut Object = msg_send![dict, objectForKey: key];
        if url_types.is_null() {
            return None;
        }

        let count: usize = msg_send![url_types, count];
        for i in 0..count {
            let entry: *mut Object = msg_send![url_types, objectAtIndex: i];
            let schemes_key: *mut Object = msg_send![ns_string_class,
                stringWithUTF8String: "CFBundleURLSchemes\0".as_ptr() as *const std::os::raw::c_char
            ];
            let schemes: *mut Object = msg_send![entry, objectForKey: schemes_key];
            if schemes.is_null() {
                continue;
            }
            let scheme_count: usize = msg_send![schemes, count];
            for j in 0..scheme_count {
                let scheme: *mut Object = msg_send![schemes, objectAtIndex: j];
                let c_str: *const std::os::raw::c_char = msg_send![scheme, UTF8String];
                if !c_str.is_null() {
                    let s = std::ffi::CStr::from_ptr(c_str).to_string_lossy();
                    if s == "http" || s == "https" {
                        return Some(ApplescriptSyntax::Chromium);
                    }
                }
            }
        }

        None
    }
}

// ---------------------------------------------------------------------------
// AppleScript selection via NSAppleScript (must run on main thread)
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
#[link(name = "System")]
extern "C" {
    static _dispatch_main_q: u8;
    fn dispatch_async_f(
        queue: *mut std::ffi::c_void,
        context: *mut std::ffi::c_void,
        work: extern "C" fn(*mut std::ffi::c_void),
    );
}

#[cfg(target_os = "macos")]
struct ApplescriptContext {
    script: CString,
    done: AtomicBool,
    timed_out: AtomicBool,
    result: Mutex<Option<String>>,
    error_code: Mutex<Option<i32>>,
}

#[cfg(target_os = "macos")]
extern "C" fn applescript_work(context_ptr: *mut std::ffi::c_void) {
    let ctx = unsafe { &*(context_ptr as *const ApplescriptContext) };

    // If already timed out, skip execution
    if ctx.timed_out.load(Ordering::SeqCst) {
        return;
    }

    unsafe {
        let ns_apple_script_class = match objc::runtime::Class::get("NSAppleScript") {
            Some(c) => c,
            None => {
                log::warn!("[selection] AppleScript: NSAppleScript class not found");
                ctx.done.store(true, Ordering::SeqCst);
                return;
            }
        };
        let ns_string_class = match objc::runtime::Class::get("NSString") {
            Some(c) => c,
            None => {
                ctx.done.store(true, Ordering::SeqCst);
                return;
            }
        };

        let source: *mut objc::runtime::Object = msg_send![ns_string_class,
            stringWithUTF8String: ctx.script.as_ptr()
        ];
        let script: *mut objc::runtime::Object =
            msg_send![ns_apple_script_class, initWithSource: source];

        let mut error_dict: *mut objc::runtime::Object = std::ptr::null_mut();
        let result: *mut objc::runtime::Object =
            msg_send![script, executeAndReturnError: &mut error_dict];

        if !result.is_null() {
            let c_str: *const std::os::raw::c_char = msg_send![result, UTF8String];
            if !c_str.is_null() {
                let text = std::ffi::CStr::from_ptr(c_str).to_string_lossy().into_owned();
                if !text.trim().is_empty() {
                    *ctx.result.lock().unwrap() = Some(text);
                }
            }
        } else if !error_dict.is_null() {
            // Extract error number
            let key: *mut objc::runtime::Object = msg_send![ns_string_class,
                stringWithUTF8String: "NSAppleScriptErrorNumber\0".as_ptr()
                    as *const std::os::raw::c_char
            ];
            let error_num: *mut objc::runtime::Object = msg_send![error_dict, objectForKey: key];
            if !error_num.is_null() {
                let num: i32 = msg_send![error_num, intValue];
                *ctx.error_code.lock().unwrap() = Some(num);
            }
        }
    }

    ctx.done.store(true, Ordering::SeqCst);
}

#[cfg(target_os = "macos")]
fn run_applescript_on_main_thread(script: &str, bundle_id: &str) -> Option<String> {
    let ctx = Box::new(ApplescriptContext {
        script: CString::new(script).unwrap(),
        done: AtomicBool::new(false),
        timed_out: AtomicBool::new(false),
        result: Mutex::new(None),
        error_code: Mutex::new(None),
    });
    let ctx_ptr = Box::into_raw(ctx) as *mut std::ffi::c_void;

    unsafe {
        dispatch_async_f(
            &_dispatch_main_q as *const u8 as *mut std::ffi::c_void,
            ctx_ptr,
            applescript_work,
        );
    }

    // Spin-wait with 500ms timeout
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(500);
    while !unsafe { &*((ctx_ptr as *const ApplescriptContext)) }
        .done
        .load(Ordering::SeqCst)
    {
        if start.elapsed() > timeout {
            unsafe { &*((ctx_ptr as *const ApplescriptContext)) }
                .timed_out
                .store(true, Ordering::SeqCst);
            log::warn!(
                "[selection] AppleScript: timeout (500ms) for bundle_id={}",
                bundle_id
            );
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let ctx = unsafe { Box::from_raw(ctx_ptr as *mut ApplescriptContext) };

    if let Some(err) = ctx.error_code.lock().unwrap().as_ref() {
        if *err == -1743 {
            log::warn!(
                "[selection] AppleScript: permission denied (-1743) for bundle_id={}",
                bundle_id
            );
        } else {
            log::warn!(
                "[selection] AppleScript: error {} for bundle_id={}",
                err,
                bundle_id
            );
        }
    }

    let result = ctx.result.lock().unwrap().take();
    if result.is_some() {
        log::info!(
            "[selection] AppleScript: got {} chars for bundle_id={}",
            result.as_ref().map(|s| s.len()).unwrap_or(0),
            bundle_id
        );
    } else {
        log::info!("[selection] AppleScript: empty/failed for bundle_id={}", bundle_id);
    }
    result
}

#[cfg(target_os = "macos")]
fn get_selected_text_via_applescript(
    syntax: &ApplescriptSyntax,
    bundle_id: &str,
) -> Option<String> {
    match syntax {
        ApplescriptSyntax::Skip => {
            log::info!("[selection] AppleScript: skipped for bundle_id={}", bundle_id);
            None
        }
        ApplescriptSyntax::Safari => {
            let script = format!(
                "tell application id \"{}\" to do JavaScript \"window.getSelection().toString()\" in current tab of front window",
                bundle_id
            );
            run_applescript_on_main_thread(&script, bundle_id)
        }
        ApplescriptSyntax::Chromium => {
            let script = format!(
                "tell application id \"{}\" to execute front window's active tab javascript \"window.getSelection().toString()\"",
                bundle_id
            );
            run_applescript_on_main_thread(&script, bundle_id)
        }
    }
}

// ---------------------------------------------------------------------------
// CGEvent-based Cmd+C simulation
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
const CMD_KEYCODE: u16 = 0x37;
#[cfg(target_os = "macos")]
const C_KEYCODE: u16 = 0x08;

#[cfg(target_os = "macos")]
fn simulate_cmd_c() -> bool {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = match CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
        Ok(s) => s,
        Err(_) => {
            log::warn!("[selection] Cmd+C: failed to create CGEventSource");
            return false;
        }
    };

    // Cmd key down
    let mut cmd_down = match CGEvent::new_keyboard_event(source.clone(), CMD_KEYCODE, true) {
        Ok(e) => e,
        Err(_) => {
            log::warn!("[selection] Cmd+C: failed to create Cmd down event");
            return false;
        }
    };
    cmd_down.set_flags(CGEventFlags::CGEventFlagCommand);
    cmd_down.post(CGEventTapLocation::HID);

    std::thread::sleep(std::time::Duration::from_millis(20));

    // C key down
    let mut c_down = match CGEvent::new_keyboard_event(source.clone(), C_KEYCODE, true) {
        Ok(e) => e,
        Err(_) => return false,
    };
    c_down.set_flags(CGEventFlags::CGEventFlagCommand);
    c_down.post(CGEventTapLocation::HID);

    std::thread::sleep(std::time::Duration::from_millis(5));

    // C key up
    let mut c_up = match CGEvent::new_keyboard_event(source.clone(), C_KEYCODE, false) {
        Ok(e) => e,
        Err(_) => return false,
    };
    c_up.set_flags(CGEventFlags::CGEventFlagCommand);
    c_up.post(CGEventTapLocation::HID);

    std::thread::sleep(std::time::Duration::from_millis(5));

    // Cmd key up
    let cmd_up = match CGEvent::new_keyboard_event(source, CMD_KEYCODE, false) {
        Ok(e) => e,
        Err(_) => return false,
    };
    cmd_up.post(CGEventTapLocation::HID);

    true
}

// ---------------------------------------------------------------------------
// Clipboard changeCount polling
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn get_clipboard_change_count() -> i64 {
    use objc::runtime::{Class, Object};

    unsafe {
        let ns_pasteboard = Class::get("NSPasteboard").unwrap();
        let general: *mut Object = msg_send![ns_pasteboard, generalPasteboard];
        let count: i64 = msg_send![general, changeCount];
        count
    }
}

#[cfg(target_os = "macos")]
fn wait_for_clipboard_change(original_count: i64) -> bool {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(300);
    loop {
        if get_clipboard_change_count() != original_count {
            return true;
        }
        if start.elapsed() > timeout {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

// ---------------------------------------------------------------------------
// Full clipboard backup/restore
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
struct ClipboardBackup {
    change_count: i64,
    types: Vec<(String, Vec<u8>)>, // (UTI type name, raw data)
}

#[cfg(target_os = "macos")]
fn backup_clipboard() -> ClipboardBackup {
    use objc::runtime::{Class, Object};

    unsafe {
        let ns_pasteboard = Class::get("NSPasteboard").unwrap();
        let general: *mut Object = msg_send![ns_pasteboard, generalPasteboard];
        let change_count: i64 = msg_send![general, changeCount];

        let types_array: *mut Object = msg_send![general, types];
        let count: usize = msg_send![types_array, count];

        let mut types = Vec::new();
        for i in 0..count {
            let type_obj: *mut Object = msg_send![types_array, objectAtIndex: i];
            let c_str: *const std::os::raw::c_char = msg_send![type_obj, UTF8String];
            if c_str.is_null() {
                continue;
            }
            let type_name = std::ffi::CStr::from_ptr(c_str)
                .to_string_lossy()
                .into_owned();

            let ns_data: *mut Object = msg_send![general, dataForType: type_obj];
            if ns_data.is_null() {
                continue;
            }
            let length: usize = msg_send![ns_data, length];
            if length == 0 || length > 50 * 1024 * 1024 {
                continue;
            }

            let bytes_ptr: *const u8 = msg_send![ns_data, bytes];
            if bytes_ptr.is_null() {
                continue;
            }
            let data = std::slice::from_raw_parts(bytes_ptr, length).to_vec();
            types.push((type_name, data));
        }

        ClipboardBackup {
            change_count,
            types,
        }
    }
}

#[cfg(target_os = "macos")]
fn restore_clipboard(backup: &ClipboardBackup) {
    use objc::runtime::{Class, Object};

    unsafe {
        let ns_pasteboard = Class::get("NSPasteboard").unwrap();
        let general: *mut Object = msg_send![ns_pasteboard, generalPasteboard];
        let current_count: i64 = msg_send![general, changeCount];

        // Only restore if the clipboard was actually modified
        if current_count == backup.change_count {
            return;
        }

        let _: usize = msg_send![general, clearContents];

        let ns_data_class = Class::get("NSData").unwrap();
        let ns_string_class = Class::get("NSString").unwrap();

        for (type_name, data) in &backup.types {
            let type_str: *mut Object = msg_send![ns_string_class,
                stringWithUTF8String: type_name.as_ptr() as *const std::os::raw::c_char
            ];
            let ns_data: *mut Object = msg_send![ns_data_class,
                dataWithBytes: data.as_ptr() length: data.len()
            ];
            let _: bool = msg_send![general, setData: ns_data forType: type_str];
        }
    }
}

// ---------------------------------------------------------------------------
// Clipboard text read helper
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn read_clipboard_text() -> Option<String> {
    use objc::runtime::{Class, Object};

    unsafe {
        let ns_pasteboard = Class::get("NSPasteboard").unwrap();
        let general: *mut Object = msg_send![ns_pasteboard, generalPasteboard];
        let ns_string_class = Class::get("NSString").unwrap();
        let text_type: *mut Object = msg_send![ns_string_class,
            stringWithUTF8String: "public.utf8-plain-text\0".as_ptr()
                as *const std::os::raw::c_char
        ];
        let ns_string: *mut Object = msg_send![general, stringForType: text_type];
        if ns_string.is_null() {
            return None;
        }
        let c_str: *const std::os::raw::c_char = msg_send![ns_string, UTF8String];
        if c_str.is_null() {
            return None;
        }
        let text = std::ffi::CStr::from_ptr(c_str)
            .to_string_lossy()
            .into_owned();
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }
}

// ---------------------------------------------------------------------------
// Stale clipboard fallback with recency check
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
static SEEN_CHANGE_COUNT: AtomicI64 = AtomicI64::new(0);
#[cfg(target_os = "macos")]
static SEEN_CHANGE_TIME: Mutex<Option<std::time::Instant>> = Mutex::new(None);

#[cfg(target_os = "macos")]
fn check_clipboard_recency() -> bool {
    let current = get_clipboard_change_count();
    let last_seen = SEEN_CHANGE_COUNT.load(Ordering::SeqCst);
    if current != last_seen {
        SEEN_CHANGE_COUNT.store(current, Ordering::SeqCst);
        *SEEN_CHANGE_TIME.lock().unwrap() = Some(std::time::Instant::now());
        return true;
    }
    let time_lock = SEEN_CHANGE_TIME.lock().unwrap();
    if let Some(instant) = *time_lock {
        instant.elapsed() < std::time::Duration::from_secs(2)
    } else {
        false
    }
}

#[cfg(target_os = "macos")]
fn try_stale_clipboard() -> Option<String> {
    if !check_clipboard_recency() {
        log::info!("[selection] stale clipboard: changeCount not recent, skipping");
        return None;
    }
    let text = read_clipboard_text();
    if text.is_some() {
        log::info!(
            "[selection] stale clipboard: got {} chars",
            text.as_ref().map(|s| s.len()).unwrap_or(0)
        );
    }
    text
}

// ---------------------------------------------------------------------------
// Cmd+C with full clipboard backup
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn try_cmd_c_with_backup() -> Option<String> {
    // Restore focus to the original foreground app
    crate::paste::restore_foreground_app();
    std::thread::sleep(std::time::Duration::from_millis(150));

    let backup = backup_clipboard();
    let original_count = backup.change_count;

    if !simulate_cmd_c() {
        log::warn!("[selection] Cmd+C: CGEvent simulation failed");
        return None;
    }

    let changed = wait_for_clipboard_change(original_count);
    if !changed {
        log::info!("[selection] Cmd+C: clipboard did not change within 300ms");
        return None;
    }

    let text = read_clipboard_text();
    if text.is_none() || text.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        log::info!("[selection] Cmd+C: clipboard changed but text is empty");
        restore_clipboard(&backup);
        return None;
    }

    restore_clipboard(&backup);

    log::info!(
        "[selection] Cmd+C: got {} chars",
        text.as_ref().map(|s| s.len()).unwrap_or(0)
    );
    text
}

// ---------------------------------------------------------------------------
// Main selection chain
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn capture_browser_selection(bundle_id: &str, syntax: &ApplescriptSyntax, trusted: bool) -> String {
    // Level 1: AppleScript (primary for browsers)
    if let Some(text) = get_selected_text_via_applescript(syntax, bundle_id) {
        log::info!("[selection] browser: got {} chars via AppleScript", text.len());
        return text;
    }

    // Level 2: AX API (secondary for browsers — often returns empty/wrong)
    if trusted {
        if let Some(text) = get_selected_text_via_ax() {
            log::info!("[selection] browser: got {} chars via AX", text.len());
            return text;
        }
    }

    // Level 3: Simulated Cmd+C with full clipboard backup
    if trusted {
        if let Some(text) = try_cmd_c_with_backup() {
            log::info!("[selection] browser: got {} chars via Cmd+C", text.len());
            return text;
        }
    }

    // Level 4: Stale clipboard (only if recent)
    if let Some(text) = try_stale_clipboard() {
        log::info!("[selection] browser: got {} chars via stale clipboard", text.len());
        return text;
    }

    log::info!("[selection] browser: no text captured, bundle_id={}", bundle_id);
    String::new()
}

#[cfg(target_os = "macos")]
fn capture_native_selection(bundle_id: &str, trusted: bool) -> String {
    // Level 1: AX API
    if trusted {
        if let Some(text) = get_selected_text_via_ax() {
            log::info!(
                "[selection] native: got {} chars via AX, bundle_id={}",
                text.len(),
                bundle_id
            );
            return text;
        }
    }

    // Level 2: Simulated Cmd+C with full clipboard backup
    if trusted {
        if let Some(text) = try_cmd_c_with_backup() {
            log::info!(
                "[selection] native: got {} chars via Cmd+C, bundle_id={}",
                text.len(),
                bundle_id
            );
            return text;
        }
    }

    // Level 3: Stale clipboard (only if recent)
    if let Some(text) = try_stale_clipboard() {
        log::info!(
            "[selection] native: got {} chars via stale clipboard, bundle_id={}",
            text.len(),
            bundle_id
        );
        return text;
    }

    log::info!("[selection] native: no text captured, bundle_id={}", bundle_id);
    String::new()
}

/// Capture the currently selected text using a multi-level fallback strategy.
///
/// **Browser path**:  AppleScript → AX API → Cmd+C → stale clipboard
/// **Native path**:   AX API → Cmd+C → stale clipboard
#[cfg(target_os = "macos")]
pub fn capture_selected_text() -> String {
    let bundle_id = get_frontmost_bundle_id().unwrap_or_default();
    let browser_syntax = is_browser(&bundle_id);
    let trusted = is_accessibility_permitted();

    log::info!(
        "[selection] bundle_id={}, is_browser={}, accessibility_trusted={}",
        bundle_id,
        browser_syntax.is_some(),
        trusted
    );

    if let Some(syntax) = &browser_syntax {
        capture_browser_selection(&bundle_id, syntax, trusted)
    } else {
        capture_native_selection(&bundle_id, trusted)
    }
}

/// Windows fallback: simulate Ctrl+C via enigo.
#[cfg(not(target_os = "macos"))]
pub fn capture_selected_text() -> String {
    // Windows implementation is handled in shortcut.rs for now
    // (kept as-is to avoid breaking changes)
    String::new()
}
