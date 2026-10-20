// Diagnostic counts only: no handle names, paths or contents are read.
#[cfg(target_os = "windows")]
fn handle_types() -> std::collections::BTreeMap<String, u32> {
    use std::{collections::BTreeMap, ffi::c_void};
    #[repr(C)]
    struct Unicode {
        len: u16,
        max: u16,
        text: *const u16,
    }
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtQueryObject(
            handle: *mut c_void,
            class: u32,
            buffer: *mut c_void,
            len: u32,
            written: *mut u32,
        ) -> i32;
    }
    let mut counts = BTreeMap::new();
    for value in (4..65536usize).step_by(4) {
        let handle = windows::Win32::Foundation::HANDLE(value as *mut _);
        let mut flags = 0;
        if unsafe { windows::Win32::Foundation::GetHandleInformation(handle, &mut flags) }.is_err()
        {
            continue;
        }
        let mut buffer = [0u64; 512];
        let mut written = 0;
        if unsafe { NtQueryObject(handle.0, 2, buffer.as_mut_ptr().cast(), 4096, &mut written) } < 0
        {
            continue;
        }
        let name = unsafe { &*(buffer.as_ptr() as *const Unicode) };
        let start = buffer.as_ptr() as usize;
        let pointer = name.text as usize;
        if pointer < start || pointer + name.len as usize > start + 4096 || name.len % 2 != 0 {
            continue;
        }
        let text = String::from_utf16_lossy(unsafe {
            std::slice::from_raw_parts(name.text, name.len as usize / 2)
        });
        *counts.entry(text).or_insert(0) += 1;
    }
    counts
}
// Opt-in native acceptance fixture. Own windows only; never writes user data.
#[cfg(target_os = "windows")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if researchclip_lib::services::window_capture::run_capture_worker() {
        return Ok(());
    }
    use researchclip_lib::services::{
        screen_capture::encode_png_ref,
        window_capture::{WindowCaptureService, list_targets},
    };
    use std::sync::atomic::AtomicBool;
    use windows::{
        Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*},
        core::w,
    };
    if std::env::args().nth(1).as_deref() == Some("clipboard-image") {
        use windows::Win32::System::{DataExchange::*, Memory::*};
        unsafe {
            OpenClipboard(None)?;
            struct Clipboard;
            impl Drop for Clipboard {
                fn drop(&mut self) {
                    unsafe {
                        let _ = CloseClipboard();
                    }
                }
            }
            let _clipboard = Clipboard;
            // Standard Windows formats: CF_DIBV5=17, CF_DIB=8. Neither is a file path.
            let format = if IsClipboardFormatAvailable(17).is_ok() {
                17
            } else {
                8
            };
            IsClipboardFormatAvailable(format)?;
            let handle = GetClipboardData(format)?;
            let global = HGLOBAL(handle.0);
            if GlobalSize(global) < std::mem::size_of::<BITMAPINFOHEADER>() {
                return Err("clipboard DIB header incomplete".into());
            }
            let bytes = GlobalLock(global);
            if bytes.is_null() {
                return Err("clipboard DIB lock failed".into());
            }
            let header = std::ptr::read_unaligned(bytes as *const BITMAPINFOHEADER);
            let _ = GlobalUnlock(global);
            if header.biWidth <= 0 || header.biHeight == 0 {
                return Err("invalid clipboard image size".into());
            }
            println!(
                "PASS Windows clipboard image DIB {}x{}",
                header.biWidth,
                header.biHeight.unsigned_abs()
            );
        }
        return Ok(());
    }
    let backend = WindowCaptureService::default();
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "fixture") {
        unsafe extern "system" fn procedure(
            hwnd: HWND,
            message: u32,
            w: WPARAM,
            l: LPARAM,
        ) -> LRESULT {
            if message == WM_PAINT {
                let mut paint = PAINTSTRUCT::default();
                let dc = unsafe { BeginPaint(hwnd, &mut paint) };
                let mut rect = RECT::default();
                let _ = unsafe { GetClientRect(hwnd, &mut rect) };
                let brush = unsafe { CreateSolidBrush(COLORREF(0x0033aa11)) };
                unsafe {
                    FillRect(dc, &rect, brush);
                    let _ = DeleteObject(brush.into());
                }
                rect.left = rect.right / 2;
                let brush = unsafe { CreateSolidBrush(COLORREF(0x001144dd)) };
                unsafe {
                    FillRect(dc, &rect, brush);
                    let _ = DeleteObject(brush.into());
                    let _ = EndPaint(hwnd, &paint);
                }
                return LRESULT(0);
            }
            if message == WM_DESTROY {
                unsafe { PostQuitMessage(0) };
                return LRESULT(0);
            }
            unsafe { DefWindowProcW(hwnd, message, w, l) }
        }
        unsafe {
            let class = WNDCLASSW {
                lpfnWndProc: Some(procedure),
                lpszClassName: w!("ResearchClipA07Fixture"),
                ..Default::default()
            };
            RegisterClassW(&class);
            let window = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("ResearchClipA07Fixture"),
                w!("ResearchClip A07 native fixture"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                150,
                150,
                640,
                480,
                None,
                None,
                None,
                None,
            )?;
            let _ = ShowWindow(window, SW_SHOWNORMAL);
            let _ = ShowWindow(window, SW_SHOWNORMAL);
            let _ = UpdateWindow(window);
            if args.get(2).is_some_and(|s| s == "protected") {
                SetWindowDisplayAffinity(window, WDA_EXCLUDEFROMCAPTURE)?;
            }
            if args.get(2).is_some_and(|s| s == "occluded") {
                CreateWindowExW(
                    WS_EX_TOPMOST,
                    w!("STATIC"),
                    w!("A07 occluder"),
                    WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                    300,
                    260,
                    260,
                    180,
                    None,
                    None,
                    None,
                    None,
                )?;
            }
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        return Ok(());
    }
    let mode = args.get(1).map(String::as_str).unwrap_or("probe");
    let mut target = list_targets()?
        .into_iter()
        .find(|t| {
            if mode == "browser" {
                t.title.contains("ResearchClip A07 browser fixture")
            } else {
                t.title == "ResearchClip A07 native fixture"
            }
        })
        .ok_or("fixture not found")?;
    if mode == "hover" || mode == "click" {
        let picker = unsafe { GetForegroundWindow() };
        let mut title = [0u16; 128];
        let len = unsafe { GetWindowTextW(picker, &mut title) };
        if String::from_utf16_lossy(&title[..len.max(0) as usize]) != "ResearchClip 窗口截图" {
            return Err("picker does not own foreground input".into());
        }
        unsafe {
            SetCursorPos(
                target.bounds.x + target.bounds.width as i32 * 3 / 4,
                target.bounds.y + target.bounds.height as i32 / 2,
            )?;
        }
        if mode == "click" {
            use windows::Win32::UI::Input::KeyboardAndMouse::*;
            let inputs = [MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP].map(|flags| INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dwFlags: flags,
                        ..Default::default()
                    },
                },
            });
            if unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) } != 2 {
                return Err("mouse input rejected".into());
            }
        }
        println!("PASS owned picker input {mode}");
        return Ok(());
    }
    let native = if mode == "browser" {
        HWND::default()
    } else {
        unsafe { FindWindowW(None, w!("ResearchClip A07 native fixture")) }?
    };
    if mode == "secondary" {
        let displays = researchclip_lib::capture::enumerate_displays()?;
        let Some(display) = displays.iter().find(|d| d.x_px < 0 || d.y_px < 0) else {
            println!("NOT_RUN negative-origin display absent");
            return Ok(());
        };
        unsafe {
            let _ = windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext(
                windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            );
            SetWindowPos(
                native,
                None,
                display.x_px + 100,
                display.y_px + 100,
                640,
                480,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )?;
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
        target = list_targets()?
            .into_iter()
            .find(|t| t.title == "ResearchClip A07 native fixture")
            .ok_or("moved fixture missing")?;
        let captured = backend.capture(&target, &AtomicBool::new(false));
        unsafe {
            SetWindowPos(
                native,
                None,
                150,
                150,
                640,
                480,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )?;
        }
        let frame = captured?;
        if frame
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[..3] == [0x11, 0xaa, 0x33])
            .count()
            < 80000
        {
            return Err("secondary display pixels invalid".into());
        }
        println!(
            "PASS negative-origin display WGC {}x{}; target origin {},{} at {} DPI",
            frame.width_px, frame.height_px, target.bounds.x, target.bounds.y, target.dpi
        );
        return Ok(());
    }
    if mode == "closed" {
        unsafe {
            PostMessageW(Some(native), WM_CLOSE, WPARAM(0), LPARAM(0))?;
        }
        for _ in 0..100 {
            if !unsafe { IsWindow(Some(native)) }.as_bool() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if unsafe { IsWindow(Some(native)) }.as_bool() {
            return Err("fixture did not close".into());
        }
        if backend.capture(&target, &AtomicBool::new(false)).is_ok() {
            return Err("closed target was captured".into());
        }
        println!("PASS closed target refused");
        return Ok(());
    }
    if mode == "cancel-active" {
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let flag = std::sync::Arc::clone(&cancel);
        let interrupt = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(40));
            flag.store(true, std::sync::atomic::Ordering::Release);
        });
        let result = backend.capture(&target, &cancel);
        interrupt.join().map_err(|_| "cancel thread failed")?;
        if !matches!(result,Err(error) if error.code==researchclip_lib::capture::ErrorCode::Cancelled)
        {
            return Err("in-progress capture not cancelled".into());
        }
        println!("PASS in-progress worker cancellation and cleanup");
        return Ok(());
    }
    if mode == "minimized" {
        unsafe {
            let _ = ShowWindow(native, SW_MINIMIZE);
        }
        let result = backend.capture(&target, &AtomicBool::new(false));
        unsafe {
            let _ = ShowWindow(native, SW_RESTORE);
        }
        if result.is_ok() {
            return Err("minimized window was captured".into());
        }
        println!("PASS minimized window refused");
        return Ok(());
    }
    let cancel = AtomicBool::new(args.get(1).is_some_and(|s| s == "cancel"));
    let captured = backend.capture(&target, &cancel);
    if mode == "cancel" || mode == "protected" {
        let code = if mode == "cancel" {
            researchclip_lib::capture::ErrorCode::Cancelled
        } else {
            researchclip_lib::capture::ErrorCode::PermissionDenied
        };
        match captured {
            Err(error) if error.code == code => {
                println!("PASS {mode} refused with {code:?}");
                return Ok(());
            }
            _ => return Err(format!("{mode} unexpected result").into()),
        }
    }
    let frame = captured?;
    // The occluder covers part of the right-side red panel. Both colors must survive.
    let green = frame
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[..3] == [0x11, 0xaa, 0x33])
        .count();
    let red = frame
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[..3] == [0xdd, 0x44, 0x11])
        .count();
    if green < 80000 || red < 80000 {
        return Err(format!("pixel verification failed: green={green}, red={red}").into());
    }
    if mode == "repeat" {
        use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};
        let types_before = handle_types();
        let mut before = 0;
        let mut after = 0;
        unsafe {
            GetProcessHandleCount(GetCurrentProcess(), &mut before)?;
        }
        for _ in 0..20 {
            let frame = backend.capture(&target, &cancel)?;
            if (frame.width_px, frame.height_px) != (626, 473) {
                return Err("repeat dimensions changed".into());
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
        unsafe {
            GetProcessHandleCount(GetCurrentProcess(), &mut after)?;
        }
        println!(
            "HANDLE TYPES before={types_before:?} after={:?}",
            handle_types()
        );
        if after > before + 8 {
            return Err(format!("handles grew: {before}->{after}").into());
        }
        println!("PASS 20 captures, process handles {before}->{after}");
    }
    if mode == "occlusion" {
        let cover = list_targets()?
            .into_iter()
            .find(|t| t.title == "A07 occluder")
            .ok_or("occluder missing")?;
        let x = cover.bounds.x + cover.bounds.width as i32 * 3 / 4;
        let y = cover.bounds.y + cover.bounds.height as i32 / 2;
        let display = researchclip_lib::capture::enumerate_displays()?
            .into_iter()
            .find(|d| {
                x >= d.x_px
                    && y >= d.y_px
                    && x < d.x_px + d.width_px as i32
                    && y < d.y_px + d.height_px as i32
            })
            .ok_or("fixture display missing")?;
        let desktop = researchclip_lib::capture::capture_display(&display.id)?;
        let index = |x: i32, y: i32, w: u32| (y as usize * w as usize + x as usize) * 4;
        let a = index(x - target.bounds.x, y - target.bounds.y, frame.width_px);
        let b = index(x - display.x_px, y - display.y_px, desktop.width_px);
        if frame.rgba[a..a + 3] == desktop.rgba[b..b + 3] {
            return Err("occlusion proof failed: identical desktop and WGC pixel".into());
        }
        println!("PASS actual desktop occluder differs from unobscured WGC window pixel");
    }
    if let Some(path) = args.get(2) {
        std::fs::write(path, encode_png_ref(&frame)?)?;
    }
    if mode == "browser" {
        println!("PASS Edge WGC {}x{}", frame.width_px, frame.height_px);
    }
    println!(
        "PASS window pixels {}x{}, green={}, red={}",
        frame.width_px, frame.height_px, green, red
    );
    Ok(())
}
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Windows only");
}
