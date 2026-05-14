use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::ptr;

pub static PASTING: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
static LAST_FOREGROUND_HWND: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(ptr::null_mut());

#[cfg(target_os = "windows")]
pub fn save_foreground_window() {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    unsafe {
        let hwnd = GetForegroundWindow();
        LAST_FOREGROUND_HWND.store(hwnd.0, Ordering::SeqCst);
    }
}

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

struct CachedImage {
    rgba: Arc<Vec<u8>>,
    width: u32,
    height: u32,
    png_bytes: Arc<Vec<u8>>,
}

static IMAGE_CACHE: OnceLock<Mutex<HashMap<String, CachedImage>>> = OnceLock::new();

fn get_image_cache() -> &'static Mutex<HashMap<String, CachedImage>> {
    IMAGE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn cache_image(path: String, rgba: Vec<u8>, width: u32, height: u32, png_bytes: Vec<u8>) {
    let mut cache = get_image_cache().lock().unwrap();
    if cache.len() >= 30 {
        let keys: Vec<String> = cache.keys().take(15).cloned().collect();
        for k in keys {
            cache.remove(&k);
        }
    }
    cache.insert(path, CachedImage {
        rgba: Arc::new(rgba),
        width,
        height,
        png_bytes: Arc::new(png_bytes),
    });
}

use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use enigo::{Enigo, Keyboard, Key, Direction, Settings};
use std::thread;
use std::time::Duration;

