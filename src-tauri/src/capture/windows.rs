use super::{CaptureError, DisplayInfo, ErrorCode, RawFrame, rgba_len};
use std::mem::size_of;
use windows::{
    Win32::{
        Foundation::{LPARAM, RECT},
        Graphics::{Dwm::DwmFlush, Gdi::*},
        UI::{
            HiDpi::{
                DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
                SetThreadDpiAwarenessContext,
            },
            WindowsAndMessaging::MONITORINFOF_PRIMARY,
        },
    },
    core::{BOOL, PCWSTR},
};

fn native_error(code: ErrorCode, message: &str) -> CaptureError {
    let error = windows::core::Error::from_thread();
    if error.code().0 as u32 == 0x80070005 {
        CaptureError::new(
            ErrorCode::PermissionDenied,
            &format!("{message} Windows 拒绝访问，请确认当前是已解锁的普通桌面。"),
        )
    } else {
        CaptureError::new(code, message)
    }
}

struct DpiContext(DPI_AWARENESS_CONTEXT);
impl DpiContext {
    fn enter() -> Result<Self, CaptureError> {
        // This thread alone uses physical desktop pixels; the previous context is restored.
        let previous =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        if previous.0.is_null() {
            return Err(native_error(
                ErrorCode::CaptureFailed,
                "无法设置截图线程的物理像素坐标模式。",
            ));
        }
        Ok(Self(previous))
    }
}
impl Drop for DpiContext {
    fn drop(&mut self) {
        if unsafe { SetThreadDpiAwarenessContext(self.0) }.0.is_null() {
            eprintln!("ResearchClip: capture DPI context restoration failed");
        }
    }
}

// Fixed capacity: never allocate or panic across the Windows callback boundary.
struct MonitorList {
    handles: [HMONITOR; 64],
    len: usize,
    overflow: bool,
}
unsafe extern "system" fn enum_monitor(
    monitor: HMONITOR,
    _: HDC,
    _: *mut RECT,
    data: LPARAM,
) -> BOOL {
    // EnumDisplayMonitors calls synchronously, with this valid stack-owned list.
    let list = unsafe { &mut *(data.0 as *mut MonitorList) };
    if list.len == list.handles.len() {
        list.overflow = true;
        return BOOL(0);
    }
    list.handles[list.len] = monitor;
    list.len += 1;
    BOOL(1)
}

fn read_display(monitor: HMONITOR) -> Result<DisplayInfo, CaptureError> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    if !unsafe { GetMonitorInfoW(monitor, &mut info as *mut _ as *mut MONITORINFO) }.as_bool() {
        return Err(native_error(
            ErrorCode::EnumerationFailed,
            "无法读取显示器信息，请刷新显示器列表。",
        ));
    }
    let len = info
        .szDevice
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(info.szDevice.len());
    let id = String::from_utf16(&info.szDevice[..len]).map_err(|_| {
        CaptureError::new(
            ErrorCode::EnumerationFailed,
            "Windows 返回了无效的显示器名称。",
        )
    })?;
    let rect = info.monitorInfo.rcMonitor;
    let width = rect
        .right
        .checked_sub(rect.left)
        .and_then(|v| u32::try_from(v).ok())
        .filter(|v| *v > 0);
    let height = rect
        .bottom
        .checked_sub(rect.top)
        .and_then(|v| u32::try_from(v).ok())
        .filter(|v| *v > 0);
    let (Some(width), Some(height)) = (width, height) else {
        return Err(CaptureError::new(
            ErrorCode::EnumerationFailed,
            "显示器返回了无效的像素尺寸。",
        ));
    };
    let mut mode = DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };
    if !unsafe {
        EnumDisplaySettingsW(
            PCWSTR(info.szDevice.as_ptr()),
            ENUM_CURRENT_SETTINGS,
            &mut mode,
        )
    }
    .as_bool()
    {
        return Err(native_error(
            ErrorCode::EnumerationFailed,
            "无法读取显示器当前分辨率。",
        ));
    }
    // Detect stale topology or DPI virtualization instead of guessing a multiplier.
    if (mode.dmPelsWidth, mode.dmPelsHeight) != (width, height) {
        return Err(CaptureError::new(
            ErrorCode::DisplayChanged,
            "显示器分辨率正在变化，或像素坐标不一致，请刷新后重试。",
        ));
    }
    Ok(DisplayInfo {
        name: id.clone(),
        id,
        width_px: width,
        height_px: height,
        x_px: rect.left,
        y_px: rect.top,
        is_primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
    })
}

fn list_in_physical_context() -> Result<Vec<DisplayInfo>, CaptureError> {
    let mut list = MonitorList {
        handles: [HMONITOR::default(); 64],
        len: 0,
        overflow: false,
    };
    let ok = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(enum_monitor),
            LPARAM(&mut list as *mut _ as isize),
        )
    }
    .as_bool();
    if list.overflow {
        return Err(CaptureError::new(
            ErrorCode::ResourceLimit,
            "显示器数量超出当前支持范围。",
        ));
    }
    if !ok {
        return Err(native_error(
            ErrorCode::EnumerationFailed,
            "无法枚举显示器，请确认 Windows 桌面可访问。",
        ));
    }
    if list.len == 0 {
        return Err(CaptureError::new(
            ErrorCode::NoDisplays,
            "当前没有可捕获的显示器。",
        ));
    }
    let mut displays = list.handles[..list.len]
        .iter()
        .map(|handle| read_display(*handle))
        .collect::<Result<Vec<_>, _>>()?;
    displays.sort_by(|a, b| b.is_primary.cmp(&a.is_primary).then(a.id.cmp(&b.id)));
    Ok(displays)
}

