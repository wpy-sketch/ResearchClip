use crate::capture::{CaptureError, DisplayInfo, ErrorCode, MAX_PNG_BYTES, geometry::RectPx};
use serde::Serialize;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc::{Receiver, SyncSender, sync_channel},
};

pub enum Decision {
    Selected(RectPx),
    Failed(CaptureError),
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayInfo {
    pub session_id: String,
    pub display: DisplayInfo,
}

struct ActiveRegion {
    info: OverlayInfo,
    png: Option<Vec<u8>>,
    sender: SyncSender<Decision>,
    ready: bool,
    completed: bool,
    viewport: Option<(f64, f64)>,
}

#[derive(Default)]
pub struct RegionCaptureService {
    sequence: AtomicU64,
    active: Mutex<Option<ActiveRegion>>,
}

pub fn overlay_label(id: &str) -> String {
    format!("capture-overlay-{id}")
}
fn session_error() -> CaptureError {
    CaptureError::new(
        ErrorCode::InvalidSession,
        "截图会话已结束或该窗口无权访问。",
    )
}
fn lock_error() -> CaptureError {
    CaptureError::new(ErrorCode::Internal, "区域截图状态不可用，请重新启动应用。")
}

impl RegionCaptureService {
    pub fn begin(
        &self,
        display: DisplayInfo,
        png: Vec<u8>,
    ) -> Result<(OverlayInfo, Receiver<Decision>), CaptureError> {
        if png.len() > MAX_PNG_BYTES {
            return Err(CaptureError::new(
                ErrorCode::ResourceLimit,
                "冻结图像超出大小上限。",
            ));
        }
        let mut active = self.active.lock().map_err(|_| lock_error())?;
        if active.is_some() {
            return Err(CaptureError::new(
                ErrorCode::Busy,
                "已有区域截图会话，请先完成或取消。",
            ));
        }
        let value = self
            .sequence
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| lock_error())?
            + 1;
        let info = OverlayInfo {
            session_id: value.to_string(),
            display,
        };
        let (sender, receiver) = sync_channel(1);
        *active = Some(ActiveRegion {
            info: info.clone(),
            png: Some(png),
            sender,
            ready: false,
            completed: false,
            viewport: None,
        });
        Ok((info, receiver))
    }
    fn authorized<'a>(
        active: &'a mut Option<ActiveRegion>,
        id: &str,
        label: &str,
    ) -> Result<&'a mut ActiveRegion, CaptureError> {
        active
            .as_mut()
            .filter(|s| s.info.session_id == id && overlay_label(id) == label && !s.completed)
            .ok_or_else(session_error)
    }
    pub fn info(&self, id: &str, label: &str) -> Result<OverlayInfo, CaptureError> {
        let mut active = self.active.lock().map_err(|_| lock_error())?;
        Ok(Self::authorized(&mut active, id, label)?.info.clone())
    }
    pub fn take_preview(&self, id: &str, label: &str) -> Result<Vec<u8>, CaptureError> {
        let mut active = self.active.lock().map_err(|_| lock_error())?;
        Self::authorized(&mut active, id, label)?
            .png
            .take()
            .ok_or_else(session_error)
    }
    pub fn mark_ready(
        &self,
        id: &str,
        label: &str,
        width: f64,
        height: f64,
    ) -> Result<(), CaptureError> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(CaptureError::new(
                ErrorCode::InvalidSelection,
                "选择窗口尺寸无效。",
            ));
        }
        let mut active = self.active.lock().map_err(|_| lock_error())?;
        let session = Self::authorized(&mut active, id, label)?;
        session.ready = true;
        session.viewport = Some((width, height));
        Ok(())
    }
    pub fn is_ready(&self, id: &str) -> bool {
        self.active
            .lock()
            .ok()
            .and_then(|active| {
                active
                    .as_ref()
                    .map(|s| s.info.session_id == id && s.ready && !s.completed)
            })
            .unwrap_or(false)
    }
    pub fn viewport(&self, id: &str, label: &str) -> Result<(f64, f64), CaptureError> {
        let mut active = self.active.lock().map_err(|_| lock_error())?;
        Self::authorized(&mut active, id, label)?
            .viewport
            .ok_or_else(session_error)
    }
    pub fn complete(&self, id: &str, label: &str, rect: RectPx) -> Result<(), CaptureError> {
        let mut active = self.active.lock().map_err(|_| lock_error())?;
        let session = Self::authorized(&mut active, id, label)?;
        if !session.ready {
            return Err(session_error());
        }
        session
            .sender
            .try_send(Decision::Selected(rect))
            .map_err(|_| session_error())?;
        session.completed = true;
        Ok(())
    }
    // Window callbacks can only end the session matching their own ID. Stale callbacks are harmless.
    pub fn fail(&self, id: &str, error: CaptureError) {
        if let Ok(mut active) = self.active.lock()
            && let Some(session) = active
                .as_mut()
                .filter(|s| s.info.session_id == id && !s.completed)
            && session.sender.try_send(Decision::Failed(error)).is_ok()
        {
            session.completed = true;
        }
    }
    pub fn finish(&self, id: &str) {
        if let Ok(mut active) = self.active.lock()
            && active.as_ref().is_some_and(|s| s.info.session_id == id)
        {
            *active = None;
        }
    }
    pub fn cancel_active(&self) {
        if let Ok(mut active) = self.active.lock()
            && let Some(session) = active.as_mut().filter(|s| !s.completed)
            && session
                .sender
                .try_send(Decision::Failed(CaptureError::new(
                    ErrorCode::Cancelled,
                    "主窗口已关闭，区域截图已取消。",
                )))
                .is_ok()
        {
            session.completed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn display() -> DisplayInfo {
        DisplayInfo {
            id: "display".into(),
            name: "display".into(),
            width_px: 800,
            height_px: 600,
            x_px: -800,
            y_px: -200,
            is_primary: true,
        }
    }
    #[test]
    fn one_session_restricts_window_and_preview_reads() {
        let service = RegionCaptureService::default();
        let (info, _) = service.begin(display(), vec![1, 2, 3]).unwrap();
        let label = overlay_label(&info.session_id);
        assert!(service.begin(display(), vec![]).is_err());
        assert!(service.info(&info.session_id, "main").is_err());
        assert_eq!(
            service.info(&info.session_id, &label).unwrap().display.x_px,
            -800
        );
        assert_eq!(
            service.take_preview(&info.session_id, &label).unwrap(),
            [1, 2, 3]
        );
        assert!(service.take_preview(&info.session_id, &label).is_err());
        service.finish(&info.session_id);
        assert!(service.info(&info.session_id, &label).is_err());
    }
    #[test]
    fn rejects_repeat_completion_and_stale_cancellation() {
        let service = RegionCaptureService::default();
        let (first, receiver) = service.begin(display(), vec![]).unwrap();
        let label = overlay_label(&first.session_id);
        let rect = RectPx {
            x: 1,
            y: 2,
            width: 3,
            height: 4,
        };
        assert!(service.complete(&first.session_id, &label, rect).is_err());
        service
            .mark_ready(&first.session_id, &label, 400.0, 300.0)
            .unwrap();
        service.complete(&first.session_id, &label, rect).unwrap();
        assert!(service.complete(&first.session_id, &label, rect).is_err());
        assert!(matches!(receiver.recv().unwrap(), Decision::Selected(value) if value == rect));
        service.finish(&first.session_id);
        let (second, _) = service.begin(display(), vec![]).unwrap();
        service.fail(
            &first.session_id,
            CaptureError::new(ErrorCode::Cancelled, "old"),
        );
        service.finish(&first.session_id);
        assert!(
            service
                .info(&second.session_id, &overlay_label(&second.session_id))
                .is_ok()
        );
    }
    #[test]
    fn main_window_close_cancels_once_and_next_session_can_start() {
        let service = RegionCaptureService::default();
        let (info, receiver) = service.begin(display(), vec![]).unwrap();
        service.cancel_active();
        service.cancel_active();
        assert!(matches!(
            receiver.recv().unwrap(),
            Decision::Failed(CaptureError {
                code: ErrorCode::Cancelled,
                ..
            })
        ));
        assert!(receiver.try_recv().is_err());
        assert!(
            service
                .info(&info.session_id, &overlay_label(&info.session_id))
                .is_err()
        );
        service.finish(&info.session_id);
        assert!(service.begin(display(), vec![]).is_ok());
    }
}
