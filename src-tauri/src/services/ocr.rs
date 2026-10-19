//! Local OCR boundary. No paths, persistence, networking or logging of recognized text.
use crate::capture::geometry::{RectPx, crop_frame};
use crate::capture::{CaptureError, ErrorCode, MAX_PNG_BYTES, RawFrame};
use serde::{Deserialize, Serialize};

pub const MAX_TEXT_BYTES: usize = 1_048_576;
fn failure(message: &str) -> CaptureError {
    CaptureError::new(ErrorCode::CaptureFailed, message)
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrLanguage {
    pub tag: String,
    pub name: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrStatus {
    pub languages: Vec<OcrLanguage>,
    pub max_dimension: u32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OcrOptions {
    pub language: String,
    pub region: Option<OcrRegion>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OcrRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrResult {
    pub text: String,
    pub language: String,
    pub width_px: u32,
    pub height_px: u32,
}

pub fn unpack(bytes: &[u8]) -> Result<(OcrOptions, &[u8]), CaptureError> {
    if bytes.len() < 5 || bytes.len() > MAX_PNG_BYTES + 1028 {
        return Err(failure("OCR 图片数据无效或过大。"));
    }
    let length = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
    if length == 0 || length > 1024 || length + 4 >= bytes.len() {
        return Err(failure("OCR 请求格式无效。"));
    }
    let options: OcrOptions = serde_json::from_slice(&bytes[4..4 + length])
        .map_err(|_| failure("OCR 语言或选区无效。"))?;
    if options.language.is_empty()
        || options.language.len() > 64
        || !options
            .language
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
    {
        return Err(failure("请选择已安装的 OCR 语言。"));
    }
    Ok((options, &bytes[4 + length..]))
}
pub fn prepare(
    bytes: &[u8],
    region: Option<OcrRegion>,
    max_dimension: u32,
) -> Result<RawFrame, CaptureError> {
    let mut frame = super::workflow::decode_png(bytes)?;
    if let Some(r) = region {
        frame = crop_frame(
            frame,
            RectPx {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
            },
        )?;
    }
    if frame.width_px > max_dimension || frame.height_px > max_dimension {
        return Err(failure(&format!(
            "图片或选区超过本机 OCR 单边 {max_dimension} 像素上限，请框选较小区域。"
        )));
    }
    // Native OCR expects BGRA8. Composite transparency onto white without changing the source PNG.
    for p in frame.rgba.as_chunks_mut::<4>().0 {
        let a = u32::from(p[3]);
        let blend = |v: u8| ((u32::from(v) * a + 255 * (255 - a) + 127) / 255) as u8;
        let r = blend(p[0]);
        let g = blend(p[1]);
        let b = blend(p[2]);
        p.copy_from_slice(&[b, g, r, 255]);
    }
    Ok(frame)
}
pub fn validate_text(text: &str) -> Result<(), CaptureError> {
    if text.len() > MAX_TEXT_BYTES || text.contains('\0') {
        return Err(failure("识别文字超过 1 MiB 或包含无效字符。"));
    }
    Ok(())
}
fn cjk(c: char) -> bool {
    matches!(c as u32,0x3400..=0x4dbf|0x4e00..=0x9fff|0xf900..=0xfaff|0x20000..=0x323af|0x3000..=0x303f|0xff01..=0xff60)
}
fn clean_cjk_spacing(text: &str) -> String {
    let mut result = String::new();
    let mut spaces = 0;
    for c in text.chars() {
        if c == ' ' {
            spaces += 1;
            continue;
        }
        if spaces > 0
            && !result.is_empty()
            && !result
                .chars()
                .next_back()
                .is_some_and(|previous| cjk(previous) && cjk(c))
        {
            result.extend(std::iter::repeat_n(' ', spaces));
        }
        result.push(c);
        spaces = 0;
    }
    result
}
// Respect OCR reading order and line breaks; a large vertical gap becomes a paragraph.
pub fn assemble_lines(lines: Vec<(String, f32, f32)>) -> Result<String, CaptureError> {
    let mut result = String::new();
    let mut previous: Option<(f32, f32)> = None;
    for (text, top, height) in lines {
        if text.trim().is_empty() {
            continue;
        }
        if !result.is_empty() {
            result.push('\n');
            if let Some((bottom, h)) = previous
                && top.is_finite()
                && h > 0.0
                && top - bottom > h * 0.75
            {
                result.push('\n');
            }
        }
        result.push_str(&clean_cjk_spacing(text.trim()));
        validate_text(&result)?;
        previous = Some((top + height, height));
    }
    Ok(result)
}
/// Platform-neutral service interface; future macOS implementation belongs behind this boundary.
pub struct OcrService;
impl OcrService {
    pub fn status() -> Result<OcrStatus, CaptureError> {
        #[cfg(target_os = "windows")]
        {
            native::status()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(CaptureError::new(
                ErrorCode::Unsupported,
                "本版本 OCR 仅支持 Windows；macOS 尚未实现。",
            ))
        }
    }
    pub fn recognize(bytes: &[u8]) -> Result<OcrResult, CaptureError> {
        Self::recognize_controlled(bytes, || false)
    }
    pub fn recognize_controlled(
        bytes: &[u8],
        cancelled: impl Fn() -> bool,
    ) -> Result<OcrResult, CaptureError> {
        let (options, png) = unpack(bytes)?;
        #[cfg(target_os = "windows")]
        {
            native::recognize(png, options, &cancelled)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (options, png, cancelled);
            Self::status()?;
            unreachable!()
        }
    }
}
#[cfg(target_os = "windows")]
mod native {
    use super::*;
    use windows::{
        Globalization::Language,
        Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap},
        Media::Ocr::OcrEngine,
        Storage::Streams::DataWriter,
    };
    // windows-core 0.62.2 load_factory pins the process MTA when necessary.
    // Do not tear down the last apartment after each call: the projection keeps
    // agile activation factories cached across calls and worker threads.
    fn error(_: windows::core::Error) -> CaptureError {
        failure("Windows OCR 识别失败。请确认已安装相应 OCR 语言包，尝试较清晰或较小的图片。")
    }
    fn capabilities() -> Result<OcrStatus, CaptureError> {
        let mut languages = Vec::new();
        for language in OcrEngine::AvailableRecognizerLanguages().map_err(error)? {
            languages.push(OcrLanguage {
                tag: language.LanguageTag().map_err(error)?.to_string(),
                name: language.DisplayName().map_err(error)?.to_string(),
            });
        }
        Ok(OcrStatus {
            languages,
            max_dimension: OcrEngine::MaxImageDimension().map_err(error)?,
        })
    }
    pub fn status() -> Result<OcrStatus, CaptureError> {
        capabilities()
    }
    pub fn recognize(
        png: &[u8],
        options: OcrOptions,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<OcrResult, CaptureError> {
        let status = capabilities()?;
        if !status
            .languages
            .iter()
            .any(|l| l.tag.eq_ignore_ascii_case(&options.language))
        {
            return Err(failure(
                "所选 OCR 语言包不可用，请在 Windows 设置的语言选项中安装文字识别组件后重试。",
            ));
        }
        if cancelled() {
            return Err(failure("识别已取消。"));
        }
        let frame = prepare(png, options.region, status.max_dimension)?;
        let language = Language::CreateLanguage(&options.language.into()).map_err(error)?;
        let engine = OcrEngine::TryCreateFromLanguage(&language).map_err(error)?;
        let writer = DataWriter::new().map_err(error)?;
        let buffer = (|| {
            writer.WriteBytes(&frame.rgba)?;
            writer.DetachBuffer()
        })();
        let _ = writer.Close();
        let buffer = buffer.map_err(error)?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            frame.width_px as i32,
            frame.height_px as i32,
        )
        .map_err(error)?;
        drop(buffer);
        drop(frame.rgba);
        let recognized = (|| {
            // Keep SoftwareBitmap alive until the native operation completes. Cancellation
            // is cooperative between images; closing it immediately after Cancel can race
            // Windows OCR's native reader. The caller discards a cancelled result.
            let result = engine.RecognizeAsync(&bitmap)?.join()?;
            let mut lines = Vec::new();
            for line in result.Lines()? {
                let mut top = f32::INFINITY;
                let mut bottom = f32::NEG_INFINITY;
                for word in line.Words()? {
                    let rect = word.BoundingRect()?;
                    top = top.min(rect.Y);
                    bottom = bottom.max(rect.Y + rect.Height);
                }
                lines.push((line.Text()?.to_string(), top, bottom - top));
            }
            Ok::<_, windows::core::Error>(lines)
        })();
        let _ = bitmap.Close();
        Ok(OcrResult {
            text: assemble_lines(recognized.map_err(error)?)?,
            language: language.LanguageTag().map_err(error)?.to_string(),
            width_px: frame.width_px,
            height_px: frame.height_px,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chinese_word_spacing_does_not_destroy_latin_words_or_mixed_boundaries() {
        assert_eq!(
            clean_cjk_spacing("科 研 笔 记 。 English words 中 文 local only"),
            "科研笔记。 English words 中文 local only"
        );
    }
    #[test]
    fn validates_upload_before_decoding() {
        for bytes in [vec![], vec![0; 4], vec![255; 8]] {
            assert!(unpack(&bytes).is_err());
        }
        let mut b = Vec::from(2_u32.to_le_bytes());
        b.extend_from_slice(b"{}png");
        assert!(unpack(&b).is_err());
    }
    #[test]
    fn lines_unicode_paragraphs_and_limits() {
        assert_eq!(
            assemble_lines(vec![
                ("中文 Alpha ±".into(), 0., 20.),
                ("第二行".into(), 25., 20.),
                ("new paragraph".into(), 80., 20.)
            ])
            .unwrap(),
            "中文 Alpha ±\n第二行\n\nnew paragraph"
        );
        assert_eq!(assemble_lines(vec![]).unwrap(), "");
        assert!(validate_text(&"a".repeat(MAX_TEXT_BYTES + 1)).is_err());
        assert!(validate_text("a\0").is_err());
    }
    #[test]
    fn crop_bgra_alpha_and_size() {
        let png = super::super::screen_capture::encode_png(RawFrame {
            width_px: 2,
            height_px: 1,
            rgba: vec![255, 0, 0, 255, 0, 0, 255, 0],
        })
        .unwrap();
        let f = prepare(
            &png,
            Some(OcrRegion {
                x: 1,
                y: 0,
                width: 1,
                height: 1,
            }),
            1,
        )
        .unwrap();
        assert_eq!(f.rgba, vec![255; 4]);
        assert_eq!(prepare(&png, None, 2).unwrap().rgba[..4], [0, 0, 255, 255]);
        assert!(prepare(&png, None, 1).is_err());
        assert!(
            prepare(
                &png,
                Some(OcrRegion {
                    x: 2,
                    y: 0,
                    width: 1,
                    height: 1
                }),
                2
            )
            .is_err()
        );
        assert!(prepare(b"not png", None, 2).is_err());
    }
}
