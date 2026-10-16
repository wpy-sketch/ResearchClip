use crate::{
    capture::{CaptureError, ErrorCode},
    services::desktop::{self, DesktopAction, DesktopService, DesktopSnapshot},
};
use tauri::{AppHandle, Manager, WebviewWindow};
fn check(window: &WebviewWindow) -> Result<(), CaptureError> {
    if window.label() != "main" {
        return Err(CaptureError::new(
            ErrorCode::WindowState,
            "该窗口无权操作桌面生命周期。",
        ));
    }
    Ok(())
}
#[tauri::command]
pub fn desktop_status(
    window: WebviewWindow,
    app: AppHandle,
) -> Result<DesktopSnapshot, CaptureError> {
    check(&window)?;
    Ok(app.state::<DesktopService>().snapshot(&app))
}
#[tauri::command]
pub fn desktop_action(
    window: WebviewWindow,
    app: AppHandle,
    action: DesktopAction,
) -> Result<(), CaptureError> {
    check(&window)?;
    desktop::dispatch(&app, action)
}
#[tauri::command]
pub async fn complete_desktop_exit(
    window: WebviewWindow,
    app: AppHandle,
    approved: bool,
) -> Result<(), CaptureError> {
    check(&window)?;
    tauri::async_runtime::spawn_blocking(move || desktop::finish_exit(&app, approved))
        .await
        .map_err(|_| CaptureError::new(ErrorCode::Internal, "退出清理任务异常结束，请重试。"))?
}
