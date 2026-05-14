use std::io::Write;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

fn is_url(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("ftp://")
        || lower.starts_with("ftps://")
}

#[cfg(target_os = "windows")]
fn read_clipboard_files() -> Option<Vec<String>> {
    use windows::Win32::System::DataExchange::*;
    use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
    use windows::Win32::Foundation::HWND;

    const CF_HDROP: u32 = 15;

    unsafe {
        if OpenClipboard(HWND(std::ptr::null_mut())).is_err() {
            return None;
        }

        if IsClipboardFormatAvailable(CF_HDROP).is_err() {
            let _ = CloseClipboard();
            return None;
        }

        let handle = match GetClipboardData(CF_HDROP) {
            Ok(h) => h,
            Err(_) => {
                let _ = CloseClipboard();
                return None;
            }
        };

        let hdrop = HDROP(handle.0);

        let count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);
        if count == 0 {
            let _ = CloseClipboard();
            return None;
        }

        let mut paths = Vec::new();
        for i in 0..count {
            let len = DragQueryFileW(hdrop, i, None);
            if len == 0 {
                continue;
            }
            let mut buf = vec![0u16; (len as usize) + 1];
            DragQueryFileW(hdrop, i, Some(&mut buf));
            let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let path = String::from_utf16_lossy(&buf[..end]);
            if !path.is_empty() {
                paths.push(path);
            }
        }

        let _ = CloseClipboard();

        if paths.is_empty() {
            None
        } else {
            Some(paths)
        }
    }
}

