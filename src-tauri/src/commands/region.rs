use super::capture::HiddenWindow;
use crate::{
    capture::{
        self, CaptureError, DisplayInfo, ErrorCode,
        geometry::{Selection, crop_frame, selection_to_pixels},
        validate_display_id,
    },
    services::{
        region_capture::{Decision, OverlayInfo, RegionCaptureService, overlay_label},
        screen_capture::{CapturePermit, ScreenCaptureService, encode_png, encode_png_ref},
        workflow::{CaptureInfo, CaptureWorkflow},
    },
};
use std::{
    sync::{Arc, mpsc::RecvTimeoutError},
    time::{Duration, Instant},
};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent, ipc::Response,
};

fn window_error() -> CaptureError {
    CaptureError::new(ErrorCode::WindowState, "无法正确建立或关闭截图选择窗口。")
}
fn cancelled() -> CaptureError {
    CaptureError::new(ErrorCode::Cancelled, "已取消区域截图。")
}
fn current_display(id: &str) -> Result<DisplayInfo, CaptureError> {
    capture::enumerate_displays()?
        .into_iter()
        .find(|d| d.id == id)
        .ok_or_else(|| CaptureError::new(ErrorCode::DisplayGone, "目标显示器已断开。"))
}

struct OverlayGuard {
    app: AppHandle,
    service: Arc<RegionCaptureService>,
    id: String,
    closed: bool,
}
impl OverlayGuard {
    fn close(&mut self) -> Result<(), CaptureError> {
        if self.closed {
            return Ok(());
        }
        let label = overlay_label(&self.id);
        if let Some(window) = self.app.get_webview_window(&label) {
            window.destroy().map_err(|_| window_error())?;
        }
        let start = Instant::now();
        while self.app.get_webview_window(&label).is_some()
            && start.elapsed() < Duration::from_secs(2)
        {
            std::thread::sleep(Duration::from_millis(10));
        }
        if self.app.get_webview_window(&label).is_some() {
            return Err(window_error());
        }
        self.service.finish(&self.id);
        self.closed = true;
        Ok(())
    }
}
impl Drop for OverlayGuard {
    fn drop(&mut self) {
        if self.close().is_err() {
            if let Some(window) = self.app.get_webview_window(&overlay_label(&self.id)) {
                let _ = window.hide();
            }
            self.service.finish(&self.id);
            eprintln!("ResearchClip: region overlay cleanup failed");
        }
    }
}

#[tauri::command]
pub async fn capture_region(
    window: WebviewWindow,
    app: AppHandle,
    captures: State<'_, Arc<ScreenCaptureService>>,
    regions: State<'_, Arc<RegionCaptureService>>,
    display_id: String,
) -> Result<CaptureInfo, CaptureError> {
    run_region_capture(
        window,
        app,
        captures.try_begin()?,
        Arc::clone(regions.inner()),
        display_id,
    )
    .await
}

