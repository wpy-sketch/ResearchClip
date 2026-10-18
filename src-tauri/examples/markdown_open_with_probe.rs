// Acceptance only: inspect/cancel a single visible popup owned by a supplied
// verified test process or newly started System32/OpenWith host. Never sends
// global input or changes file associations.
#[cfg(target_os = "windows")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use windows::{
        Win32::{
            Foundation::{HWND, LPARAM, WPARAM},
            UI::WindowsAndMessaging::*,
        },
        core::BOOL,
    };
    struct Owned {
        pid: u32,
        windows: Vec<HWND>,
    }
    unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> BOOL {
        let s = unsafe { &mut *(data.0 as *mut Owned) };
        if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
            return BOOL(1);
        }
        let mut current = hwnd;
        for _ in 0..8 {
            let mut pid = 0;
            unsafe {
                GetWindowThreadProcessId(current, Some(&mut pid));
            }
            if pid == s.pid {
                let mut title = [0u16; 128];
                let len = unsafe { GetWindowTextW(hwnd, &mut title) };
                if String::from_utf16_lossy(&title[..len as usize]) != "ResearchClip" {
                    s.windows.push(hwnd);
                }
                break;
            }
            match unsafe { GetWindow(current, GW_OWNER) } {
                Ok(owner) if owner.0 != current.0 => current = owner,
                _ => break,
            }
        }
        BOOL(1)
    }
    let args: Vec<_> = std::env::args().collect();
    let pid: u32 = args.get(2).ok_or("expected PID")?.parse()?;
    if pid == 0 {
        return Err("invalid PID".into());
    }
    let mut s = Owned {
        pid,
        windows: vec![],
    };
    unsafe {
        EnumWindows(Some(collect), LPARAM(&mut s as *mut _ as isize))?;
    }
    let rows: Vec<_> = s
        .windows
        .iter()
        .map(|hwnd| {
            let mut title = [0u16; 128];
            let len = unsafe { GetWindowTextW(*hwnd, &mut title) };
            {let mut pid=0;let mut class=[0u16;128];unsafe{GetWindowThreadProcessId(*hwnd,Some(&mut pid));}let class_len=unsafe{GetClassNameW(*hwnd,&mut class)};
              serde_json::json!({"pid":pid,"class":String::from_utf16_lossy(&class[..class_len as usize]),"title":String::from_utf16_lossy(&title[..len as usize])})}
        })
        .collect();
    if args.get(1).map(String::as_str) == Some("cancel") {
        if s.windows.len() != 1 {
            return Err("expected exactly one owned popup; refusing input".into());
        }
        unsafe {
            PostMessageW(Some(s.windows[0]), WM_CLOSE, WPARAM(0), LPARAM(0))?;
        }
    }
    println!("{}", serde_json::to_string(&rows)?);
    Ok(())
}
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Windows acceptance helper only");
}