pub(super) fn enumerate_displays() -> Result<Vec<DisplayInfo>, CaptureError> {
    let _dpi = DpiContext::enter()?;
    list_in_physical_context()
}

#[derive(Default)]
struct GdiResources {
    screen: Option<HDC>,
    memory: Option<HDC>,
    bitmap: Option<HBITMAP>,
    previous: Option<HGDIOBJ>,
}
impl Drop for GdiResources {
    fn drop(&mut self) {
        unsafe {
            if let (Some(dc), Some(previous)) = (self.memory, self.previous)
                && SelectObject(dc, previous).is_invalid()
            {
                eprintln!("ResearchClip: capture bitmap deselection failed");
            }
            // Close the DC before deleting the bitmap, even if deselection failed.
            if let Some(dc) = self.memory
                && !DeleteDC(dc).as_bool()
            {
                eprintln!("ResearchClip: capture memory DC release failed");
            }
            if let Some(bitmap) = self.bitmap
                && !DeleteObject(bitmap.into()).as_bool()
            {
                eprintln!("ResearchClip: capture bitmap release failed");
            }
            if let Some(dc) = self.screen
                && ReleaseDC(None, dc) != 1
            {
                eprintln!("ResearchClip: capture screen DC release failed");
            }
        }
    }
}

pub(super) fn capture_display(id: &str) -> Result<RawFrame, CaptureError> {
    let _dpi = DpiContext::enter()?;
    let display = list_in_physical_context()?
        .into_iter()
        .find(|display| display.id == id)
        .ok_or_else(|| {
            CaptureError::new(
                ErrorCode::DisplayGone,
                "所选显示器已断开，请刷新列表后重新选择。",
            )
        })?;
    let size = rgba_len(display.width_px, display.height_px)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(size)
        .map_err(|_| CaptureError::new(ErrorCode::ResourceLimit, "没有足够内存捕获该显示器。"))?;
    rgba.resize(size, 0);
    unsafe { DwmFlush() }.map_err(|_| {
        CaptureError::new(
            ErrorCode::CaptureFailed,
            "无法等待桌面更新，请在普通 Windows 桌面重试。",
        )
    })?;
    let mut resources = GdiResources::default();
    let screen = unsafe { GetDC(None) };
    if screen.is_invalid() {
        return Err(native_error(ErrorCode::CaptureFailed, "无法访问桌面画面。"));
    }
    resources.screen = Some(screen);
    let memory = unsafe { CreateCompatibleDC(Some(screen)) };
    if memory.is_invalid() {
        return Err(native_error(
            ErrorCode::CaptureFailed,
            "无法建立截图绘图上下文。",
        ));
    }
    resources.memory = Some(memory);
    let bitmap = unsafe {
        CreateCompatibleBitmap(screen, display.width_px as i32, display.height_px as i32)
    };
    if bitmap.is_invalid() {
        return Err(native_error(ErrorCode::ResourceLimit, "无法分配截图位图。"));
    }
    resources.bitmap = Some(bitmap);
    let previous = unsafe { SelectObject(memory, bitmap.into()) };
    if previous.is_invalid() {
        return Err(native_error(ErrorCode::CaptureFailed, "无法选择截图位图。"));
    }
    resources.previous = Some(previous);
    unsafe {
        BitBlt(
            memory,
            0,
            0,
            display.width_px as i32,
            display.height_px as i32,
            Some(screen),
            display.x_px,
            display.y_px,
            SRCCOPY | CAPTUREBLT,
        )
    }
    .map_err(|_| {
        native_error(
            ErrorCode::CaptureFailed,
            "无法捕获显示器画面，请检查桌面权限后重试。",
        )
    })?;
    // GetDIBits requires a bitmap that is NOT selected into a DC.
    if unsafe { SelectObject(memory, previous) }.is_invalid() {
        return Err(native_error(ErrorCode::CaptureFailed, "无法读取截图位图。"));
    }
    resources.previous = None;
    let mut bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: display.width_px as i32,
            biHeight: -(display.height_px as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let lines = unsafe {
        GetDIBits(
            screen,
            bitmap,
            0,
            display.height_px,
            Some(rgba.as_mut_ptr().cast()),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        )
    };
    if lines != display.height_px as i32 {
        return Err(native_error(
            ErrorCode::CaptureFailed,
            "Windows 返回的截图像素不完整，请重试。",
        ));
    }
    drop(resources);
    let current = list_in_physical_context()?
        .into_iter()
        .find(|item| item.id == id);
    if current.as_ref() != Some(&display) {
        return Err(CaptureError::new(
            ErrorCode::DisplayChanged,
            "截图期间显示器配置发生变化，请刷新后重试。",
        ));
    }
    // Top-down BGRA from GDI; desktop alpha is undefined, so make the PNG opaque.
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
    Ok(RawFrame {
        width_px: display.width_px,
        height_px: display.height_px,
        rgba,
    })
}