// Buttons and the native global shortcut share the same gate and window lifecycle.
pub(crate) async fn run_region_capture(
    window: WebviewWindow,
    app: AppHandle,
    permit: CapturePermit,
    regions: Arc<RegionCaptureService>,
    display_id: String,
) -> Result<CaptureInfo, CaptureError> {
    validate_display_id(&display_id)?;
    if window.label() != "main" {
        return Err(window_error());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let mut hidden = HiddenWindow::hide(window)?;
        let result = (|| {
            std::thread::sleep(Duration::from_millis(350));
            let display = current_display(&display_id)?;
            // Freeze native pixels BEFORE any overlay window exists. Never capture again on commit.
            let frame = capture::capture_display(&display_id)?;
            if current_display(&display_id)? != display
                || (frame.width_px, frame.height_px) != (display.width_px, display.height_px)
            {
                return Err(CaptureError::new(
                    ErrorCode::DisplayChanged,
                    "显示器配置已变化，请重新截图。",
                ));
            }
            let preview = encode_png_ref(&frame)?;
            if app.get_webview_window("main").is_none() {
                return Err(cancelled());
            }
            let (info, receiver) = regions.begin(display.clone(), preview)?;
            let label = overlay_label(&info.session_id);
            let mut overlay_guard = OverlayGuard {
                app: app.clone(),
                service: Arc::clone(&regions),
                id: info.session_id.clone(),
                closed: false,
            };
            let overlay = WebviewWindowBuilder::new(
                &app,
                &label,
                WebviewUrl::App(format!("index.html?captureOverlay={}", info.session_id).into()),
            )
            .title("ResearchClip 区域截图")
            .decorations(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(false)
            .focused(false)
            .build()
            .map_err(|_| window_error())?;
            overlay
                .set_position(PhysicalPosition::new(display.x_px, display.y_px))
                .map_err(|_| window_error())?;
            overlay
                .set_size(PhysicalSize::new(display.width_px, display.height_px))
                .map_err(|_| window_error())?;
            let events = Arc::clone(&regions);
            let event_id = info.session_id.clone();
            let expected = (display.width_px, display.height_px);
            let expected_position = (display.x_px, display.y_px);
            overlay.on_window_event(move |event| match event {
                WindowEvent::Destroyed | WindowEvent::CloseRequested { .. } => events.fail(
                    &event_id,
                    CaptureError::new(ErrorCode::Cancelled, "截图选择层已关闭，区域截图已取消。"),
                ),
                WindowEvent::Focused(false) if events.is_ready(&event_id) => events.fail(
                    &event_id,
                    CaptureError::new(ErrorCode::Cancelled, "截图选择层失去焦点，区域截图已取消。"),
                ),
                WindowEvent::Moved(position)
                    if events.is_ready(&event_id)
                        && (position.x, position.y) != expected_position =>
                {
                    events.fail(
                        &event_id,
                        CaptureError::new(
                            ErrorCode::DisplayChanged,
                            "选择窗口位置已变化，请重新截图。",
                        ),
                    )
                }
                WindowEvent::ScaleFactorChanged { .. } if events.is_ready(&event_id) => events
                    .fail(
                        &event_id,
                        CaptureError::new(
                            ErrorCode::DisplayChanged,
                            "选择窗口缩放已变化，请重新截图。",
                        ),
                    ),
                WindowEvent::Resized(size)
                    if events.is_ready(&event_id) && (size.width, size.height) != expected =>
                {
                    events.fail(
                        &event_id,
                        CaptureError::new(
                            ErrorCode::DisplayChanged,
                            "选择窗口尺寸已变化，请重新截图。",
                        ),
                    )
                }
                _ => {}
            });
            let start = Instant::now();
            let mut topology_checked = Instant::now();
            let selected = loop {
                match receiver.recv_timeout(Duration::from_millis(200)) {
                    Ok(Decision::Selected(rect)) => break rect,
                    Ok(Decision::Failed(error)) => return Err(error),
                    Err(RecvTimeoutError::Disconnected) => return Err(cancelled()),
                    Err(RecvTimeoutError::Timeout) => {}
                }
                let ready = regions.is_ready(&info.session_id);
                if (!ready && start.elapsed() > Duration::from_secs(15))
                    || start.elapsed() > Duration::from_secs(300)
                {
                    return Err(CaptureError::new(
                        ErrorCode::Timeout,
                        "截图选择层未响应或会话已过期，请重试。",
                    ));
                }
                if topology_checked.elapsed() > Duration::from_secs(1) {
                    if current_display(&display_id)? != display {
                        return Err(CaptureError::new(
                            ErrorCode::DisplayChanged,
                            "截图期间显示器配置变化，请重新截图。",
                        ));
                    }
                    topology_checked = Instant::now();
                }
            };
            if current_display(&display_id)? != display {
                return Err(CaptureError::new(
                    ErrorCode::DisplayChanged,
                    "显示器配置已变化，请重新截图。",
                ));
            }
            let png = encode_png(crop_frame(frame, selected)?)?;
            overlay_guard.close()?;
            app.state::<Arc<CaptureWorkflow>>()
                .store(display_id, "region", png)
        })();
        hidden.restore()?;
        result
    })
    .await
    .map_err(|_| {
        CaptureError::new(
            ErrorCode::Internal,
            "区域截图任务异常结束，已尝试关闭选择层并恢复主窗口。",
        )
    })?
}

