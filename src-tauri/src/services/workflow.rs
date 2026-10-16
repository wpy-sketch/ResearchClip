use crate::capture::{CaptureError, ErrorCode, MAX_PNG_BYTES, RawFrame, rgba_len};
use serde::Serialize;
use std::{
    io::{Cursor, Write},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureInfo {
    pub capture_id: String,
    pub display_id: String,
    pub width_px: u32,
    pub height_px: u32,
    pub mode: String,
}

// Exactly one bounded, app-owned result. Paths and pixels never come from the webview.
pub struct CaptureResource {
    pub info: CaptureInfo,
    pub png: Vec<u8>,
}
#[derive(Default)]
pub struct CaptureWorkflow {
    current: Mutex<Option<Arc<CaptureResource>>>,
    sequence: AtomicU64,
}
fn resource_error() -> CaptureError {
    CaptureError::new(
        ErrorCode::InvalidSession,
        "这张截图已取消或被新截图替换，请重新截图。",
    )
}

impl CaptureWorkflow {
    pub fn store(
        &self,
        display_id: String,
        mode: &str,
        png: Vec<u8>,
    ) -> Result<CaptureInfo, CaptureError> {
        let frame = decode_png(&png)?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| resource_error())?
            .as_nanos();
        let sequence = self
            .sequence
            .try_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| resource_error())?;
        let info = CaptureInfo {
            capture_id: format!("{stamp}-{}-{sequence}", std::process::id()),
            display_id,
            width_px: frame.width_px,
            height_px: frame.height_px,
            mode: mode.into(),
        };
        let resource = Arc::new(CaptureResource {
            info: info.clone(),
            png,
        });
        *self.current.lock().map_err(|_| resource_error())? = Some(resource);
        Ok(info)
    }
    pub fn get(&self, id: &str) -> Result<Arc<CaptureResource>, CaptureError> {
        self.current
            .lock()
            .map_err(|_| resource_error())?
            .as_ref()
            .filter(|r| r.info.capture_id == id)
            .cloned()
            .ok_or_else(resource_error)
    }
    pub fn current(&self) -> Option<CaptureInfo> {
        self.current.lock().ok()?.as_ref().map(|r| r.info.clone())
    }
    pub fn discard(&self, id: &str) -> Result<(), CaptureError> {
        let mut current = self.current.lock().map_err(|_| resource_error())?;
        if current.as_ref().is_some_and(|r| r.info.capture_id == id) {
            *current = None;
        }
        // Late cleanup of a replaced screenshot is intentionally harmless.
        Ok(())
    }
}

pub fn decode_png(bytes: &[u8]) -> Result<RawFrame, CaptureError> {
    let invalid = || {
        CaptureError::new(
            ErrorCode::EncodeFailed,
            "截图 PNG 数据无效，无法复制或保存。",
        )
    };
    if bytes.is_empty() || bytes.len() > MAX_PNG_BYTES {
        return Err(invalid());
    }
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    decoder.set_limits(png::Limits {
        bytes: MAX_PNG_BYTES,
    });
    let mut reader = decoder.read_info().map_err(|_| invalid())?;
    let width = reader.info().width;
    let height = reader.info().height;
    let len = rgba_len(width, height)?;
    if reader.info().animation_control.is_some()
        || reader.output_color_type().1 != png::BitDepth::Eight
    {
        return Err(invalid());
    }
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(len).map_err(|_| {
        CaptureError::new(
            ErrorCode::ResourceLimit,
            "没有足够内存解码截图，请关闭其他图片后重试。",
        )
    })?;
    rgba.resize(len, 0);
    let output = reader.next_frame(&mut rgba).map_err(|_| invalid())?;
    let channels = match output.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        _ => return Err(invalid()),
    };
    if output.buffer_size() != len / 4 * channels {
        return Err(invalid());
    }
    // Validate the complete stream (including IEND), then expand backwards in place.
    reader.finish().map_err(|_| invalid())?;
    for i in (0..len / 4).rev() {
        let s = i * channels;
        let pixel = match channels {
            4 => [rgba[s], rgba[s + 1], rgba[s + 2], rgba[s + 3]],
            3 => [rgba[s], rgba[s + 1], rgba[s + 2], 255],
            2 => [rgba[s], rgba[s], rgba[s], rgba[s + 1]],
            _ => [rgba[s], rgba[s], rgba[s], 255],
        };
        rgba[i * 4..i * 4 + 4].copy_from_slice(&pixel);
    }
    Ok(RawFrame {
        width_px: width,
        height_px: height,
        rgba,
    })
}