pub fn start_monitor(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.clone();
    let mut last_text = String::new();
    let mut last_image_hash: u64 = 0;
    let mut last_files_key = String::new();

    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(800));

        if crate::paste::PASTING.swap(false, std::sync::atomic::Ordering::SeqCst) {
            let _ = handle.clipboard().read_text();
            if let Ok(image) = handle.clipboard().read_image() {
                let rgba = image.rgba();
                if !rgba.is_empty() && image.width() > 0 && image.height() > 0 {
                    last_image_hash = rgba
                        .iter()
                        .take(400)
                        .fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64));
                }
            }
            if let Ok(text) = handle.clipboard().read_text() {
                last_text = text.trim().to_string();
            }
            #[cfg(target_os = "windows")]
            {
                if let Some(files) = read_clipboard_files() {
                    last_files_key = files.join("|");
                }
            }
            continue;
        }

        let mut image_recorded = false;

        if let Ok(image) = handle.clipboard().read_image() {
            let rgba = image.rgba();
            if !rgba.is_empty() && image.width() > 0 && image.height() > 0 {
                let hash = rgba
                    .iter()
                    .take(400)
                    .fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64));

                if hash != last_image_hash {
                    last_image_hash = hash;

                    let rgba_vec = rgba.to_vec();
                    let img_w = image.width();
                    let img_h = image.height();

                    let mut png_bytes: Vec<u8> = Vec::new();
                    {
                        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
                        use image::ImageEncoder;
                        let _ = encoder.write_image(
                            rgba,
                            img_w,
                            img_h,
                            image::ExtendedColorType::Rgba8,
                        );
                    }

                    if !png_bytes.is_empty() {
                        let mut dir = crate::db::get_storage_dir(&handle);
                        dir.push("images");
                        std::fs::create_dir_all(&dir).ok();

                        let filename = format!("{}.png", uuid::Uuid::new_v4());
                        let filepath = dir.join(&filename);

                        if let Ok(mut f) = std::fs::File::create(&filepath) {
                            if f.write_all(&png_bytes).is_ok() {
                                let relative = format!("images/{}", filename);

                                crate::paste::cache_image(relative.clone(), rgba_vec, img_w, img_h, png_bytes.clone());

                                let mut thumb_dir = dir.clone();
                                thumb_dir.push("thumbs");
                                std::fs::create_dir_all(&thumb_dir).ok();
                                let thumb_path = thumb_dir.join(&filename);
                                if let Ok(decoded) = image::load_from_memory(&png_bytes) {
                                    let (tw, th) = (decoded.width(), decoded.height());
                                    let max_thumb: u32 = 200;
                                    let scale = if tw > max_thumb || th > max_thumb {
                                        max_thumb as f32 / tw.max(th) as f32
                                    } else {
                                        1.0
                                    };
                                    let thumb = if scale < 1.0 {
                                        decoded.resize(
                                            (tw as f32 * scale) as u32,
                                            (th as f32 * scale) as u32,
                                            image::imageops::FilterType::Triangle,
                                        )
                                    } else {
                                        decoded
                                    };
                                    let mut thumb_buf = std::io::Cursor::new(Vec::new());
                                    if thumb.write_to(&mut thumb_buf, image::ImageFormat::Png).is_ok() {
                                        if let Ok(mut tf) = std::fs::File::create(&thumb_path) {
                                            let _ = tf.write_all(&thumb_buf.into_inner());
                                        }
                                    }
                                }

                                let id = uuid::Uuid::new_v4().to_string();
                                let now = chrono::Utc::now().to_rfc3339();

                                let state = handle.state::<crate::db::DbState>();
                                let conn = state.conn.lock().unwrap();
                                conn.execute(
                                    "INSERT INTO clipboard_records (id, type, content, source_app, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                                    rusqlite::params![id, "image", relative, "", &now],
                                )
                                .ok();

                                handle
                                    .emit(
                                        "clipboard-update",
                                        serde_json::json!({
                                            "id": id,
                                            "type": "image",
                                            "content": relative,
                                            "source_app": "",
                                            "created_at": now,
                                        }),
                                    )
                                    .ok();

                                image_recorded = true;
                            }
                        }
                    }
                }
            }
        }

        if image_recorded {
            if let Ok(text) = handle.clipboard().read_text() {
                last_text = text.trim().to_string();
            }
            #[cfg(target_os = "windows")]
            {
                if let Some(files) = read_clipboard_files() {
                    last_files_key = files.join("|");
                }
            }
        } else {
            if let Ok(text) = handle.clipboard().read_text() {
                let text = text.trim().to_string();
                if !text.is_empty() && text != last_text {
                    last_text = text.clone();

                    let record_type = if is_url(&text) {
                        "link"
                    } else {
                        "text"
                    };

                    let id = uuid::Uuid::new_v4().to_string();
                    let now = chrono::Utc::now().to_rfc3339();
                    let state = handle.state::<crate::db::DbState>();
                    let conn = state.conn.lock().unwrap();
                    conn.execute(
                        "INSERT INTO clipboard_records (id, type, content, source_app, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                        rusqlite::params![id, record_type, text, "", &now],
                    )
                    .ok();

                    handle
                        .emit(
                            "clipboard-update",
                            serde_json::json!({
                                "id": id,
                                "type": record_type,
                                "content": text,
                                "source_app": "",
                                "created_at": now,
                            }),
                        )
                        .ok();
                }
            }

            #[cfg(target_os = "windows")]
            {
                if let Some(files) = read_clipboard_files() {
                    let key = files.join("|");
                    if key != last_files_key {
                        last_files_key = key.clone();

                        for file_path in files {
                            if file_path.trim().is_empty() {
                                continue;
                            }

                            let id = uuid::Uuid::new_v4().to_string();
                            let now = chrono::Utc::now().to_rfc3339();
                            let state = handle.state::<crate::db::DbState>();
                            let conn = state.conn.lock().unwrap();
                            conn.execute(
                                "INSERT INTO clipboard_records (id, type, content, source_app, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                                rusqlite::params![id, "file", file_path, "", &now],
                            )
                            .ok();

                            handle
                                .emit(
                                    "clipboard-update",
                                    serde_json::json!({
                                        "id": id,
                                        "type": "file",
                                        "content": file_path,
                                        "source_app": "",
                                        "created_at": now,
                                    }),
                                )
                                .ok();
                        }
                    }
                }
            }
        }
    });

    Ok(())
}
