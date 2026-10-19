use crate::{
    capture::{CaptureError, ErrorCode},
    services::clipper::{ClipperService, Status},
};
use tauri::{AppHandle, State, WebviewWindow};
fn check(window: &WebviewWindow) -> Result<(), CaptureError> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(CaptureError::new(
            ErrorCode::WindowState,
            "此窗口不能配置网页剪藏。",
        ))
    }
}
#[tauri::command]
pub fn clipper_status(
    window: WebviewWindow,
    service: State<'_, ClipperService>,
) -> Result<Status, CaptureError> {
    check(&window)?;
    service.status()
}
#[tauri::command]
pub fn clipper_connection(
    window: WebviewWindow,
    app: AppHandle,
    service: State<'_, ClipperService>,
    enabled: bool,
) -> Result<Status, CaptureError> {
    check(&window)?;
    if enabled {
        service.start(app)
    } else {
        service.stop();
        service.status()
    }
}
