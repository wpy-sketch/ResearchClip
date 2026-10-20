//! A bounded, user-initiated window selection session. Native handles never cross IPC.
use crate::capture::{CaptureError, DisplayInfo, ErrorCode, RawFrame, rgba_len};
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Receiver, SyncSender},
};
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
mod worker;

pub(super) fn failure(message: &str) -> CaptureError {
    CaptureError::new(ErrorCode::CaptureFailed, message)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
impl Bounds {
    fn contains(self, p: (i32, i32)) -> bool {
        i64::from(p.0) >= i64::from(self.x)
            && i64::from(p.1) >= i64::from(self.y)
            && i64::from(p.0) < i64::from(self.x) + i64::from(self.width)
            && i64::from(p.1) < i64::from(self.y) + i64::from(self.height)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub(super) handle: usize,
    pub(super) pid: u32,
    pub(super) thread: u32,
    pub(super) class: String,
    pub title: String,
    pub bounds: Bounds,
    pub dpi: u32,
    pub protected: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickerInfo {
    pub session_id: String,
    pub desktop: Bounds,
    pub selected: Option<WindowInfo>,
    pub candidate_count: usize,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub title: String,
    pub bounds: Bounds,
    pub dpi: u32,
    pub blocked: bool,
}
impl From<&Target> for WindowInfo {
    fn from(t: &Target) -> Self {
        Self {
            title: t.title.clone(),
            bounds: t.bounds,
            dpi: t.dpi,
            blocked: t.protected,
        }
    }
}
pub enum Decision {
    Selected(Target),
    Cancelled,
}
struct Session {
    ready: bool,
    id: String,
    desktop: Bounds,
    targets: Vec<Target>,
    selected: Option<usize>,
    cursor: Option<(i32, i32)>,
    sender: Option<SyncSender<Decision>>,
    cancel: Arc<AtomicBool>,
}
#[derive(Default)]
pub struct WindowCaptureService {
    active: Mutex<Option<Session>>,
    sequence: AtomicU64,
}
pub fn picker_label(id: &str) -> String {
    format!("window-picker-{id}")
}
impl WindowCaptureService {
    pub fn begin(
        &self,
        displays: &[DisplayInfo],
    ) -> Result<(PickerInfo, Receiver<Decision>, Arc<AtomicBool>), CaptureError> {
        let desktop = desktop_bounds(displays)?;
        let targets = list_targets()?;
        if targets.is_empty() {
            return Err(failure(
                "没有可选择的普通桌面窗口。请先打开并还原目标窗口。",
            ));
        }
        let mut active = self
            .active
            .lock()
            .map_err(|_| failure("窗口截图状态不可用。"))?;
        if active.is_some() {
            return Err(CaptureError::new(ErrorCode::Busy, "已有窗口选择会话。"));
        }
        let id = format!(
            "{}-{}",
            std::process::id(),
            self.sequence.fetch_add(1, Ordering::Relaxed)
        );
        let (sender, receiver) = mpsc::sync_channel(1);
        let cancel = Arc::new(AtomicBool::new(false));
        let info = PickerInfo {
            session_id: id.clone(),
            desktop,
            selected: None,
            candidate_count: targets.len(),
        };
        *active = Some(Session {
            ready: false,
            id,
            desktop,
            targets,
            selected: None,
            cursor: None,
            sender: Some(sender),
            cancel: Arc::clone(&cancel),
        });
        Ok((info, receiver, cancel))
    }
    fn with<T>(
        &self,
        id: &str,
        label: &str,
        operation: impl FnOnce(&mut Session) -> Result<T, CaptureError>,
    ) -> Result<T, CaptureError> {
        if picker_label(id) != label {
            return Err(CaptureError::new(
                ErrorCode::InvalidSession,
                "无权访问此窗口选择会话。",
            ));
        }
        let mut active = self
            .active
            .lock()
            .map_err(|_| failure("窗口截图状态不可用。"))?;
        let session = active
            .as_mut()
            .filter(|s| s.id == id)
            .ok_or_else(|| CaptureError::new(ErrorCode::InvalidSession, "窗口选择会话已结束。"))?;
        operation(session)
    }
    pub fn status(&self, id: &str, label: &str) -> Result<PickerInfo, CaptureError> {
        let cursor = cursor_position()?;
        let targets = list_targets()?;
        self.with(id, label, |s| {
            if s.sender.is_some() {
                let selected_handle = s.selected.and_then(|i| s.targets.get(i)).map(|t| t.handle);
                s.targets = targets;
                s.selected = if s.cursor != Some(cursor) {
                    s.targets.iter().position(|t| t.bounds.contains(cursor))
                } else {
                    selected_handle.and_then(|h| s.targets.iter().position(|t| t.handle == h))
                };
                s.cursor = Some(cursor);
            }
            Ok(PickerInfo {
                session_id: s.id.clone(),
                desktop: s.desktop,
                selected: s
                    .selected
                    .and_then(|i| s.targets.get(i))
                    .map(WindowInfo::from),
                candidate_count: s.targets.iter().filter(|t| !t.protected).count(),
            })
        })
    }
    pub fn action(&self, id: &str, label: &str, action: &str) -> Result<(), CaptureError> {
        self.with(id, label, |s| {
            if s.sender.is_none() {
                return Ok(());
            }
            match action {
                "ready" => s.ready = true,
                "next" | "previous" => {
                    s.selected = cycle(&s.targets, s.selected, action == "previous")
                }
                "capture" => {
                    let target = s
                        .selected
                        .and_then(|i| s.targets.get(i))
                        .ok_or_else(|| failure("鼠标下没有可捕获窗口，请移动鼠标或按 Tab。"))?
                        .clone();
                    if target.protected {
                        return Err(CaptureError::new(
                            ErrorCode::PermissionDenied,
                            "目标窗口禁止捕获，请选择其他窗口。",
                        ));
                    }
                    s.sender
                        .take()
                        .ok_or_else(|| failure("选择会话已结束。"))?
                        .try_send(Decision::Selected(target))
                        .map_err(|_| failure("无法提交窗口选择。"))?;
                }
                "cancel" => {
                    s.cancel.store(true, Ordering::Release);
                    if let Some(sender) = s.sender.take() {
                        let _ = sender.try_send(Decision::Cancelled);
                    }
                }
                _ => return Err(failure("未知窗口选择操作。")),
            }
            Ok(())
        })
    }
    pub fn is_ready(&self, id: &str) -> bool {
        self.active
            .lock()
            .ok()
            .is_some_and(|a| a.as_ref().is_some_and(|s| s.id == id && s.ready))
    }
    pub fn cancel_pending(&self, id: &str) {
        if let Ok(mut active) = self.active.lock()
            && let Some(s) = active.as_mut().filter(|s| s.id == id && s.sender.is_some())
        {
            s.cancel.store(true, Ordering::Release);
            if let Some(sender) = s.sender.take() {
                let _ = sender.try_send(Decision::Cancelled);
            }
        }
    }
    pub fn cancel_active(&self) {
        if let Ok(mut active) = self.active.lock()
            && let Some(s) = active.as_mut()
        {
            s.cancel.store(true, Ordering::Release);
            if let Some(sender) = s.sender.take() {
                let _ = sender.try_send(Decision::Cancelled);
            }
        }
    }
    pub fn finish(&self, id: &str) {
        if let Ok(mut active) = self.active.lock()
            && active.as_ref().is_some_and(|s| s.id == id)
        {
            *active = None;
        }
    }
}
fn cycle(targets: &[Target], current: Option<usize>, reverse: bool) -> Option<usize> {
    let len = targets.len();
    if len == 0 {
        return None;
    }
    let start = current.unwrap_or(if reverse { 0 } else { len - 1 });
    (1..=len)
        .map(|offset| {
            if reverse {
                (start + len - offset) % len
            } else {
                (start + offset) % len
            }
        })
        .find(|i| !targets[*i].protected)
}
pub fn desktop_bounds(displays: &[DisplayInfo]) -> Result<Bounds, CaptureError> {
    let left = displays
        .iter()
        .map(|d| i64::from(d.x_px))
        .min()
        .ok_or_else(|| failure("没有可用显示器。"))?;
    let top = displays
        .iter()
        .map(|d| i64::from(d.y_px))
        .min()
        .unwrap_or(0);
    let right = displays
        .iter()
        .map(|d| i64::from(d.x_px) + i64::from(d.width_px))
        .max()
        .unwrap_or(left);
    let bottom = displays
        .iter()
        .map(|d| i64::from(d.y_px) + i64::from(d.height_px))
        .max()
        .unwrap_or(top);
    let width = u32::try_from(right - left).map_err(|_| failure("显示器布局超出支持范围。"))?;
    let height = u32::try_from(bottom - top).map_err(|_| failure("显示器布局超出支持范围。"))?;
    // Bound the transparent selection surface, independently of the captured window texture.
    if width == 0
        || height == 0
        || width > 16384
        || height > 16384
        || u64::from(width) * u64::from(height) > 67_108_864
    {
        return Err(failure(
            "当前多显示器布局超出选择层安全上限，请调整布局后重试。",
        ));
    }
    Ok(Bounds {
        x: left as i32,
        y: top as i32,
        width,
        height,
    })
}
pub fn convert_bgra(
    bytes: &[u8],
    pitch: usize,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, CaptureError> {
    let len = rgba_len(width, height)?;
    let row = width as usize * 4;
    if pitch < row
        || pitch
            .checked_mul(height as usize)
            .is_none_or(|n| n > bytes.len())
    {
        return Err(failure("窗口图像缓冲区不完整。"));
    }
    let mut rgba = vec![0; len];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let s = y * pitch + x * 4;
            let d = (y * width as usize + x) * 4;
            rgba[d..d + 4].copy_from_slice(&[bytes[s + 2], bytes[s + 1], bytes[s], bytes[s + 3]]);
        }
    }
    Ok(rgba)
}
pub fn list_targets() -> Result<Vec<Target>, CaptureError> {
    #[cfg(target_os = "windows")]
    {
        windows::list()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(CaptureError::new(
            ErrorCode::Unsupported,
            "窗口截图目前仅支持 Windows。",
        ))
    }
}
fn cursor_position() -> Result<(i32, i32), CaptureError> {
    #[cfg(target_os = "windows")]
    {
        windows::cursor()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(CaptureError::new(
            ErrorCode::Unsupported,
            "窗口截图目前仅支持 Windows。",
        ))
    }
}
impl WindowCaptureService {
    pub fn capture(&self, target: &Target, cancel: &AtomicBool) -> Result<RawFrame, CaptureError> {
        #[cfg(target_os = "windows")]
        {
            worker::capture(target, cancel)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (target, cancel);
            Err(CaptureError::new(
                ErrorCode::Unsupported,
                "窗口截图目前仅支持 Windows。",
            ))
        }
    }
}
/// Entry point is checked before Tauri initialization. No settings or SQLite are opened.
pub fn run_capture_worker() -> bool {
    #[cfg(target_os = "windows")]
    {
        worker::entry()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target(n: usize, protected: bool) -> Target {
        Target {
            handle: n,
            pid: 1,
            thread: 1,
            class: "test".into(),
            title: "test".into(),
            bounds: Bounds {
                x: -100,
                y: -20,
                width: 100,
                height: 50,
            },
            dpi: 144,
            protected,
        }
    }
    #[test]
    fn physical_bounds_are_exclusive_and_support_negative_origins() {
        let b = target(1, false).bounds;
        assert!(b.contains((-100, -20)));
        assert!(b.contains((-1, 29)));
        assert!(!b.contains((0, 0)));
        assert!(!b.contains((-101, 0)));
    }
    #[test]
    fn tab_wraps_and_skips_protected_windows() {
        let targets = vec![target(1, false), target(2, true), target(3, false)];
        assert_eq!(cycle(&targets, None, false), Some(0));
        assert_eq!(cycle(&targets, Some(0), false), Some(2));
        assert_eq!(cycle(&targets, Some(0), true), Some(2));
        assert_eq!(cycle(&[target(1, true)], None, false), None);
    }
    #[test]
    fn pitched_bgra_preserves_pixels_and_alpha() {
        assert_eq!(
            convert_bgra(&[1, 2, 3, 4, 9, 9, 9, 9, 5, 6, 7, 8, 0, 0, 0, 0], 8, 1, 2).unwrap(),
            [3, 2, 1, 4, 7, 6, 5, 8]
        );
        assert!(convert_bgra(&[0; 4], 3, 1, 1).is_err());
        assert!(convert_bgra(&[0; 4], 8, 1, 2).is_err());
    }
    #[test]
    fn virtual_desktop_is_bounded() {
        let d = |x, width| DisplayInfo {
            id: "test".into(),
            name: "test".into(),
            x_px: x,
            y_px: 0,
            width_px: width,
            height_px: 1080,
            is_primary: false,
        };
        assert_eq!(
            desktop_bounds(&[d(-1920, 1920), d(0, 1920)]).unwrap(),
            Bounds {
                x: -1920,
                y: 0,
                width: 3840,
                height: 1080
            }
        );
        assert!(desktop_bounds(&[]).is_err());
        assert!(desktop_bounds(&[d(0, 20000)]).is_err());
    }
    #[test]
    fn session_identity_and_cancellation_do_not_leak() {
        let service = WindowCaptureService::default();
        let (sender, receiver) = mpsc::sync_channel(1);
        let cancel = Arc::new(AtomicBool::new(false));
        *service.active.lock().unwrap() = Some(Session {
            ready: false,
            id: "test".into(),
            desktop: target(1, false).bounds,
            targets: vec![target(1, false)],
            selected: Some(0),
            cursor: None,
            sender: Some(sender),
            cancel: Arc::clone(&cancel),
        });
        assert!(service.action("test", "main", "capture").is_err());
        service
            .action("test", "window-picker-test", "capture")
            .unwrap();
        assert!(matches!(receiver.try_recv(), Ok(Decision::Selected(_))));
        service.cancel_pending("test");
        assert!(!cancel.load(Ordering::Acquire));
        service.cancel_active();
        assert!(cancel.load(Ordering::Acquire));
        service.finish("wrong");
        assert!(service.active.lock().unwrap().is_some());
        service.finish("test");
        assert!(service.active.lock().unwrap().is_none());
    }
}
