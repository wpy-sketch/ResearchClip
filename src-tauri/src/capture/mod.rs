use serde::Serialize;
pub mod geometry;

#[cfg(target_os = "windows")]
mod windows;

pub const MAX_PIXELS: u64 = 16_777_216;
pub const MAX_PNG_BYTES: usize = 16_777_216 * 4 + 1_048_576;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    /// Current Windows display device name. Re-enumerated on every capture, never a path.
    pub id: String,
    pub name: String,
    pub width_px: u32,
    pub height_px: u32,
    pub x_px: i32,
    pub y_px: i32,
    pub is_primary: bool,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Unsupported,
    InvalidDisplay,
    NoDisplays,
    EnumerationFailed,
    DisplayGone,
    DisplayChanged,
    PermissionDenied,
    CaptureFailed,
    ResourceLimit,
    Busy,
    EncodeFailed,
    WindowState,
    Internal,
    InvalidSelection,
    ZeroArea,
    InvalidSession,
    Cancelled,
    Timeout,
    SaveFailed,
    ClipboardUnavailable,
    InvalidSettings,
    SettingsUnavailable,
    ShortcutConflict,
    FileExists,
}

#[derive(Debug, Serialize)]
pub struct CaptureError {
    pub code: ErrorCode,
    pub message: String,
}

impl CaptureError {
    pub fn new(code: ErrorCode, message: &str) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for CaptureError {}

// Intentionally no Debug implementation: pixels must not appear in ordinary logs.
pub struct RawFrame {
    pub width_px: u32,
    pub height_px: u32,
    pub rgba: Vec<u8>,
}

pub fn rgba_len(width: u32, height: u32) -> Result<usize, CaptureError> {
    let pixels = u64::from(width) * u64::from(height);
    if width == 0
        || height == 0
        || width > i32::MAX as u32
        || height > i32::MAX as u32
        || pixels > MAX_PIXELS
    {
        return Err(CaptureError::new(
            ErrorCode::ResourceLimit,
            "图像尺寸无效或超过 16,777,216 像素的捕获上限。",
        ));
    }
    usize::try_from(pixels * 4)
        .map_err(|_| CaptureError::new(ErrorCode::ResourceLimit, "图像超出可用内存范围。"))
}

pub fn validate_display_id(id: &str) -> Result<(), CaptureError> {
    if id.is_empty() || id.len() > 256 || id.chars().any(char::is_control) {
        return Err(CaptureError::new(
            ErrorCode::InvalidDisplay,
            "显示器标识无效，请重新选择显示器。",
        ));
    }
    Ok(())
}

pub fn enumerate_displays() -> Result<Vec<DisplayInfo>, CaptureError> {
    #[cfg(target_os = "windows")]
    {
        windows::enumerate_displays()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(CaptureError::new(
            ErrorCode::Unsupported,
            "本轮截图模块仅支持 Windows。",
        ))
    }
}

pub fn capture_display(id: &str) -> Result<RawFrame, CaptureError> {
    validate_display_id(id)?;
    #[cfg(target_os = "windows")]
    {
        windows::capture_display(id)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(CaptureError::new(
            ErrorCode::Unsupported,
            "本轮截图模块仅支持 Windows。",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_are_checked_before_allocation() {
        assert_eq!(rgba_len(3840, 2160).unwrap(), 33_177_600);
        assert_eq!(rgba_len(4096, 4096).unwrap(), 67_108_864);
        for (w, h) in [(0, 1), (1, 0), (4097, 4096), (u32::MAX, u32::MAX)] {
            assert_eq!(rgba_len(w, h).unwrap_err().code, ErrorCode::ResourceLimit);
        }
    }

    #[test]
    fn ids_and_dto_do_not_expose_handles_or_paths() {
        for id in ["", "\nDISPLAY1"] {
            assert!(validate_display_id(id).is_err());
        }
        assert!(validate_display_id(&"x".repeat(257)).is_err());
        assert!(validate_display_id(r"\\.\DISPLAY1").is_ok());
        let value = serde_json::to_value(DisplayInfo {
            id: "display-1".into(),
            name: "显示器 1".into(),
            width_px: 1920,
            height_px: 1080,
            x_px: -1920,
            y_px: 0,
            is_primary: false,
        })
        .unwrap();
        assert_eq!(value["widthPx"], 1920);
        assert_eq!(value["xPx"], -1920);
        assert!(value.get("handle").is_none());
        assert_eq!(
            serde_json::to_value(CaptureError::new(ErrorCode::DisplayGone, "显示器已断开"))
                .unwrap()["code"],
            "DISPLAY_GONE"
        );
    }
}
