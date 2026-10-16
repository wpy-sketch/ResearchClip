use crate::capture::{
    self, CaptureError, DisplayInfo, ErrorCode, MAX_PNG_BYTES, RawFrame, rgba_len,
};
use std::{
    io::{self, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Default)]
pub struct ScreenCaptureService {
    busy: Arc<AtomicBool>,
}

pub struct CapturePermit(Arc<AtomicBool>);
impl Drop for CapturePermit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl ScreenCaptureService {
    // Acquire BEFORE spawn_blocking: repeated IPC cannot build an unbounded work queue.
    pub fn try_begin(&self) -> Result<CapturePermit, CaptureError> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| CaptureError::new(ErrorCode::Busy, "正在处理截图请求，请稍后再试。"))?;
        Ok(CapturePermit(Arc::clone(&self.busy)))
    }
    pub fn enumerate_displays(&self) -> Result<Vec<DisplayInfo>, CaptureError> {
        capture::enumerate_displays()
    }
    pub fn capture_display(&self, id: &str) -> Result<Vec<u8>, CaptureError> {
        encode_png(capture::capture_display(id)?)
    }
}

struct BoundedPng {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}
impl Write for BoundedPng {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if data.len() > self.limit.saturating_sub(self.bytes.len()) {
            self.exceeded = true;
            return Err(io::Error::other("PNG size limit"));
        }
        let required = self.bytes.len() + data.len();
        let capacity = required.next_power_of_two().min(self.limit);
        self.bytes
            .try_reserve_exact(capacity.saturating_sub(self.bytes.len()))
            .map_err(|_| io::Error::other("PNG memory allocation"))?;
        self.bytes.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn encode_png(frame: RawFrame) -> Result<Vec<u8>, CaptureError> {
    encode_png_ref(&frame)
}

pub fn encode_png_ref(frame: &RawFrame) -> Result<Vec<u8>, CaptureError> {
    let expected = rgba_len(frame.width_px, frame.height_px)?;
    if frame.rgba.len() != expected {
        return Err(CaptureError::new(
            ErrorCode::CaptureFailed,
            "捕获图像的像素缓冲区不完整。",
        ));
    }
    let mut output = BoundedPng {
        bytes: Vec::new(),
        limit: MAX_PNG_BYTES,
        exceeded: false,
    };
    let result = (|| {
        let mut encoder = png::Encoder::new(&mut output, frame.width_px, frame.height_px);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&frame.rgba)?;
        writer.finish()
    })();
    if result.is_err() {
        let code = if output.exceeded {
            ErrorCode::ResourceLimit
        } else {
            ErrorCode::EncodeFailed
        };
        return Err(CaptureError::new(
            code,
            "无法编码截图，可能超出内存或图像大小上限。",
        ));
    }
    Ok(output.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_requests_and_releases_on_error_or_panic() {
        let service = ScreenCaptureService::default();
        let permit = service.try_begin().unwrap();
        assert!(matches!(
            service.try_begin(),
            Err(CaptureError {
                code: ErrorCode::Busy,
                ..
            })
        ));
        drop(permit);
        let _ = std::panic::catch_unwind(|| {
            let _permit = service.try_begin().unwrap();
            panic!("test unwind");
        });
        assert!(service.try_begin().is_ok());
    }

    #[test]
    fn png_preserves_actual_dimensions_and_pixels() {
        let pixels = vec![255, 0, 0, 255, 0, 0, 255, 255];
        let encoded = encode_png(RawFrame {
            width_px: 2,
            height_px: 1,
            rgba: pixels.clone(),
        })
        .unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(encoded))
            .read_info()
            .unwrap();
        let mut decoded = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut decoded).unwrap();
        assert_eq!((info.width, info.height), (2, 1));
        assert_eq!(&decoded[..info.buffer_size()], pixels);
    }

    #[test]
    fn refuses_truncated_frame_and_bounded_writer_overflow() {
        assert_eq!(
            encode_png(RawFrame {
                width_px: 2,
                height_px: 2,
                rgba: vec![0; 15]
            })
            .unwrap_err()
            .code,
            ErrorCode::CaptureFailed
        );
        let mut output = BoundedPng {
            bytes: Vec::new(),
            limit: 3,
            exceeded: false,
        };
        output.write_all(&[1, 2, 3]).unwrap();
        assert!(output.write_all(&[4]).is_err());
        assert!(output.exceeded);
        assert_eq!(output.bytes, [1, 2, 3]);
    }
}
