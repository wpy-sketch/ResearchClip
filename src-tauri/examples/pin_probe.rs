// Acceptance helper, never linked into the application. Only operates on the supplied PID's pins.
#[cfg(target_os = "windows")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use windows::{
        Win32::{
            Foundation::{COLORREF, HWND, LPARAM, POINT, RECT},
            UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
        },
        core::BOOL,
    };
    struct Owned {
        pid: u32,
        windows: Vec<(String, HWND)>,
    }
    unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> BOOL {
        let owned = unsafe { &mut *(data.0 as *mut Owned) };
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        if pid == owned.pid {
            let mut text = [0; 128];
            let len = unsafe { GetWindowTextW(hwnd, &mut text) } as usize;
            let title = String::from_utf16_lossy(&text[..len]);
            if let Some(label) = title.strip_prefix("ResearchClip 贴图 ") {
                owned.windows.push((label.to_string(), hwnd));
            }
        }
        BOOL(1)
    }
    unsafe {
        windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
    }
    let args: Vec<_> = std::env::args().collect();
    let mode = args.get(1).ok_or("expected mode")?;
    let mut owned = Owned {
        pid: args.get(2).ok_or("expected owned test PID")?.parse()?,
        windows: vec![],
    };
    if owned.pid == 0 {
        return Err("invalid PID".into());
    }
    unsafe {
        EnumWindows(Some(collect), LPARAM((&mut owned as *mut Owned) as isize))?;
    }
    if mode == "inspect" {
        let mut result = vec![];
        for (label, hwnd) in owned.windows {
            let mut rect = RECT::default();
            let mut client = RECT::default();
            let mut origin = POINT::default();
            unsafe {
                GetClientRect(hwnd, &mut client)?;
                windows::Win32::Graphics::Gdi::ClientToScreen(hwnd, &mut origin).ok()?;
            }
            let mut alpha = 255;
            let mut color = COLORREF(0);
            let mut flags = LAYERED_WINDOW_ATTRIBUTES_FLAGS(0);
            unsafe {
                GetWindowRect(hwnd, &mut rect)?;
            }
            let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
            let ex = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
            let layered = unsafe {
                GetLayeredWindowAttributes(
                    hwnd,
                    Some(&mut color),
                    Some(&mut alpha),
                    Some(&mut flags),
                )
            }
            .is_ok();
            result.push(serde_json::json!({"label":label,"left":rect.left,"top":rect.top,"width":rect.right-rect.left,"height":rect.bottom-rect.top,"visible":unsafe{IsWindowVisible(hwnd)}.as_bool(),"topmost":ex&WS_EX_TOPMOST.0!=0,"style":style,"exStyle":ex,"captionStyle":style&WS_CAPTION.0==WS_CAPTION.0,"clientFillsWindow":origin.x==rect.left&&origin.y==rect.top&&client.right==rect.right-rect.left&&client.bottom==rect.bottom-rect.top,"clickThrough":ex&WS_EX_TRANSPARENT.0!=0,"layered":layered,"alpha":alpha}));
        }
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }
    let label = args.get(3).ok_or("expected label")?;
    if !label.starts_with("pin-") || !label[4..].chars().all(|c| c.is_ascii_digit()) {
        return Err("invalid pin label".into());
    }
    let hwnd = owned
        .windows
        .iter()
        .find(|(l, _)| l == label)
        .map(|(_, h)| *h)
        .ok_or("owned pin missing")?;
    if mode == "offscreen" {
        unsafe {
            SetWindowPos(
                hwnd,
                None,
                -20000,
                -20000,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )?;
        }
        return Ok(());
    }
    if mode == "drag" {
        let mut rect = RECT::default();
        unsafe {
            GetWindowRect(hwnd, &mut rect)?;
            let _ = SetForegroundWindow(hwnd);
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        if unsafe { GetForegroundWindow() } != hwnd {
            return Err("test pin not foreground; no input sent".into());
        }
        let (x, y) = (
            rect.left + (rect.right - rect.left) / 2,
            rect.top + (rect.bottom - rect.top) / 2,
        );
        unsafe {
            SetCursorPos(x, y)?;
        }
        let mouse = |flags| INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        };
        let send =
            |flags| unsafe { SendInput(&[mouse(flags)], std::mem::size_of::<INPUT>() as i32) };
        if send(MOUSEEVENTF_LEFTDOWN) != 1 {
            return Err("mouse down rejected".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(400));
        let result = unsafe { SetCursorPos(x + 100, y + 80) };
        std::thread::sleep(std::time::Duration::from_millis(400));
        let released = send(MOUSEEVENTF_LEFTUP);
        result?;
        if released != 1 {
            return Err("mouse up rejected".into());
        }
        return Ok(());
    }
    Err("unsupported action".into())
}
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("pin_probe requires Windows");
    std::process::exit(1);
}
