use crate::{
    capture::{CaptureError, ErrorCode},
    services::{
        region_capture::RegionCaptureService,
        screen_capture::ScreenCaptureService,
        settings::{SettingsService, ShortcutAction},
    },
};
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU8, Ordering},
};
use tauri::{
    AppHandle, Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
pub const TRAY_ID: &str = "researchclip-tray";
#[derive(Default)]
pub struct DesktopService {
    phase: AtomicU8,
    error: Mutex<Option<String>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSnapshot {
    pub tray_available: bool,
    pub tray_count: u8,
    pub exit_pending: bool,
    pub error: Option<String>,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DesktopAction {
    Open,
    Settings,
    Hide,
    Exit,
    Region,
    Fullscreen,
}
impl DesktopService {
    pub fn running(&self) -> bool {
        self.phase.load(Ordering::Acquire) == 0
    }
    pub fn closing(&self) -> bool {
        self.phase.load(Ordering::Acquire) == 2
    }
    fn request(&self) -> bool {
        self.phase
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
    fn cancel(&self) {
        let _ = self
            .phase
            .compare_exchange(1, 0, Ordering::AcqRel, Ordering::Acquire);
    }
    fn commit(&self) -> bool {
        self.phase
            .compare_exchange(1, 2, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
    pub fn snapshot(&self, app: &AppHandle) -> DesktopSnapshot {
        let available = app.tray_by_id(TRAY_ID).is_some();
        DesktopSnapshot {
            tray_available: available,
            tray_count: u8::from(available),
            exit_pending: !self.running(),
            error: match self.error.lock() {
                Ok(error) => error.clone(),
                Err(_) => Some("桌面状态锁异常，请保留未保存内容并重启。".into()),
            },
        }
    }
}
fn failure(message: &str) -> CaptureError {
    CaptureError::new(ErrorCode::WindowState, message)
}
pub fn notice(app: &AppHandle, message: &str) {
    if let Ok(mut error) = app.state::<DesktopService>().error.lock() {
        *error = Some(message.into());
    }
    let _ = app.emit_to("main", "desktop-notice", message);
}
pub fn show_main(app: &AppHandle) -> Result<(), CaptureError> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| failure("主窗口不可用，请重启应用。"))?;
    window
        .show()
        .and_then(|_| window.unminimize())
        .and_then(|_| window.set_focus())
        .map_err(|_| failure("无法恢复主窗口，请通过系统任务栏重试。"))
}
pub fn install_tray(app: &AppHandle) -> Result<(), String> {
    if app.tray_by_id(TRAY_ID).is_some() {
        return Ok(());
    }
    let item = |id, text| MenuItem::with_id(app, id, text, true, None::<&str>);
    let open = item("desktop-open", "打开 ResearchClip(&O)").map_err(|_| "托盘菜单创建失败。")?;
    let region = item("desktop-region", "开始区域截图(&R)").map_err(|_| "托盘菜单创建失败。")?;
    let full = item("desktop-fullscreen", "全屏截图(&F)").map_err(|_| "托盘菜单创建失败。")?;
    let settings = item("desktop-settings", "打开设置(&S)").map_err(|_| "托盘菜单创建失败。")?;
    let exit = item("desktop-exit", "退出 ResearchClip(&Q)").map_err(|_| "托盘菜单创建失败。")?;
    let menu = Menu::with_items(app, &[&open, &region, &full, &settings, &exit])
        .map_err(|_| "托盘菜单创建失败。")?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("应用图标不可用，无法创建托盘。")?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("ResearchClip")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let action = match event.id.as_ref() {
                "desktop-open" => DesktopAction::Open,
                "desktop-settings" => DesktopAction::Settings,
                "desktop-region" => DesktopAction::Region,
                "desktop-fullscreen" => DesktopAction::Fullscreen,
                "desktop-exit" => DesktopAction::Exit,
                _ => return,
            };
            if let Err(error) = dispatch(app, action) {
                notice(app, &error.message);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) && let Err(error) = show_main(tray.app_handle())
            {
                notice(tray.app_handle(), &error.message);
            }
        })
        .build(app)
        .map_err(|_| "系统托盘创建失败；主窗口仍可使用，关闭窗口将退出应用。")?;
    Ok(())
}
pub fn request_exit(app: &AppHandle) -> Result<(), CaptureError> {
    let service = app.state::<DesktopService>();
    if !service.request() {
        return Ok(());
    }
    app.state::<Arc<super::ocr_index::OcrIndexService>>()
        .cancel();
    app.state::<Arc<super::scrolling::ScrollService>>().cancel();
    app.state::<Arc<RegionCaptureService>>().cancel_active();
    if let Err(error) = show_main(app).and_then(|_| {
        app.emit_to("main", "desktop-action", "exit")
            .map_err(|_| failure("无法通知窗口保存资料；退出已取消，请重试。"))
    }) {
        service.cancel();
        return Err(error);
    }
    Ok(())
}
pub fn dispatch(app: &AppHandle, action: DesktopAction) -> Result<(), CaptureError> {
    if matches!(action, DesktopAction::Exit) {
        return request_exit(app);
    }
    if !app.state::<DesktopService>().running() {
        return Err(failure("正在准备退出，请先完成或取消退出。"));
    }
    match action {
        DesktopAction::Open => show_main(app),
        DesktopAction::Settings => {
            show_main(app)?;
            app.emit_to("main", "desktop-action", "settings")
                .map_err(|_| failure("无法打开设置页面，请在主窗口选择设置。"))
        }
        DesktopAction::Hide => {
            if app.tray_by_id(TRAY_ID).is_none() {
                return Err(failure("托盘不可用，已保留主窗口。"));
            }
            app.state::<Arc<super::scrolling::ScrollService>>().cancel();
            app.state::<Arc<RegionCaptureService>>().cancel_active();
            app.get_webview_window("main")
                .ok_or_else(|| failure("主窗口不可用。"))?
                .hide()
                .map_err(|_| failure("无法隐藏主窗口。"))
        }
        DesktopAction::Region | DesktopAction::Fullscreen => {
            crate::commands::workflow::shortcut_capture(
                app.clone(),
                if matches!(action, DesktopAction::Region) {
                    ShortcutAction::Region
                } else {
                    ShortcutAction::Fullscreen
                },
            );
            Ok(())
        }
        DesktopAction::Exit => unreachable!(),
    }
}
pub fn finish_exit(app: &AppHandle, approved: bool) -> Result<(), CaptureError> {
    let service = app.state::<DesktopService>();
    if !approved {
        service.cancel();
        return Ok(());
    }
    if service.running() || service.closing() {
        return Err(failure("没有等待确认的退出请求。"));
    }
    let captures = app.state::<Arc<ScreenCaptureService>>();
    let start = std::time::Instant::now();
    let permit = loop {
        match captures.try_begin() {
            Ok(permit) => break permit,
            Err(_) if start.elapsed() < std::time::Duration::from_secs(3) => {
                std::thread::sleep(std::time::Duration::from_millis(20))
            }
            Err(_) => {
                service.cancel();
                return Err(failure(
                    "仍有截图、保存或文件对话框未完成，退出已取消，请完成操作后重试。",
                ));
            }
        }
    };
    if !service.commit() {
        return Err(failure("退出请求已改变，请重试。"));
    }
    let result = app
        .state::<Arc<SettingsService>>()
        .prepare_shutdown()
        .and_then(|_| {
            app.global_shortcut().unregister_all().map_err(|_| {
                failure("快捷键注销失败，应用暂未退出。请重试退出；快捷键触发已暂停。")
            })
        });
    if let Err(error) = result {
        service.phase.store(0, Ordering::Release);
        notice(app, &error.message);
        return Err(error);
    }
    app.state::<Arc<super::scrolling::ScrollService>>().cancel();
    app.state::<Arc<RegionCaptureService>>().cancel_active();
    if let Err(error) = app.state::<Arc<super::pins::PinManager>>().close_all(app) {
        service.phase.store(0, Ordering::Release);
        return Err(error);
    }
    // Destroy the main WebView before the last-window Exit event. Exiting the
    // event loop directly can leave Chromium child windows alive during cleanup.
    if let Some(window) = app.get_webview_window("main") {
        if window.close().is_err() {
            service.phase.store(0, Ordering::Release);
            return Err(failure("窗口关闭失败，快捷键触发已暂停，请重试退出。"));
        }
    } else {
        app.exit(0);
    }
    drop(permit);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exit_is_requested_once_and_cancel_allows_retry() {
        let s = DesktopService::default();
        assert!(s.running());
        assert!(s.request());
        assert!(!s.request());
        s.cancel();
        assert!(s.running());
        assert!(s.request());
        assert!(s.commit());
        assert!(s.closing());
        assert!(!s.request());
        s.cancel();
        assert!(s.closing());
    }
    #[test]
    fn stale_completion_cannot_exit_without_a_request() {
        let s = DesktopService::default();
        assert!(!s.commit());
        assert!(s.running());
        assert!(s.request());
        s.cancel();
        assert!(!s.commit());
    }
}