#[tauri::command]
pub fn region_overlay_info(
    window: WebviewWindow,
    regions: State<'_, Arc<RegionCaptureService>>,
    session_id: String,
) -> Result<OverlayInfo, CaptureError> {
    regions.info(&session_id, window.label())
}

#[tauri::command]
pub fn region_overlay_image(
    window: WebviewWindow,
    regions: State<'_, Arc<RegionCaptureService>>,
    session_id: String,
) -> Result<Response, CaptureError> {
    Ok(Response::new(
        regions.take_preview(&session_id, window.label())?,
    ))
}

#[tauri::command]
pub fn ready_region_overlay(
    window: WebviewWindow,
    regions: State<'_, Arc<RegionCaptureService>>,
    session_id: String,
    viewport_width: f64,
    viewport_height: f64,
) -> Result<(), CaptureError> {
    let info = regions.info(&session_id, window.label())?;
    let size = window.inner_size().map_err(|_| window_error())?;
    let position = window.outer_position().map_err(|_| window_error())?;
    let scale = window.scale_factor().map_err(|_| window_error())?;
    if (size.width, size.height) != (info.display.width_px, info.display.height_px)
        || (position.x, position.y) != (info.display.x_px, info.display.y_px)
        || !viewport_width.is_finite()
        || !viewport_height.is_finite()
        || (viewport_width * scale - f64::from(size.width)).abs() > 2.0
        || (viewport_height * scale - f64::from(size.height)).abs() > 2.0
    {
        return Err(CaptureError::new(
            ErrorCode::DisplayChanged,
            "选择窗口与显示器像素边界不一致，请重试。",
        ));
    }
    window.show().map_err(|_| window_error())?;
    window.set_focus().map_err(|_| window_error())?;
    regions.mark_ready(&session_id, window.label(), viewport_width, viewport_height)
}

#[tauri::command]
pub fn commit_region_selection(
    window: WebviewWindow,
    regions: State<'_, Arc<RegionCaptureService>>,
    session_id: String,
    selection: Selection,
) -> Result<(), CaptureError> {
    let info = regions.info(&session_id, window.label())?;
    let viewport = regions.viewport(&session_id, window.label())?;
    if (selection.viewport_width, selection.viewport_height) != viewport {
        return Err(CaptureError::new(
            ErrorCode::DisplayChanged,
            "选择窗口尺寸已改变，请取消后重试。",
        ));
    }
    let size = window.inner_size().map_err(|_| window_error())?;
    let position = window.outer_position().map_err(|_| window_error())?;
    if (size.width, size.height) != (info.display.width_px, info.display.height_px)
        || (position.x, position.y) != (info.display.x_px, info.display.y_px)
    {
        return Err(window_error());
    }
    let rect = selection_to_pixels(selection, info.display.width_px, info.display.height_px)?;
    regions.complete(&session_id, window.label(), rect)
}

#[tauri::command]
pub fn cancel_region_selection(
    window: WebviewWindow,
    regions: State<'_, Arc<RegionCaptureService>>,
    session_id: String,
    reason: Option<String>,
) -> Result<(), CaptureError> {
    regions.info(&session_id, window.label())?;
    let error = match reason.as_deref() {
        None | Some("user") => cancelled(),
        Some("error") => CaptureError::new(
            ErrorCode::CaptureFailed,
            "截图选择层初始化或提交失败，请重新截图。",
        ),
        Some("viewport_changed") => CaptureError::new(
            ErrorCode::DisplayChanged,
            "选择窗口尺寸已变化，请重新截图。",
        ),
        _ => {
            return Err(CaptureError::new(
                ErrorCode::InvalidSelection,
                "取消原因无效。",
            ));
        }
    };
    regions.fail(&session_id, error);
    Ok(())
}

#[tauri::command]
pub async fn region_result_png(
    window: WebviewWindow,
    workflow: State<'_, Arc<CaptureWorkflow>>,
    capture_id: String,
) -> Result<Response, CaptureError> {
    if window.label() != "main" {
        return Err(window_error());
    }
    Ok(Response::new(workflow.get(&capture_id)?.png.clone()))
}
