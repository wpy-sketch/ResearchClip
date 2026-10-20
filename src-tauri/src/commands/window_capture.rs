use super::capture::HiddenWindow;
use crate::{
    capture::{self, CaptureError, ErrorCode},
    services::{
        screen_capture::{ScreenCaptureService, encode_png},
        window_capture::{Decision, PickerInfo, WindowCaptureService, picker_label},
        workflow::{CaptureInfo, CaptureWorkflow},
    },
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::RecvTimeoutError,
    },
    time::{Duration, Instant},
};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
fn failure() -> CaptureError {
    CaptureError::new(ErrorCode::WindowState, "无法创建或关闭窗口选择层，请重试。")
}
fn cancelled() -> CaptureError {
    CaptureError::new(ErrorCode::Cancelled, "已取消窗口截图。")
}
struct PickerGuard {
    app: AppHandle,
    service: Arc<WindowCaptureService>,
    id: String,
}
impl PickerGuard {
    fn close_window(&self) -> Result<(), CaptureError> {
        let label = picker_label(&self.id);
        if let Some(window) = self.app.get_webview_window(&label) {
            window.destroy().map_err(|_| failure())?;
        }
        let start = Instant::now();
        while self.app.get_webview_window(&label).is_some()
            && start.elapsed() < Duration::from_secs(2)
        {
            std::thread::sleep(Duration::from_millis(10));
        }
        if self.app.get_webview_window(&label).is_some() {
            return Err(failure());
        }
        Ok(())
    }
}
impl Drop for PickerGuard {
    fn drop(&mut self) {
        if self.close_window().is_err() {
            if let Some(window) = self.app.get_webview_window(&picker_label(&self.id)) {
                let _ = window.hide();
            }
            eprintln!("ResearchClip: window picker cleanup failed");
        }
        self.service.finish(&self.id);
    }
}
#[tauri::command]
pub async fn capture_window(
    window: WebviewWindow,
    app: AppHandle,
    captures: State<'_, Arc<ScreenCaptureService>>,
    windows: State<'_, Arc<WindowCaptureService>>,
) -> Result<CaptureInfo, CaptureError> {
    if window.label() != "main" {
        return Err(failure());
    }
    let permit = captures.try_begin()?;
    let service = Arc::clone(windows.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let mut hidden = HiddenWindow::hide(window)?;
        let result = (|| {
            let displays = capture::enumerate_displays()?;
            let (info, receiver, cancel) = service.begin(&displays)?;
            let guard = PickerGuard {
                app: app.clone(),
                service: Arc::clone(&service),
                id: info.session_id.clone(),
            };
            let ready = Arc::new(AtomicBool::new(false));
            let overlay = WebviewWindowBuilder::new(
                &app,
                picker_label(&info.session_id),
                WebviewUrl::App(format!("index.html?windowPicker={}", info.session_id).into()),
            )
            .title("ResearchClip 窗口截图")
            .decorations(false)
            .shadow(false)
            .transparent(true)
            .background_color(tauri::window::Color(0, 0, 0, 0))
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(false)
            .focused(false)
            .build()
            .map_err(|_| failure())?;
            overlay
                .set_position(PhysicalPosition::new(info.desktop.x, info.desktop.y))
                .map_err(|_| failure())?;
            overlay
                .set_size(PhysicalSize::new(info.desktop.width, info.desktop.height))
                .map_err(|_| failure())?;
            let events = Arc::clone(&service);
            let event_id = info.session_id.clone();
            let event_ready = Arc::clone(&ready);
            let bounds = info.desktop;
            overlay.on_window_event(move |event| match event {
                WindowEvent::Destroyed | WindowEvent::CloseRequested { .. } => {
                    events.cancel_pending(&event_id)
                }
                WindowEvent::Focused(false) if event_ready.load(Ordering::Acquire) => {
                    events.cancel_pending(&event_id)
                }
                WindowEvent::Moved(p)
                    if event_ready.load(Ordering::Acquire)
                        && (p.x, p.y) != (bounds.x, bounds.y) =>
                {
                    events.cancel_pending(&event_id)
                }
                WindowEvent::Resized(s)
                    if event_ready.load(Ordering::Acquire)
                        && (s.width, s.height) != (bounds.width, bounds.height) =>
                {
                    events.cancel_pending(&event_id)
                }
                _ => {}
            });
            // The page announces readiness through window_picker_action. Its IPC origin is checked there.
            let start = Instant::now();
            let mut shown = false;
            let mut topology = Instant::now();
            let target = loop {
                match receiver.recv_timeout(Duration::from_millis(60)) {
                    Ok(Decision::Selected(target)) => break target,
                    Ok(Decision::Cancelled) => return Err(cancelled()),
                    Err(RecvTimeoutError::Disconnected) => return Err(cancelled()),
                    Err(RecvTimeoutError::Timeout) => {}
                }
                // ready means the webview completed its first successful status request.
                if !shown && service.is_ready(&info.session_id) {
                    overlay.show().map_err(|_| failure())?;
                    overlay.set_focus().map_err(|_| failure())?;
                    if !overlay.is_focused().map_err(|_| failure())? {
                        return Err(CaptureError::new(
                            ErrorCode::WindowState,
                            "Windows 未允许选择层获得焦点，请重试。",
                        ));
                    }
                    ready.store(true, Ordering::Release);
                    shown = true;
                }
                if start.elapsed() > Duration::from_secs(120)
                    || (!shown && start.elapsed() > Duration::from_secs(15))
                {
                    return Err(CaptureError::new(
                        ErrorCode::Timeout,
                        "窗口选择超时，请重新开始。",
                    ));
                }
                if topology.elapsed() > Duration::from_secs(1) {
                    if capture::enumerate_displays()? != displays {
                        return Err(CaptureError::new(
                            ErrorCode::DisplayChanged,
                            "显示器布局已变化，请重新选择窗口。",
                        ));
                    }
                    topology = Instant::now();
                }
            };
            guard.close_window()?;
            if cancel.load(Ordering::Acquire) {
                return Err(cancelled());
            }
            let frame = service.capture(&target, &cancel)?;
            let png = encode_png(frame)?;
            if cancel.load(Ordering::Acquire) {
                return Err(cancelled());
            }
            app.state::<Arc<CaptureWorkflow>>()
                .store("window".into(), "window", png)
        })();
        hidden.restore()?;
        result
    })
    .await
    .map_err(|_| {
        CaptureError::new(
            ErrorCode::Internal,
            "窗口截图任务异常结束，已尝试恢复主窗口。",
        )
    })?
}
#[tauri::command]
pub async fn window_picker_status(
    window: WebviewWindow,
    windows: State<'_, Arc<WindowCaptureService>>,
    session_id: String,
) -> Result<PickerInfo, CaptureError> {
    let label = window.label().to_owned();
    let windows = Arc::clone(windows.inner());
    tauri::async_runtime::spawn_blocking(move || windows.status(&session_id, &label))
        .await
        .map_err(|_| failure())?
}
#[tauri::command]
pub fn window_picker_action(
    window: WebviewWindow,
    windows: State<'_, Arc<WindowCaptureService>>,
    session_id: String,
    action: String,
) -> Result<(), CaptureError> {
    windows.action(&session_id, window.label(), &action)
}
