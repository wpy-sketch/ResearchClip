// Acceptance only. Restricts UI actions to one supplied app PID and fixed keys.
#[cfg(target_os = "windows")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    use windows::{
        Win32::{
            Foundation::{HWND, LPARAM, WPARAM},
            UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
        },
        core::BOOL,
    };
    let args: Vec<_> = std::env::args().collect();
    let mode = args.get(1).ok_or("Expected action")?;
    if mode == "hold-f11" || mode == "probe-free" {
        let keys = if mode == "hold-f11" {
            vec![VK_F11.0 as u32]
        } else {
            vec![VK_F9.0 as u32, VK_F10.0 as u32, VK_F11.0 as u32]
        };
        for (n, key) in keys.iter().enumerate() {
            unsafe {
                RegisterHotKey(
                    None,
                    300 + n as i32,
                    MOD_CONTROL | MOD_ALT | MOD_SHIFT,
                    *key,
                )?;
            }
        }
        println!("READY");
        std::io::stdout().flush()?;
        if mode == "hold-f11" {
            std::thread::sleep(std::time::Duration::from_secs(120));
        }
        for n in 0..keys.len() {
            unsafe {
                UnregisterHotKey(None, 300 + n as i32)?;
            }
        }
        return Ok(());
    }
    struct Owned {
        pid: u32,
        main: Vec<HWND>,
        tray: Vec<HWND>,
        popup: Vec<HWND>,
    }
    unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> BOOL {
        let owned = unsafe { &mut *(data.0 as *mut Owned) };
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        if pid != owned.pid {
            return BOOL(1);
        }
        let mut class = [0; 128];
        let mut name = [0; 128];
        let c = unsafe { GetClassNameW(hwnd, &mut class) } as usize;
        let n = unsafe { GetWindowTextW(hwnd, &mut name) } as usize;
        let class = String::from_utf16_lossy(&class[..c]);
        let name = String::from_utf16_lossy(&name[..n]);
        if name == "ResearchClip" {
            owned.main.push(hwnd);
        }
        if class == "tray_icon_app" {
            owned.tray.push(hwnd);
        }
        if class == "#32768" && unsafe { IsWindowVisible(hwnd) }.as_bool() {
            owned.popup.push(hwnd);
        }
        BOOL(1)
    }
    let mut owned = Owned {
        pid: args.get(2).ok_or("Expected test PID")?.parse()?,
        main: Vec::new(),
        tray: Vec::new(),
        popup: Vec::new(),
    };
    if owned.pid == 0 {
        return Err("Invalid PID".into());
    }
    unsafe {
        EnumWindows(Some(collect), LPARAM((&mut owned as *mut Owned) as isize))?;
    }
    if mode == "inspect" {
        let main = owned.main.first().copied();
        println!(
            "{}",
            serde_json::json!({"mainCount":owned.main.len(),"trayCount":owned.tray.len(),"popupCount":owned.popup.len(),"visible":main.is_some_and(|w|unsafe{IsWindowVisible(w)}.as_bool()),"minimized":main.is_some_and(|w|unsafe{IsIconic(w)}.as_bool()),"mainHandle":main.map(|w|w.0 as usize)})
        );
        return Ok(());
    }
    if owned.main.len() != 1 || owned.tray.len() != 1 {
        return Err("Expected one owned main window and one native tray window".into());
    }
    match mode.as_str() {
        "minimize" => unsafe {
            let _ = ShowWindow(owned.main[0], SW_MINIMIZE);
        },
        "close" => unsafe {
            PostMessageW(Some(owned.main[0]), WM_CLOSE, WPARAM(0), LPARAM(0))?;
        },
        "tray-left" | "menu-begin" => {
            let message = if mode == "tray-left" {
                WM_LBUTTONUP
            } else {
                WM_RBUTTONUP
            };
            unsafe {
                PostMessageW(
                    Some(owned.tray[0]),
                    6002,
                    WPARAM(0),
                    LPARAM(message as isize),
                )?;
            }
        }
        "choose-open" | "choose-region" | "choose-fullscreen" | "choose-settings"
        | "choose-exit" => {
            if owned.popup.len() != 1 {
                return Err("Expected the actual owned tray popup menu".into());
            }
            let mut foreground_pid = 0;
            unsafe {
                GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut foreground_pid));
            }
            if foreground_pid != owned.pid {
                return Err("Owned tray menu is not foreground; refusing input".into());
            }
            let key = match mode.as_str() {
                "choose-open" => b'O',
                "choose-region" => b'R',
                "choose-fullscreen" => b'F',
                "choose-settings" => b'S',
                _ => b'Q',
            };
            let mut inputs = Vec::new();
            for flags in [KEYBD_EVENT_FLAGS(0), KEYEVENTF_KEYUP] {
                inputs.push(INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(key as u16),
                            dwFlags: flags,
                            ..Default::default()
                        },
                    },
                });
            }
            if unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) }
                != inputs.len() as u32
            {
                return Err("Native menu input failed".into());
            }
        }
        _ => return Err("Unsupported fixed acceptance action".into()),
    }
    Ok(())
}
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Windows-only acceptance helper");
    std::process::exit(1);
}
