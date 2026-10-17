use crate::capture::{CaptureError, ErrorCode, MAX_PIXELS, RawFrame, rgba_len};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

pub const MAX_FILE: usize = 20 * 1024 * 1024;
pub const MAX_BATCH: usize = 64 * 1024 * 1024;
pub const MAX_FILES: usize = 10;
pub const MAX_HEADER: usize = 16 * 1024;
pub fn invalid(message: &str) -> CaptureError {
    CaptureError::new(ErrorCode::EncodeFailed, message)
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileMeta {
    pub name: String,
    pub size: usize,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    pub files: Vec<FileMeta>,
}
pub fn unpack(bytes: &[u8]) -> Result<(Batch, Vec<&[u8]>), CaptureError> {
    if bytes.len() < 4 || bytes.len() > 4 + MAX_HEADER + MAX_BATCH {
        return Err(invalid("导入封包超限：最多10张、合计64 MiB。"));
    }
    let size = u32::from_le_bytes(
        bytes[..4]
            .try_into()
            .map_err(|_| invalid("导入封包无效。"))?,
    ) as usize;
    if size > MAX_HEADER || size + 4 > bytes.len() {
        return Err(invalid("导入封包头无效。"));
    }
    let batch: Batch =
        serde_json::from_slice(&bytes[4..4 + size]).map_err(|_| invalid("导入文件信息无效。"))?;
    if batch.files.is_empty() || batch.files.len() > MAX_FILES {
        return Err(invalid("一次请导入1至10张图片。"));
    }
    let mut offset = 4 + size;
    let mut files = Vec::new();
    for file in &batch.files {
        if file.name.len() > 1024
            || file.size == 0
            || file.size > MAX_FILE
            || file.size > bytes.len().saturating_sub(offset)
        {
            return Err(invalid("单张图片需非空且不超过20 MiB。"));
        }
        files.push(&bytes[offset..offset + file.size]);
        offset += file.size;
    }
    if offset != bytes.len() {
        return Err(invalid("导入封包含多余内容。"));
    }
    Ok((batch, files))
}
pub fn title(name: &str) -> Result<String, CaptureError> {
    if name.is_empty() || name.contains(['/', '\\']) || name.chars().any(char::is_control) {
        return Err(invalid("文件名称无效。"));
    }
    let stem = name.rsplit_once('.').map_or(name, |(s, _)| s).trim();
    if stem.is_empty() {
        return Err(invalid("图片名称不能为空。"));
    }
    Ok(stem.chars().take(200).collect())
}
pub fn decode_file(name: &str, bytes: &[u8]) -> Result<RawFrame, CaptureError> {
    title(name)?;
    let expected = match name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => ImageFormat::Png,
        "jpg" | "jpeg" => ImageFormat::Jpeg,
        "webp" => ImageFormat::WebP,
        _ => return Err(invalid("格式不支持：请选择 PNG、JPG/JPEG 或 WebP 图片。")),
    };
    if bytes.is_empty() || bytes.len() > MAX_FILE {
        return Err(invalid("单张图片需非空且不超过20 MiB。"));
    }
    if image::guess_format(bytes).ok() != Some(expected) {
        return Err(invalid("文件扩展名与真实图片格式不符，或图片已损坏。"));
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), expected);
    let mut limits = Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| invalid("图片头损坏、格式不支持或超过解码资源限制。"))?;
    let (width, height) = decoder.dimensions();
    rgba_len(width, height).map_err(|_| invalid("图片尺寸超限：最多16,777,216像素。"))?;
    if decoder.total_bytes() > 128 * 1024 * 1024 {
        return Err(invalid("图片解码数据超过128 MiB。"));
    }
    let orientation = decoder
        .orientation()
        .map_err(|_| invalid("图片方向信息损坏。"))?;
    let mut decoded =
        DynamicImage::from_decoder(decoder).map_err(|_| invalid("图片内容损坏，无法解码。"))?;
    decoded.apply_orientation(orientation);
    let rgba = decoded.into_rgba8();
    Ok(RawFrame {
        width_px: rgba.width(),
        height_px: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

/// DIB snapshot parser: bounded 24-bit RGB / 32-bit RGB or canonical BGRA bitfields.
pub fn decode_dib(bytes: &[u8]) -> Result<RawFrame, CaptureError> {
    let fail = || invalid("剪贴板位图损坏或不支持；请在图像应用中复制24/32位图片后重试。");
    if bytes.len() < 40 || bytes.len() > MAX_PIXELS as usize * 4 + 1024 {
        return Err(fail());
    }
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let header = u32_at(0) as usize;
    if ![40, 108, 124].contains(&header) || bytes.len() < header {
        return Err(fail());
    }
    let width = u32_at(4) as i32;
    let signed_height = u32_at(8) as i32;
    if width <= 0 || signed_height == i32::MIN {
        return Err(fail());
    }
    let height = signed_height.unsigned_abs();
    let len = rgba_len(width as u32, height)
        .map_err(|_| invalid("剪贴板图片尺寸超限：最多16,777,216像素。"))?;
    let planes = u16::from_le_bytes(bytes[12..14].try_into().unwrap());
    let bits = u16::from_le_bytes(bytes[14..16].try_into().unwrap());
    let compression = u32_at(16);
    if planes != 1
        || ![24, 32].contains(&bits)
        || !(compression == 0 || (compression == 3 && bits == 32))
    {
        return Err(fail());
    }
    let mut offset = header;
    let mut alpha = false;
    if compression == 3 {
        if header == 40 {
            offset += 12;
        }
        if bytes.len() < offset {
            return Err(fail());
        }
        if [u32_at(40), u32_at(44), u32_at(48)] != [0x00ff0000, 0x0000ff00, 0x000000ff] {
            return Err(fail());
        }
        if header >= 108 {
            let mask = u32_at(52);
            if mask != 0 && mask != 0xff000000 {
                return Err(fail());
            }
            alpha = mask != 0;
        }
    }
    // WPF commonly advertises BI_RGB in V5 headers while supplying explicit BGRA masks.
    if compression == 0
        && bits == 32
        && header == 124
        && [u32_at(40), u32_at(44), u32_at(48), u32_at(52)]
            == [0x00ff0000, 0x0000ff00, 0x000000ff, 0xff000000]
    {
        alpha = true;
    }
    // High-colour DIBs may still declare an optimisation palette.
    offset = offset
        .checked_add((u32_at(32) as usize).checked_mul(4).ok_or_else(fail)?)
        .ok_or_else(fail)?;
    let stride = (width as usize * bits as usize).div_ceil(32) * 4;
    if offset > bytes.len() || height as usize > (bytes.len() - offset) / stride {
        return Err(fail());
    }
    let mut rgba = vec![0; len];
    for y in 0..height as usize {
        let row = if signed_height < 0 {
            y
        } else {
            height as usize - 1 - y
        };
        for x in 0..width as usize {
            let src = offset + row * stride + x * (bits as usize / 8);
            let dst = (y * width as usize + x) * 4;
            rgba[dst..dst + 4].copy_from_slice(&[
                bytes[src + 2],
                bytes[src + 1],
                bytes[src],
                if alpha { bytes[src + 3] } else { 255 },
            ]);
        }
    }
    Ok(RawFrame {
        width_px: width as u32,
        height_px: height,
        rgba,
    })
}

#[cfg(windows)]
pub fn clipboard_frame() -> Result<RawFrame, CaptureError> {
    use windows::{
        Win32::{
            Foundation::HGLOBAL,
            System::{
                DataExchange::{
                    CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
                    RegisterClipboardFormatW,
                },
                Memory::{GlobalLock, GlobalSize, GlobalUnlock},
            },
        },
        core::w,
    };
    struct Clipboard;
    impl Drop for Clipboard {
        fn drop(&mut self) {
            let _ = unsafe { CloseClipboard() };
        }
    }
    struct Locked(HGLOBAL);
    impl Drop for Locked {
        fn drop(&mut self) {
            let _ = unsafe { GlobalUnlock(self.0) };
        }
    }
    let (bytes, png) = {
        unsafe { OpenClipboard(None) }.map_err(|_| {
            CaptureError::new(
                ErrorCode::ClipboardUnavailable,
                "剪贴板暂不可用，可能正被其他应用占用；请稍后重试。",
            )
        })?;
        let _clipboard = Clipboard;
        let png_format = unsafe { RegisterClipboardFormatW(w!("PNG")) };
        let (format, png) =
            if png_format != 0 && unsafe { IsClipboardFormatAvailable(png_format) }.is_ok() {
                (png_format, true)
            } else if unsafe { IsClipboardFormatAvailable(17) }.is_ok() {
                (17, false)
            } else if unsafe { IsClipboardFormatAvailable(8) }.is_ok() {
                (8, false)
            } else {
                return Err(CaptureError::new(
                    ErrorCode::ClipboardUnavailable,
                    "剪贴板中没有可导入的图片，请先复制图片本身（不是文件路径）。",
                ));
            };
        let handle = unsafe { GetClipboardData(format) }
            .map_err(|_| invalid("无法读取剪贴板图片，请重新复制。"))?;
        let handle = HGLOBAL(handle.0);
        let size = unsafe { GlobalSize(handle) };
        let limit = if png {
            MAX_FILE
        } else {
            MAX_PIXELS as usize * 4 + 1024
        };
        if size == 0 || size > limit {
            return Err(invalid("剪贴板图片数据超过导入上限。"));
        }
        let pointer = unsafe { GlobalLock(handle) };
        if pointer.is_null() {
            return Err(invalid("无法锁定剪贴板图片数据。"));
        }
        let _locked = Locked(handle);
        // The global block remains locked and the clipboard remains open throughout this bounded copy.
        (
            unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), size) }.to_vec(),
            png,
        )
    };
    if png {
        decode_file("clipboard.png", &bytes)
    } else {
        decode_dib(&bytes)
    }
}
#[cfg(not(windows))]
pub fn clipboard_frame() -> Result<RawFrame, CaptureError> {
    Err(CaptureError::new(
        ErrorCode::Unsupported,
        "当前版本的剪贴板图片导入仅在 Windows 启用。",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn encoded(format: ImageFormat) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::new_rgba8(3, 2)
            .write_to(&mut bytes, format)
            .unwrap();
        bytes.into_inner()
    }
    #[test]
    fn formats_and_corrupt_content_are_validated() {
        for (name, format) in [
            ("中文.PNG", ImageFormat::Png),
            ("research.jpg", ImageFormat::Jpeg),
            ("research.JPEG", ImageFormat::Jpeg),
            ("a.webp", ImageFormat::WebP),
        ] {
            let bytes = encoded(format);
            let frame = decode_file(name, &bytes).unwrap();
            assert_eq!((frame.width_px, frame.height_px), (3, 2));
            assert_eq!(frame.rgba.len(), 24);
            assert!(decode_file(name, &bytes[..bytes.len() / 2]).is_err());
        }
        assert!(decode_file("fake.jpg", &encoded(ImageFormat::Png)).is_err());
        assert!(decode_file("a.gif", b"GIF89a").is_err());
        assert!(decode_file("../a.png", &encoded(ImageFormat::Png)).is_err());
        assert!(decode_file("a.png", &[]).is_err());
    }
    #[test]
    fn image_dimensions_are_rejected_before_pixel_allocation() {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut encoder = png::Encoder::new(&mut bytes, 8192, 8192);
            encoder.set_color(png::ColorType::Rgba);
            let writer = encoder.write_header().unwrap();
            drop(writer);
        }
        assert!(decode_file("huge.png", bytes.get_ref()).is_err());
        assert!(decode_file("huge.png", &vec![0; MAX_FILE + 1]).is_err());
    }
    #[test]
    fn jpeg_exif_orientation_is_applied_to_the_managed_copy() {
        let original = encoded(ImageFormat::Jpeg);
        let mut exif =
            b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0".to_vec();
        let mut jpeg = vec![0xff, 0xd8, 0xff, 0xe1];
        jpeg.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
        jpeg.append(&mut exif);
        jpeg.extend_from_slice(&original[2..]);
        let frame = decode_file("rotated.jpg", &jpeg).unwrap();
        assert_eq!((frame.width_px, frame.height_px), (2, 3));
    }
    fn dib(bits: u16, top: bool) -> Vec<u8> {
        let header = if bits == 32 { 124 } else { 40 };
        let stride = 8;
        let mut b = vec![0u8; header + stride * 2];
        b[..4].copy_from_slice(&(header as u32).to_le_bytes());
        b[4..8].copy_from_slice(&2i32.to_le_bytes());
        b[8..12].copy_from_slice(&(if top { -2i32 } else { 2i32 }).to_le_bytes());
        b[12..14].copy_from_slice(&1u16.to_le_bytes());
        b[14..16].copy_from_slice(&bits.to_le_bytes());
        if bits == 32 {
            b[16..20].copy_from_slice(&3u32.to_le_bytes());
            for (at, value) in [
                (40, 0x00ff0000u32),
                (44, 0x0000ff00),
                (48, 0x000000ff),
                (52, 0xff000000),
            ] {
                b[at..at + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        b[header..header + 4].copy_from_slice(&[3, 2, 1, 128]);
        b[header + stride..header + stride + 4].copy_from_slice(&[6, 5, 4, 255]);
        b
    }
    #[test]
    fn clipboard_dib_handles_top_down_bottom_up_padding_and_alpha() {
        let a = decode_dib(&dib(32, true)).unwrap();
        assert_eq!(&a.rgba[..4], &[1, 2, 3, 128]);
        let b = decode_dib(&dib(32, false)).unwrap();
        assert_eq!(&b.rgba[..4], &[4, 5, 6, 255]);
        let c = decode_dib(&dib(24, true)).unwrap();
        assert_eq!(&c.rgba[..4], &[1, 2, 3, 255]);
        let mut rgb = dib(32, true);
        rgb[16..20].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(decode_dib(&rgb).unwrap().rgba[3], 128);
        rgb[52..56].fill(0);
        assert_eq!(decode_dib(&rgb).unwrap().rgba[3], 255);
        for data in [&[][..], &dib(32, true)[..60]] {
            assert!(decode_dib(data).is_err());
        }
        let mut invalid = dib(32, true);
        invalid[40..44].fill(0);
        assert!(decode_dib(&invalid).is_err());
        let mut huge = dib(24, true);
        huge[4..8].copy_from_slice(&i32::MAX.to_le_bytes());
        assert!(decode_dib(&huge).is_err());
    }
    fn packet(files: &[(&str, &[u8])]) -> Vec<u8> {
        let header = serde_json::to_vec(&Batch {
            files: files
                .iter()
                .map(|(name, bytes)| FileMeta {
                    name: (*name).into(),
                    size: bytes.len(),
                })
                .collect(),
        })
        .unwrap();
        let mut body = (header.len() as u32).to_le_bytes().to_vec();
        body.extend(header);
        for (_, bytes) in files {
            body.extend_from_slice(bytes);
        }
        body
    }
    #[test]
    fn batch_boundaries_and_individual_file_association() {
        let a = encoded(ImageFormat::Png);
        let b = encoded(ImageFormat::WebP);
        let bytes = packet(&[("first.png", &a), ("second.webp", &b)]);
        let (batch, files) = unpack(&bytes).unwrap();
        assert_eq!(batch.files[1].name, "second.webp");
        assert_eq!(files[0], a);
        assert_eq!(files[1], b);
        assert!(unpack(&packet(&[])).is_err());
        assert!(unpack(&packet(&vec![("x", &b[..]); 11])).is_err());
        let mut malformed = bytes.clone();
        malformed.pop();
        assert!(unpack(&malformed).is_err());
        let mut extra = bytes;
        extra.push(0);
        assert!(unpack(&extra).is_err());
        assert!(unpack(b"bad").is_err());
    }
}
