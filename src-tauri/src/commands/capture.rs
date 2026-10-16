use crate::{
    capture::{CaptureError, DisplayInfo, ErrorCode, validate_display_id},
    services::screen_capture::{CapturePermit, ScreenCaptureService},
    services::workflow::{CaptureInfo, CaptureWorkflow},
};
use std::sync::Arc;
use tauri::{State, WebviewWindow};

#[tauri::command]
pub async fn list_displays(
    service: State<'_, Arc<ScreenCaptureService>>,
) -> Result<Vec<DisplayInfo>, CaptureError> {
    let permit = service.try_begin()?;
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        service.enumerate_displays()
    })
    .await
    .map_err(|_| CaptureError::new(ErrorCode::Internal, "显示器枚举任务失败，请重试。"))?
}

pub(super) struct HiddenWindow {
    window: WebviewWindow,
    restore: bool,
    focused: bool,
}
impl HiddenWindow {
    pub(super) fn hide(window: WebviewWindow) -> Result<Self, CaptureError> {
        let visible = window.is_visible().map_err(|_| window_error())?;
        let focused = window.is_focused().map_err(|_| window_error())?;
        let guard = Self {
            window,
            restore: visible,
            focused,
        };
        if visible {
            guard.window.hide().map_err(|_| window_error())?;
        }
        Ok(guard)
    }
    pub(super) fn restore(&mut self) -> Result<(), CaptureError> {
        if self.restore {
            self.window.show().map_err(|_| window_error())?;
            self.restore = false;
            if self.focused {
                self.window.set_focus().map_err(|_| window_error())?;
            }
        }
        Ok(())
    }
}
impl Drop for HiddenWindow {
    fn drop(&mut self) {
        if self.restore().is_err() {
            eprintln!("ResearchClip: main window restoration failed");
        }
    }
}
fn window_error() -> CaptureError {
    CaptureError::new(
        ErrorCode::WindowState,
        "无法隐藏或恢复主窗口，请重新打开窗口后重试。",
    )
}

#[tauri::command]
pub async fn capture_fullscreen(
    window: WebviewWindow,
    service: State<'_, Arc<ScreenCaptureService>>,
    display_id: String,
    workflow: State<'_, Arc<CaptureWorkflow>>,
) -> Result<CaptureInfo, CaptureError> {
    validate_display_id(&display_id)?;
    if window.label() != "main" {
        return Err(CaptureError::new(
            ErrorCode::WindowState,
            "该窗口不能发起全屏截图。",
        ));
    }
    let permit = service.try_begin()?;
    let service = Arc::clone(service.inner());
    let workflow = Arc::clone(workflow.inner());
    run_fullscreen_capture(window, service, workflow, permit, display_id).await
}
pub(super) async fn run_fullscreen_capture(
    window: WebviewWindow,
    service: Arc<ScreenCaptureService>,
    workflow: Arc<CaptureWorkflow>,
    permit: CapturePermit,
    display_id: String,
) -> Result<CaptureInfo, CaptureError> {
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let mut hidden = HiddenWindow::hide(window)?;
        // Windows 11 can leave a fading window in the compositor after hide() returns.
        // Allow that transition to finish; DwmFlush alone does not end the animation.
        // This delay is validated on the recorded desktop, not a universal timing guarantee.
        std::thread::sleep(std::time::Duration::from_millis(350));
        let result = service.capture_display(&display_id);
        hidden.restore()?;
        workflow.store(display_id, "fullscreen", result?)
    })
    .await
    .map_err(|_| {
        CaptureError::new(
            ErrorCode::Internal,
            "截图任务异常结束，主窗口已尝试恢复，请重试。",
        )
    })?
}