fn paste_with_defocus(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("no window")?;

    window.hide().map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
        let last_hwnd = LAST_FOREGROUND_HWND.load(Ordering::SeqCst);
        if !last_hwnd.is_null() {
            unsafe {
                let _ = SetForegroundWindow(HWND(last_hwnd));
            }
        }
    }

    thread::sleep(Duration::from_millis(100));

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("enigo init: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        enigo.key(Key::Control, Direction::Press).map_err(|e| e.to_string())?;
        enigo.key(Key::Unicode('v'), Direction::Click).map_err(|e| e.to_string())?;
        enigo.key(Key::Control, Direction::Release).map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        enigo.key(Key::Meta, Direction::Press).map_err(|e| e.to_string())?;
        enigo.key(Key::Unicode('v'), Direction::Click).map_err(|e| e.to_string())?;
        enigo.key(Key::Meta, Direction::Release).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn write_image_to_clipboard(rgba: &[u8], w: u32, h: u32, png_bytes: &[u8]) -> Result<(), String> {
    use windows::Win32::Foundation::{HWND, HANDLE};
    use windows::Win32::System::DataExchange::*;
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    const CF_DIB: u32 = 8;

    unsafe {
        if OpenClipboard(HWND(std::ptr::null_mut())).is_err() {
            return Err("OpenClipboard failed".to_string());
        }
        let _ = EmptyClipboard();

        let dib_size = 40 + (w * h * 4) as usize;
        let hmem_dib = GlobalAlloc(GMEM_MOVEABLE, dib_size).map_err(|e| {
            let _ = CloseClipboard();
            format!("GlobalAlloc DIB failed: {}", e)
        })?;

        let ptr_dib = GlobalLock(hmem_dib);
        if ptr_dib.is_null() {
            let _ = CloseClipboard();
            return Err("GlobalLock DIB failed".to_string());
        }

        let bmi = ptr_dib as *mut u8;
        let bmi_header = std::slice::from_raw_parts_mut(bmi as *mut u32, 10);
        bmi_header[0] = 40;
        bmi_header[1] = w;
        bmi_header[2] = h + (h << 1);
        *(((bmi as *mut u8).add(12)) as *mut u16) = 1;
        *(((bmi as *mut u8).add(14)) as *mut u16) = 32;
        *(((bmi as *mut u8).add(20)) as *mut u32) = w * h * 4;

        let pixel_offset = 40;
        let dst = (bmi as *mut u8).add(pixel_offset);
        let src = rgba.as_ptr();
        std::ptr::copy_nonoverlapping(src, dst, (w * h * 4) as usize);
        let _ = GlobalUnlock(hmem_dib);

        if SetClipboardData(CF_DIB, HANDLE(hmem_dib.0)).is_err() {
            let _ = CloseClipboard();
            return Err("SetClipboardData DIB failed".to_string());
        }

        let png_format_name: Vec<u16> = "PNG\0".encode_utf16().collect();
        let cf_png = RegisterClipboardFormatW(windows::core::PCWSTR(png_format_name.as_ptr()));
        if cf_png != 0 {
            let hmem_png = GlobalAlloc(GMEM_MOVEABLE, png_bytes.len()).map_err(|e| {
                let _ = CloseClipboard();
                format!("GlobalAlloc PNG failed: {}", e)
            })?;

            let ptr_png = GlobalLock(hmem_png);
            if ptr_png.is_null() {
                let _ = CloseClipboard();
                return Err("GlobalLock PNG failed".to_string());
            }

            std::ptr::copy_nonoverlapping(png_bytes.as_ptr(), ptr_png as *mut u8, png_bytes.len());
            let _ = GlobalUnlock(hmem_png);

            if SetClipboardData(cf_png, HANDLE(hmem_png.0)).is_err() {
                let _ = CloseClipboard();
                return Err("SetClipboardData PNG failed".to_string());
            }
        }

        let _ = CloseClipboard();
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn write_files_to_clipboard(paths: &[String]) -> Result<(), String> {
    use windows::Win32::System::DataExchange::*;
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::UI::Shell::DROPFILES;
    use windows::Win32::Foundation::{HWND, HANDLE};

    const CF_HDROP: u32 = 15;

    let wide_paths: Vec<Vec<u16>> = paths.iter().map(|p| p.encode_utf16().chain(std::iter::once(0u16)).collect()).collect();
    let total_wide_len: usize = wide_paths.iter().map(|p| p.len()).sum();

    let dropfiles_size = std::mem::size_of::<DROPFILES>();
    let data_size = dropfiles_size + (total_wide_len + 1) * std::mem::size_of::<u16>();

    let mut data: Vec<u8> = vec![0u8; data_size];

    let df = data.as_mut_ptr() as *mut DROPFILES;
    unsafe {
        (*df).pFiles = dropfiles_size as u32;
        (*df).pt = windows::Win32::Foundation::POINT { x: 0, y: 0 };
        (*df).fNC = windows::Win32::Foundation::BOOL(0);
        (*df).fWide = windows::Win32::Foundation::BOOL(1);
    }

    let offset = dropfiles_size;
    let mut pos = offset;
    for wp in &wide_paths {
        let byte_len = wp.len() * std::mem::size_of::<u16>();
        data[pos..pos + byte_len].copy_from_slice(unsafe { std::slice::from_raw_parts(wp.as_ptr() as *const u8, byte_len) });
        pos += byte_len;
    }

    unsafe {
        if OpenClipboard(HWND(std::ptr::null_mut())).is_err() {
            return Err("OpenClipboard failed".to_string());
        }
        let _ = EmptyClipboard();

        let hmem = GlobalAlloc(GMEM_MOVEABLE, data_size).map_err(|e| {
            let _ = CloseClipboard();
            format!("GlobalAlloc failed: {}", e)
        })?;

        let ptr = GlobalLock(hmem);
        if ptr.is_null() {
            let _ = CloseClipboard();
            return Err("GlobalLock failed".to_string());
        }

        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data_size);
        let _ = GlobalUnlock(hmem);

        if SetClipboardData(CF_HDROP, HANDLE(hmem.0)).is_err() {
            let _ = CloseClipboard();
            return Err("SetClipboardData failed".to_string());
        }

        let _ = CloseClipboard();
    }

    Ok(())
}

#[tauri::command]
pub fn paste_text(app: AppHandle, text: String) -> Result<(), String> {
    PASTING.store(true, Ordering::SeqCst);

    app.clipboard()
        .write_text(text)
        .map_err(|e| e.to_string())?;

    let handle = app.clone();
    std::thread::spawn(move || {
        paste_with_defocus(&handle).ok();
    });

    Ok(())
}

#[tauri::command]
pub fn paste_image(app: AppHandle, path: String) -> Result<(), String> {
    let handle = app.clone();
    std::thread::spawn(move || {
        PASTING.store(true, Ordering::SeqCst);

        let (rgba, w, h, png) = {
            let cache = get_image_cache().lock().unwrap();
            if let Some(cached) = cache.get(&path) {
                (cached.rgba.clone(), cached.width, cached.height, cached.png_bytes.clone())
            } else {
                drop(cache);

                let mut base_dir = crate::db::get_storage_dir(&handle);
                base_dir.push(&path);

                let bytes = match std::fs::read(&base_dir) {
                    Ok(b) => b,
                    Err(e) => { log::error!("paste_image: read error: {}", e); return; }
                };

                let png_arc = Arc::new(bytes.clone());

                let (rgba, w, h) = {
                    use image::ImageDecoder;
                    let decoder = match image::codecs::png::PngDecoder::new(std::io::Cursor::new(&bytes)) {
                        Ok(d) => d,
                        Err(e) => { log::error!("paste_image: decode error: {}", e); return; }
                    };
                    let dims = decoder.dimensions();
                    let mut buf = vec![0; (dims.0 * dims.1 * 4) as usize];
                    if let Err(e) = decoder.read_image(&mut buf) {
                        log::error!("paste_image: read pixels error: {}", e); return;
                    }
                    (buf, dims.0, dims.1)
                };

                cache_image(path.clone(), rgba.clone(), w, h, bytes);
                (Arc::new(rgba), w, h, png_arc)
            }
        };

        #[cfg(target_os = "windows")]
        {
            if let Err(e) = write_image_to_clipboard(&rgba, w, h, &png) {
                log::error!("paste_image: write clipboard error: {}", e); return;
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let tauri_img = tauri::image::Image::new_owned(rgba.to_vec(), w, h);
            if let Err(e) = handle.clipboard().write_image(&tauri_img) {
                log::error!("paste_image: write clipboard error: {}", e); return;
            }
        }

        paste_with_defocus(&handle).ok();
    });

    Ok(())
}

#[tauri::command]
pub fn paste_file(app: AppHandle, path: String) -> Result<(), String> {
    PASTING.store(true, Ordering::SeqCst);

    #[cfg(target_os = "windows")]
    {
        write_files_to_clipboard(&[path])?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        app.clipboard().write_text(path).map_err(|e| e.to_string())?;
    }

    let handle = app.clone();
    std::thread::spawn(move || {
        paste_with_defocus(&handle).ok();
    });

    Ok(())
}
