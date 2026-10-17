//! Session-only desktop pins. No disk paths, persistence or mutable global state.
use crate::capture::{CaptureError, ErrorCode, rgba_len};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

const MAX_PINS: usize = 8;
const MAX_MEMORY: usize = 256 * 1024 * 1024;
pub fn failure(message: &str) -> CaptureError {
    CaptureError::new(ErrorCode::WindowState, message)
}

pub struct PinImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
struct Entry {
    image: Arc<PinImage>,
    opacity: u8,
    ready: bool,
    created: std::time::Instant,
}
#[derive(Default)]
pub struct PinManager {
    entries: Mutex<BTreeMap<String, Entry>>,
    sequence: AtomicU64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinInfo {
    pub width_px: u32,
    pub height_px: u32,
    pub opacity: u8,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinStatus {
    pub count: usize,
    pub retained_bytes: usize,
}
fn cost(image: &PinImage) -> usize {
    image.png.len() + image.width as usize * image.height as usize * 4
}
impl PinManager {
    fn reserve(&self, image: PinImage) -> Result<String, CaptureError> {
        rgba_len(image.width, image.height)?;
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| failure("贴图管理器不可用，请重启应用。"))?;
        if entries.len() >= MAX_PINS
            || entries.values().map(|e| cost(&e.image)).sum::<usize>() + cost(&image) > MAX_MEMORY
        {
            return Err(CaptureError::new(
                ErrorCode::ResourceLimit,
                "贴图数量或内存已达上限（最多 8 张、图像预算 256 MiB），请先关闭部分贴图。",
            ));
        }
        let id = self
            .sequence
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| failure("无法分配贴图标识。"))?;
        let label = format!("pin-{id}");
        entries.insert(
            label.clone(),
            Entry {
                image: Arc::new(image),
                opacity: 100,
                ready: false,
                created: std::time::Instant::now(),
            },
        );
        Ok(label)
    }
    pub fn image(&self, label: &str) -> Result<Arc<PinImage>, CaptureError> {
        self.entries
            .lock()
            .map_err(|_| failure("贴图资源不可用。"))?
            .get(label)
            .map(|e| Arc::clone(&e.image))
            .ok_or_else(|| failure("贴图已关闭或该窗口无权访问。"))
    }
    pub fn info(&self, label: &str) -> Result<PinInfo, CaptureError> {
        let entries = self
            .entries
            .lock()
            .map_err(|_| failure("贴图资源不可用。"))?;
        let e = entries.get(label).ok_or_else(|| failure("贴图已关闭。"))?;
        Ok(PinInfo {
            width_px: e.image.width,
            height_px: e.image.height,
            opacity: e.opacity,
        })
    }
    pub fn release(&self, label: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(label);
        }
    }
    pub fn status(&self) -> Result<PinStatus, CaptureError> {
        let entries = self
            .entries
            .lock()
            .map_err(|_| failure("贴图管理器不可用。"))?;
        Ok(PinStatus {
            count: entries.len(),
            retained_bytes: entries.values().map(|e| cost(&e.image)).sum(),
        })
    }
    pub fn create(
        self: &Arc<Self>,
        app: &AppHandle,
        image: PinImage,
    ) -> Result<String, CaptureError> {
        let label = self.reserve(image)?;
        // Never hold the resource mutex while invoking GUI operations / destruction callbacks.
        let result = (|| {
            let window = WebviewWindowBuilder::new(
                app,
                &label,
                WebviewUrl::App("index.html?desktopPin=1".into()),
            )
            .title(format!("ResearchClip 贴图 {label}"))
            .decorations(false)
            .shadow(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .focused(false)
            .inner_size(320.0, 200.0)
            .build()
            .map_err(|_| failure("无法创建贴图窗口。"))?;
            self.recover(&window)?;
            Ok(label.clone())
        })();
        if result.is_err() {
            if let Some(w) = app.get_webview_window(&label) {
                let _ = w.destroy();
            }
            self.release(&label);
        }
        result
    }
    // Exactly one watchdog for the application, rather than one timer thread per pin.
    pub fn start_watchdog(self: &Arc<Self>, app: &AppHandle) {
        let manager = Arc::clone(self);
        let app = app.clone();
        std::thread::spawn(move || {
            while app.get_webview_window("main").is_some() {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let pending = manager
                    .entries
                    .lock()
                    .map(|entries| {
                        entries
                            .iter()
                            .filter(|(_, e)| {
                                !e.ready && e.created.elapsed() > std::time::Duration::from_secs(15)
                            })
                            .map(|(label, _)| label.clone())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                for label in pending {
                    if let Some(window) = app.get_webview_window(&label)
                        && window.destroy().is_err()
                    {
                        super::desktop::notice(
                            &app,
                            "贴图加载超时且窗口无法关闭，请重试退出应用。",
                        );
                        continue;
                    }
                    manager.release(&label);
                    super::desktop::notice(&app, "贴图加载超时，已释放资源，请重试。");
                }
            }
        });
    }
    pub fn ready(&self, window: &WebviewWindow) -> Result<(), CaptureError> {
        self.image(window.label())?;
        window
            .show()
            .and_then(|_| window.set_focus())
            .map_err(|_| failure("贴图窗口无法显示。"))?;
        if let Some(e) = self
            .entries
            .lock()
            .map_err(|_| failure("贴图状态不可用。"))?
            .get_mut(window.label())
        {
            e.ready = true;
        }
        Ok(())
    }
    pub fn zoom(&self, window: &WebviewWindow, factor: f64) -> Result<(), CaptureError> {
        if !factor.is_finite() || !(0.5..=2.0).contains(&factor) {
            return Err(failure("贴图缩放参数无效。"));
        }
        let image = self.image(window.label())?;
        let size = window
            .inner_size()
            .map_err(|_| failure("无法获取贴图尺寸。"))?;
        let monitor = window
            .current_monitor()
            .ok()
            .flatten()
            .or_else(|| window.primary_monitor().ok().flatten())
            .ok_or_else(|| failure("没有可用显示器。"))?;
        let area = monitor.work_area();
        let (width, height) = scaled_size(
            image.width,
            image.height,
            size.width as f64 / image.width as f64 * factor,
            area.size.width,
            area.size.height,
        )?;
        window
            .set_size(PhysicalSize::new(width, height))
            .map_err(|_| failure("无法调整贴图大小。"))?;
        self.keep_visible(window)
    }
    fn keep_visible(&self, window: &WebviewWindow) -> Result<(), CaptureError> {
        let monitor = window
            .current_monitor()
            .ok()
            .flatten()
            .or_else(|| window.primary_monitor().ok().flatten())
            .ok_or_else(|| failure("没有可用显示器。"))?;
        let area = monitor.work_area();
        let pos = window
            .outer_position()
            .map_err(|_| failure("无法读取贴图位置。"))?;
        let size = window
            .outer_size()
            .map_err(|_| failure("无法读取贴图大小。"))?;
        let (x, y) = visible_position(
            pos.x,
            pos.y,
            size.width,
            size.height,
            area.position.x,
            area.position.y,
            area.size.width,
            area.size.height,
        );
        window
            .set_position(PhysicalPosition::new(x, y))
            .map_err(|_| failure("无法恢复贴图位置。"))
    }
    pub fn recover(&self, window: &WebviewWindow) -> Result<(), CaptureError> {
        let image = self.image(window.label())?;
        let monitor = window
            .primary_monitor()
            .map_err(|_| failure("无法读取主显示器。"))?
            .ok_or_else(|| failure("没有可用显示器。"))?;
        let area = monitor.work_area();
        let (w, h) = scaled_size(
            image.width,
            image.height,
            1.0,
            area.size.width,
            area.size.height,
        )?;
        window
            .set_size(PhysicalSize::new(w, h))
            .map_err(|_| failure("无法恢复贴图尺寸。"))?;
        let x = area.position.x + (area.size.width - w) as i32 / 2;
        let y = area.position.y + (area.size.height - h) as i32 / 2;
        window
            .set_position(PhysicalPosition::new(x, y))
            .map_err(|_| failure("无法恢复贴图位置。"))
    }
    pub fn opacity(&self, window: &WebviewWindow, value: f64) -> Result<(), CaptureError> {
        if !value.is_finite() || !(20.0..=100.0).contains(&value) {
            return Err(failure("透明度必须在 20% 至 100% 之间。"));
        }
        let value = value.round() as u8;
        self.image(window.label())?;
        set_native_opacity(window, value)?;
        if let Some(e) = self
            .entries
            .lock()
            .map_err(|_| failure("贴图状态不可用。"))?
            .get_mut(window.label())
        {
            e.opacity = value;
        }
        Ok(())
    }
    pub fn recover_all(&self, app: &AppHandle) -> Result<(), CaptureError> {
        for window in app
            .webview_windows()
            .values()
            .filter(|w| w.label().starts_with("pin-"))
        {
            self.recover(window)?;
            window
                .show()
                .and_then(|_| window.set_focus())
                .map_err(|_| failure("无法显示贴图。"))?;
        }
        Ok(())
    }
    pub fn close_all(&self, app: &AppHandle) -> Result<(), CaptureError> {
        for window in app
            .webview_windows()
            .values()
            .filter(|w| w.label().starts_with("pin-"))
        {
            window
                .destroy()
                .map_err(|_| failure("贴图关闭失败，请重试退出。"))?;
            self.release(window.label());
        }
        Ok(())
    }
}

// Scale is physical output pixels / original pixels. UI CSS pixels never enter crop/edit data.
pub fn scaled_size(
    w: u32,
    h: u32,
    scale: f64,
    available_w: u32,
    available_h: u32,
) -> Result<(u32, u32), CaptureError> {
    if w == 0 || h == 0 || available_w == 0 || available_h == 0 || !scale.is_finite() {
        return Err(failure("贴图或显示器尺寸无效。"));
    }
    let max = 4.0f64
        .min(available_w as f64 / w as f64)
        .min(available_h as f64 / h as f64);
    let min = 0.05f64.max(48.0 / w.max(h) as f64).min(max);
    let scale = scale.clamp(min, max);
    Ok((
        (w as f64 * scale).round().max(1.0) as u32,
        (h as f64 * scale).round().max(1.0) as u32,
    ))
}
#[allow(clippy::too_many_arguments)]
fn visible_position(
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    ax: i32,
    ay: i32,
    aw: u32,
    ah: u32,
) -> (i32, i32) {
    (
        x.clamp(ax, ax.saturating_add(aw.saturating_sub(w) as i32)),
        y.clamp(ay, ay.saturating_add(ah.saturating_sub(h) as i32)),
    )
}
#[cfg(target_os = "windows")]
fn set_native_opacity(window: &WebviewWindow, value: u8) -> Result<(), CaptureError> {
    use windows::Win32::{
        Foundation::COLORREF,
        UI::WindowsAndMessaging::{
            GWL_EXSTYLE, GetWindowLongPtrW, LWA_ALPHA, SetLayeredWindowAttributes,
            SetWindowLongPtrW, WS_EX_LAYERED,
        },
    };
    let hwnd = window
        .hwnd()
        .map_err(|_| failure("无法访问贴图原生窗口。"))?;
    // Tauri owns this HWND. Only add layering; never add WS_EX_TRANSPARENT (click-through).
    unsafe {
        let old = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, old | WS_EX_LAYERED.0 as isize);
        SetLayeredWindowAttributes(
            hwnd,
            COLORREF(0),
            (u16::from(value) * 255 / 100) as u8,
            LWA_ALPHA,
        )
        .map_err(|_| failure("Windows 无法调整贴图透明度。"))?;
    }
    Ok(())
}
#[cfg(not(target_os = "windows"))]
fn set_native_opacity(_window: &WebviewWindow, _value: u8) -> Result<(), CaptureError> {
    Err(CaptureError::new(
        ErrorCode::Unsupported,
        "本轮贴图透明度仅支持 Windows。",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn image() -> PinImage {
        PinImage {
            png: vec![0; 100],
            width: 320,
            height: 200,
        }
    }
    #[test]
    fn labels_ownership_and_resource_lifetime() {
        let m = PinManager::default();
        let a = m.reserve(image()).unwrap();
        let b = m.reserve(image()).unwrap();
        assert_ne!(a, b);
        assert!(m.image("main").is_err());
        let held = m.image(&a).unwrap();
        m.release(&a);
        assert!(m.image(&a).is_err());
        assert_eq!(held.png.len(), 100);
        assert_eq!(m.status().unwrap().count, 1);
        m.release(&a);
        m.release(&b);
        assert_eq!(m.status().unwrap().retained_bytes, 0);
    }
    #[test]
    fn count_and_memory_are_bounded() {
        let m = PinManager::default();
        for _ in 0..MAX_PINS {
            m.reserve(image()).unwrap();
        }
        assert!(m.reserve(image()).is_err());
        let m = PinManager::default();
        let big = || PinImage {
            png: vec![],
            width: 4096,
            height: 4096,
        };
        for _ in 0..4 {
            m.reserve(big()).unwrap();
        }
        assert!(m.reserve(big()).is_err());
        assert_eq!(m.status().unwrap().count, 4);
    }
    #[test]
    fn repeated_close_does_not_accumulate() {
        let m = PinManager::default();
        for _ in 0..200 {
            let id = m.reserve(image()).unwrap();
            m.release(&id);
        }
        assert_eq!(m.status().unwrap().count, 0);
        assert_eq!(m.status().unwrap().retained_bytes, 0);
    }
    #[test]
    fn zoom_keeps_aspect_and_limits() {
        assert_eq!(scaled_size(320, 200, 2.0, 1920, 1080).unwrap(), (640, 400));
        assert_eq!(
            scaled_size(320, 200, 20.0, 1920, 1080).unwrap(),
            (1280, 800)
        );
        assert_eq!(scaled_size(320, 200, 0.001, 1920, 1080).unwrap(), (48, 30));
        assert_eq!(
            scaled_size(4000, 2000, 1.0, 1000, 800).unwrap(),
            (1000, 500)
        );
        assert!(scaled_size(1, 1, f64::NAN, 1, 1).is_err());
        assert!(scaled_size(0, 1, 1.0, 100, 100).is_err());
    }
    #[test]
    fn recovery_supports_negative_origins() {
        assert_eq!(
            visible_position(-4000, 3000, 320, 200, -1920, -300, 1920, 1080),
            (-1920, 580)
        );
        assert_eq!(
            visible_position(100, 100, 9999, 9999, 0, 0, 800, 600),
            (0, 0)
        );
    }
}