// Export always refuses to replace an existing target, even after the OS dialog's
// overwrite prompt. Selecting a fresh name avoids races between confirmation/write.
pub fn export_png_new(path: &Path, png: &[u8]) -> Result<(), CaptureError> {
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("png"))
    {
        return Err(CaptureError::new(
            ErrorCode::SaveFailed,
            "请选择以 .png 结尾的文件名。",
        ));
    }
    if png.is_empty() || png.len() > MAX_PNG_BYTES {
        return Err(resource_error());
    }
    let parent = path.parent().ok_or_else(resource_error)?;
    let mut stage = tempfile::NamedTempFile::new_in(parent).map_err(|_| {
        CaptureError::new(
            ErrorCode::SaveFailed,
            "无法在所选文件夹写入，请检查目录和权限。",
        )
    })?;
    stage
        .write_all(png)
        .and_then(|_| stage.as_file().sync_all())
        .map_err(|_| {
            CaptureError::new(
                ErrorCode::SaveFailed,
                "写入 PNG 失败，请检查磁盘空间和权限。",
            )
        })?;
    stage.persist_noclobber(path).map_err(|error| {
        if error.error.kind() == std::io::ErrorKind::AlreadyExists {
            CaptureError::new(
                ErrorCode::FileExists,
                "同名文件已存在，原文件未改动。请重新保存并选择新的文件名。",
            )
        } else {
            CaptureError::new(
                ErrorCode::SaveFailed,
                "无法提交 PNG 文件，请检查目录和权限。",
            )
        }
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::screen_capture::encode_png;
    fn image() -> Vec<u8> {
        encode_png(RawFrame {
            width_px: 2,
            height_px: 1,
            rgba: vec![10, 20, 30, 255, 40, 50, 60, 255],
        })
        .unwrap()
    }
    #[test]
    fn decoded_clipboard_pixels_are_exact_and_invalid_png_rejected() {
        assert_eq!(
            decode_png(&image()).unwrap().rgba,
            vec![10, 20, 30, 255, 40, 50, 60, 255]
        );
        assert!(decode_png(b"file-path.png").is_err());
        let mut truncated = image();
        truncated.truncate(40);
        assert!(decode_png(&truncated).is_err());
    }
    #[test]
    fn png_rgb_palette_and_alpha_are_decoded_without_losing_transparency() {
        for (kind, data, expected) in [
            (png::ColorType::Rgb, vec![10, 20, 30], vec![10, 20, 30, 255]),
            (
                png::ColorType::GrayscaleAlpha,
                vec![80, 128],
                vec![80, 80, 80, 128],
            ),
            (
                png::ColorType::Rgba,
                vec![10, 20, 30, 0],
                vec![10, 20, 30, 0],
            ),
            (png::ColorType::Indexed, vec![0], vec![90, 120, 150, 64]),
        ] {
            let mut bytes = Vec::new();
            let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
            encoder.set_color(kind);
            encoder.set_depth(png::BitDepth::Eight);
            if kind == png::ColorType::Indexed {
                encoder.set_palette(vec![90, 120, 150]);
                encoder.set_trns(vec![64]);
            }
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&data)
                .unwrap();
            assert_eq!(decode_png(&bytes).unwrap().rgba, expected);
            bytes.truncate(bytes.len() - 3);
            assert!(decode_png(&bytes).is_err());
        }
    }
    #[test]
    fn resource_ids_are_bound_and_late_discard_does_not_clear_new_capture() {
        let service = CaptureWorkflow::default();
        let old = service
            .store("screen".into(), "fullscreen", image())
            .unwrap();
        let new = service.store("screen".into(), "region", image()).unwrap();
        assert!(service.get(&old.capture_id).is_err());
        assert!(service.get("../image.png").is_err());
        service.discard(&old.capture_id).unwrap();
        assert!(service.get(&new.capture_id).is_ok());
        service.discard(&new.capture_id).unwrap();
        assert!(service.current().is_none());
    }
    #[test]
    fn export_is_atomic_and_never_clobbers_existing_user_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("capture.png");
        let png = image();
        export_png_new(&path, &png).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), png);
        assert_eq!(
            export_png_new(&path, b"replacement").unwrap_err().code,
            ErrorCode::FileExists
        );
        assert_eq!(std::fs::read(&path).unwrap(), png);
        assert!(export_png_new(&dir.path().join("capture.txt"), &png).is_err());
        assert!(export_png_new(Path::new("relative.png"), &png).is_err());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
