use crate::{
    capture::{CaptureError, ErrorCode, MAX_PNG_BYTES},
    services::{
        ocr::{OcrResult, OcrService, OcrStatus, validate_text},
        screen_capture::ScreenCaptureService,
    },
};
use std::sync::Arc;
use tauri::{
    AppHandle, State, WebviewWindow,
    ipc::{InvokeBody, Request},
};
use tauri_plugin_clipboard_manager::ClipboardExt;
fn check(window: &WebviewWindow) -> Result<(), CaptureError> {
    if window.label() != "main" {
        return Err(CaptureError::new(
            ErrorCode::WindowState,
            "该窗口无权进行文字识别。",
        ));
    }
    Ok(())
}
fn task_error() -> CaptureError {
    CaptureError::new(ErrorCode::Internal, "OCR 操作异常结束，请重试。")
}
#[tauri::command]
pub async fn ocr_status(
    window: WebviewWindow,
    captures: State<'_, Arc<ScreenCaptureService>>,
) -> Result<OcrStatus, CaptureError> {
    check(&window)?;
    let permit = captures.try_begin()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        OcrService::status()
    })
    .await
    .map_err(|_| task_error())?
}
#[tauri::command]
pub async fn recognize_ocr(
    window: WebviewWindow,
    captures: State<'_, Arc<ScreenCaptureService>>,
    request: Request<'_>,
) -> Result<OcrResult, CaptureError> {
    check(&window)?;
    let permit = captures.try_begin()?;
    let bytes = match request.body() {
        InvokeBody::Raw(bytes) if bytes.len() <= MAX_PNG_BYTES + 1028 => bytes.clone(),
        _ => {
            return Err(CaptureError::new(
                ErrorCode::ResourceLimit,
                "OCR 上传数据无效或过大。",
            ));
        }
    };
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        OcrService::recognize(&bytes)
    })
    .await
    .map_err(|_| task_error())?
}
#[tauri::command]
pub async fn copy_ocr_text(
    window: WebviewWindow,
    app: AppHandle,
    captures: State<'_, Arc<ScreenCaptureService>>,
    text: String,
) -> Result<(), CaptureError> {
    check(&window)?;
    validate_text(&text)?;
    let permit = captures.try_begin()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        app.clipboard().write_text(text).map_err(|_| {
            CaptureError::new(
                ErrorCode::ClipboardUnavailable,
                "无法复制识别文字，剪贴板可能正被其他应用占用，请稍后重试。",
            )
        })
    })
    .await
    .map_err(|_| task_error())?
}
