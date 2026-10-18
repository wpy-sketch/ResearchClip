use super::scrolling::{Bounds, Target};
use super::stitching::failure;
use crate::capture::{CaptureError, DisplayInfo, geometry::RectPx};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT},
        Graphics::Gdi::ClientToScreen,
        System::Threading::{
            OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
        UI::{HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
    core::{BOOL, PWSTR},
};

struct Dpi(DPI_AWARENESS_CONTEXT);
impl Dpi {
    fn enter() -> Result<Self, CaptureError> {
        let prev =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        if prev.0.is_null() {
            Err(failure("无法启用物理像素坐标。"))
        } else {
            Ok(Self(prev))
        }
    }
}
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe { SetThreadDpiAwarenessContext(self.0) };
    }
}
fn hwnd(t: &Target) -> HWND {
    HWND(t.handle as *mut _)
}
fn bounds(w: HWND) -> Result<Bounds, CaptureError> {
    let mut rect = RECT::default();
    let mut point = POINT::default();
    unsafe { GetClientRect(w, &mut rect) }.map_err(|_| failure("目标窗口已不可用。"))?;
    if !unsafe { ClientToScreen(w, &mut point) }.as_bool() {
        return Err(failure("无法读取窗口位置。"));
    }
    Ok(Bounds {
        x: point.x,
        y: point.y,
        width: (rect.right - rect.left).max(0) as u32,
        height: (rect.bottom - rect.top).max(0) as u32,
    })
}
pub fn list() -> Result<Vec<Target>, CaptureError> {
    struct Handles {
        items: [usize; 256],
        len: usize,
    }
    unsafe extern "system" fn visit(w: HWND, p: LPARAM) -> BOOL {
        let a = unsafe { &mut *(p.0 as *mut Handles) };
        if a.len < a.items.len() && unsafe { IsWindowVisible(w) }.as_bool() {
            a.items[a.len] = w.0 as usize;
            a.len += 1;
        }
        BOOL(1)
    }
    let _dpi = Dpi::enter()?;
    let mut a = Handles {
        items: [0; 256],
        len: 0,
    };
    unsafe { EnumWindows(Some(visit), LPARAM(&mut a as *mut _ as isize)) }
        .map_err(|_| failure("无法枚举目标窗口。"))?;
    let mut result = Vec::new();
    for handle in &a.items[..a.len] {
        let w = HWND(*handle as *mut _);
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(w, Some(&mut pid)) };
        if pid == std::process::id() || unsafe { IsIconic(w) }.as_bool() {
            continue;
        }
        let Ok(rect) = bounds(w) else {
            continue;
        };
        if rect.width < 128 || rect.height < 160 {
            continue;
        }
        let mut text = [0u16; 180];
        let len = unsafe { GetWindowTextW(w, &mut text) };
        if len <= 0 {
            continue;
        }
        let mut class = [0u16; 128];
        let c = unsafe { GetClassNameW(w, &mut class) };
        let mut executable = String::new();
        if let Ok(process) = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
            let mut path = [0u16; 1024];
            let mut size = path.len() as u32;
            if unsafe {
                QueryFullProcessImageNameW(
                    process,
                    PROCESS_NAME_WIN32,
                    PWSTR(path.as_mut_ptr()),
                    &mut size,
                )
            }
            .is_ok()
            {
                executable = String::from_utf16_lossy(&path[..size as usize])
                    .rsplit('\\')
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();
            }
            unsafe {
                let _ = CloseHandle(process);
            }
        }
        result.push(Target {
            id: String::new(),
            title: String::from_utf16_lossy(&text[..len as usize]),
            automatic: matches!(executable.as_str(), "chrome.exe" | "msedge.exe")
                && String::from_utf16_lossy(&class[..c.max(0) as usize]) == "Chrome_WidgetWin_1",
            handle: *handle,
            pid,
            bounds: rect,
        });
    }
    Ok(result)
}
pub fn validate(t: &Target) -> Result<(), CaptureError> {
    let _dpi = Dpi::enter()?;
    let w = hwnd(t);
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(w, Some(&mut pid)) };
    let mut affinity = 0;
    if pid != t.pid
        || !unsafe { IsWindowVisible(w) }.as_bool()
        || unsafe { IsIconic(w) }.as_bool()
        || bounds(w)? != t.bounds
    {
        return Err(failure("目标窗口已移动、缩放、最小化或关闭，请重新选择。"));
    }
    unsafe { GetWindowDisplayAffinity(w, &mut affinity) }
        .map_err(|_| failure("无法确认窗口捕获权限；不继续捕获。"))?;
    if affinity != 0 {
        return Err(failure("目标窗口限制屏幕捕获，不支持对此内容进行长截图。"));
    }
    Ok(())
}
pub fn activate(t: &Target) -> Result<(), CaptureError> {
    validate(t)?;
    if unsafe { GetForegroundWindow() } != hwnd(t)
        && !unsafe { SetForegroundWindow(hwnd(t)) }.as_bool()
    {
        return Err(failure("Windows未允许激活目标。请先点击该窗口再重试。"));
    }
    Ok(())
}
fn point(display: &DisplayInfo, r: RectPx, x: u32, y: u32) -> POINT {
    POINT {
        x: display.x_px + r.x as i32 + x as i32,
        y: display.y_px + r.y as i32 + y as i32,
    }
}
pub fn visible(t: &Target, display: &DisplayInfo, r: RectPx) -> Result<(), CaptureError> {
    let _dpi = Dpi::enter()?;
    validate(t)?;
    if unsafe { GetForegroundWindow() } != hwnd(t) {
        return Err(failure(
            "目标失去前台焦点，已停止输入和采集。请停止保留已有部分，或取消重选。",
        ));
    }
    for (x, y) in [
        (8, 8),
        (r.width - 9, 8),
        (8, r.height - 9),
        (r.width - 9, r.height - 9),
        (r.width / 2, r.height / 2),
    ] {
        let root = unsafe { GetAncestor(WindowFromPoint(point(display, r, x, y)), GA_ROOT) };
        if root != hwnd(t) {
            return Err(failure(
                "选区被其他窗口遮挡，已停止采集。请移除遮挡后重新选择。",
            ));
        }
    }
    Ok(())
}
pub fn cursor(t: &Target, d: &DisplayInfo, r: RectPx) -> Result<(), CaptureError> {
    let _dpi = Dpi::enter()?;
    visible(t, d, r)?;
    let p = point(d, r, r.width / 2, r.height / 2);
    unsafe { SetCursorPos(p.x, p.y) }.map_err(|_| failure("无法定位目标滚动区域。"))
}
pub fn wheel(t: &Target, d: &DisplayInfo, r: RectPx) -> Result<(), CaptureError> {
    let _dpi = Dpi::enter()?;
    visible(t, d, r)?;
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p) }.map_err(|_| failure("无法读取鼠标位置。"))?;
    let left = d.x_px + r.x as i32;
    let top = d.y_px + r.y as i32;
    if p.x < left || p.y < top || p.x >= left + r.width as i32 || p.y >= top + r.height as i32 {
        return Err(failure(
            "鼠标已离开选区，已停止自动滚动；不会向其他窗口发送输入。",
        ));
    }
    for key in [
        VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN, VK_LBUTTON, VK_RBUTTON,
    ] {
        if unsafe { GetAsyncKeyState(key.0 as i32) } < 0 {
            return Err(failure(
                "检测到按键或鼠标按钮按住，已暂停自动滚动，避免误触缩放或其他操作。",
            ));
        }
    }
    // One notch at a time, never PostMessage to a background window. Recheck each time.
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                mouseData: (-120i32) as u32,
                dwFlags: MOUSEEVENTF_WHEEL,
                ..Default::default()
            },
        },
    };
    if unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) } != 1 {
        return Err(failure(
            "Windows拒绝滚动输入（可能是权限级别限制）。请改用手动模式。",
        ));
    }
    Ok(())
}
