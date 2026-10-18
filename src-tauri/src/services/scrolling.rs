use super::{screen_capture::CapturePermit, stitching::failure, workflow::CaptureInfo};
use crate::capture::{CaptureError, DisplayInfo, ErrorCode, geometry::RectPx};
use serde::Serialize;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
};

#[cfg(target_os = "windows")]
pub use super::scrolling_windows as native;
#[cfg(not(target_os = "windows"))]
pub mod native {
    use super::*;
    fn unsupported() -> CaptureError {
        CaptureError::new(ErrorCode::Unsupported, "滚动长截图目前仅支持Windows。")
    }
    pub fn list() -> Result<Vec<Target>, CaptureError> {
        Err(unsupported())
    }
    pub fn activate(_: &Target) -> Result<(), CaptureError> {
        Err(unsupported())
    }
    pub fn validate(_: &Target) -> Result<(), CaptureError> {
        Err(unsupported())
    }
    pub fn visible(_: &Target, _: &DisplayInfo, _: RectPx) -> Result<(), CaptureError> {
        Err(unsupported())
    }
    pub fn cursor(_: &Target, _: &DisplayInfo, _: RectPx) -> Result<(), CaptureError> {
        Err(unsupported())
    }
    pub fn wheel(_: &Target, _: &DisplayInfo, _: RectPx) -> Result<(), CaptureError> {
        Err(unsupported())
    }
}
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, Serialize)]
pub struct Target {
    pub id: String,
    pub title: String,
    pub automatic: bool,
    #[serde(skip)]
    pub handle: usize,
    #[serde(skip)]
    pub pid: u32,
    pub bounds: Bounds,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub session_id: String,
    pub state: String,
    pub frames: u32,
    pub width: u32,
    pub height: u32,
    pub automatic: bool,
    pub message: String,
    pub result: Option<CaptureInfo>,
}
pub struct Session {
    pub id: String,
    pub target: Target,
    pub display: DisplayInfo,
    pub rect: RectPx,
    pub action: AtomicU8,
    pub next: AtomicBool,
    pub started: AtomicBool,
    pub progress: Mutex<Progress>,
    pub permit: Mutex<Option<CapturePermit>>,
}
impl Session {
    pub fn status(&self) -> Result<Progress, CaptureError> {
        Ok(self
            .progress
            .lock()
            .map_err(|_| failure("长截图状态不可用。"))?
            .clone())
    }
    pub fn update(&self, state: &str, message: &str) {
        if let Ok(mut p) = self.progress.lock() {
            p.state = state.into();
            p.message = message.into();
        }
    }
    pub fn release(&self) {
        if let Ok(mut permit) = self.permit.lock() {
            permit.take();
        }
    }
}
#[derive(Default)]
pub struct ScrollService {
    pub targets: Mutex<Vec<Target>>,
    pub active: Mutex<Option<Arc<Session>>>,
    sequence: AtomicU64,
}
impl ScrollService {
    pub fn list(&self) -> Result<Vec<Target>, CaptureError> {
        if self
            .active
            .lock()
            .map_err(|_| failure("长截图状态不可用。"))?
            .as_ref()
            .is_some_and(|s| s.permit.lock().map(|p| p.is_some()).unwrap_or(true))
        {
            return Err(CaptureError::new(
                ErrorCode::Busy,
                "请先完成或取消当前长截图。",
            ));
        }
        let mut targets = native::list()?;
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        for (i, t) in targets.iter_mut().enumerate() {
            t.id = format!("{seq}-{i}");
        }
        *self
            .targets
            .lock()
            .map_err(|_| failure("无法保存目标窗口列表。"))? = targets.clone();
        Ok(targets)
    }
    pub fn target(&self, id: &str) -> Result<Target, CaptureError> {
        self.targets
            .lock()
            .map_err(|_| failure("目标列表不可用。"))?
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| failure("目标窗口授权已失效，请刷新并明确选择窗口。"))
    }
    pub fn prepare(
        &self,
        target: Target,
        display: DisplayInfo,
        rect: RectPx,
        automatic: bool,
        permit: CapturePermit,
    ) -> Result<Progress, CaptureError> {
        validate_region(target.bounds, &display, rect)?;
        if automatic && !target.automatic {
            return Err(failure(
                "自动模式只开放给Chrome/Edge普通网页；其他窗口请用手动滚动模式。",
            ));
        }
        let id = format!(
            "{}-{}",
            std::process::id(),
            self.sequence.fetch_add(1, Ordering::Relaxed)
        );
        let progress = Progress {
            session_id: id.clone(),
            state: "ready".into(),
            frames: 0,
            width: rect.width,
            height: rect.height,
            automatic,
            message: "选区已准备。点击开始，采集期间请保持目标窗口位置及焦点。".into(),
            result: None,
        };
        *self
            .active
            .lock()
            .map_err(|_| failure("长截图状态不可用。"))? = Some(Arc::new(Session {
            id,
            target,
            display,
            rect,
            action: AtomicU8::new(0),
            next: AtomicBool::new(false),
            started: AtomicBool::new(false),
            progress: Mutex::new(progress.clone()),
            permit: Mutex::new(Some(permit)),
        }));
        Ok(progress)
    }
    pub fn session(&self, id: &str, label: &str) -> Result<Arc<Session>, CaptureError> {
        if label != "main" && label != format!("scroll-control-{id}") {
            return Err(failure("该窗口无权访问长截图会话。"));
        }
        self.active
            .lock()
            .map_err(|_| failure("长截图状态不可用。"))?
            .as_ref()
            .filter(|s| s.id == id)
            .cloned()
            .ok_or_else(|| failure("长截图会话已结束。"))
    }
    pub fn cancel(&self) {
        if let Ok(active) = self.active.lock()
            && let Some(s) = active.as_ref()
        {
            s.action.store(2, Ordering::Release);
            if !s.started.load(Ordering::Acquire) {
                s.update("cancelled", "已取消，未保存任何图片。");
                s.release();
            }
        }
    }
}
pub fn validate_region(b: Bounds, d: &DisplayInfo, r: RectPx) -> Result<(), CaptureError> {
    let x = i64::from(d.x_px) + i64::from(r.x);
    let y = i64::from(d.y_px) + i64::from(r.y);
    if r.width < 128
        || r.height < 160
        || r.height > super::stitching::MAX_HEIGHT
        || r.width > 4096
        || u64::from(r.width) * u64::from(r.height) > 4_194_304
        || u64::from(r.x) + u64::from(r.width) > u64::from(d.width_px)
        || u64::from(r.y) + u64::from(r.height) > u64::from(d.height_px)
        || x < i64::from(b.x)
        || y < i64::from(b.y)
        || x + i64::from(r.width) > i64::from(b.x) + i64::from(b.width)
        || y + i64::from(r.height) > i64::from(b.y) + i64::from(b.height)
    {
        return Err(failure(
            "选区须位于所选窗口客户区和同一显示器内，至少128×160且不超过4,194,304像素；请排除地址栏、滚动条及底部工具栏。",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_coordinates_and_physical_pixel_bounds() {
        let d = DisplayInfo {
            id: "test".into(),
            name: "test".into(),
            x_px: -1920,
            y_px: 0,
            width_px: 1920,
            height_px: 1080,
            is_primary: false,
        };
        let b = Bounds {
            x: -1800,
            y: 100,
            width: 1200,
            height: 800,
        };
        for scale in [1.25, 1.5] {
            let r = crate::capture::geometry::selection_to_pixels(
                crate::capture::geometry::Selection {
                    start: crate::capture::geometry::Point {
                        x: 160.0 / scale,
                        y: 160.0 / scale,
                    },
                    end: crate::capture::geometry::Point {
                        x: 1000.0 / scale,
                        y: 800.0 / scale,
                    },
                    viewport_width: 1920.0 / scale,
                    viewport_height: 1080.0 / scale,
                },
                1920,
                1080,
            )
            .unwrap();
            assert_eq!(
                r,
                RectPx {
                    x: 160,
                    y: 160,
                    width: 840,
                    height: 640
                }
            );
            assert!(validate_region(b, &d, r).is_ok());
        }
        assert!(
            validate_region(
                b,
                &d,
                RectPx {
                    x: 0,
                    y: 0,
                    width: 300,
                    height: 300
                }
            )
            .is_err()
        );
    }
    #[test]
    fn session_authorization_cancel_and_capture_gate_release() {
        let service = ScrollService::default();
        let captures = super::super::screen_capture::ScreenCaptureService::default();
        let d = DisplayInfo {
            id: "test".into(),
            name: "test".into(),
            x_px: 0,
            y_px: 0,
            width_px: 400,
            height_px: 400,
            is_primary: true,
        };
        let t = Target {
            id: "chosen".into(),
            title: "test".into(),
            automatic: false,
            handle: 0,
            pid: 0,
            bounds: Bounds {
                x: 0,
                y: 0,
                width: 400,
                height: 400,
            },
        };
        let p = service
            .prepare(
                t,
                d,
                RectPx {
                    x: 0,
                    y: 0,
                    width: 300,
                    height: 300,
                },
                false,
                captures.try_begin().unwrap(),
            )
            .unwrap();
        assert!(captures.try_begin().is_err());
        assert!(service.session(&p.session_id, "pin-1").is_err());
        assert!(service.session("stale", "main").is_err());
        service.cancel();
        assert!(captures.try_begin().is_ok());
        assert_eq!(
            service
                .session(&p.session_id, "main")
                .unwrap()
                .status()
                .unwrap()
                .state,
            "cancelled"
        );
    }
}
