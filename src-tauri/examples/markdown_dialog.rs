// Test helper only: the one owned Markdown folder picker and artifact directories.
#[cfg(target_os = "windows")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    use windows::{
        Win32::{
            Foundation::{HWND, LPARAM, WPARAM},
            UI::WindowsAndMessaging::*,
        },
        core::BOOL,
    };
    struct Target {
        pid: u32,
        windows: Vec<HWND>,
    }
    unsafe extern "system" fn find(hwnd: HWND, value: LPARAM) -> BOOL {
        let target = unsafe { &mut *(value.0 as *mut Target) };
        let mut pid = 0;
        let mut text = [0; 128];
        let mut class = [0; 32];
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        let n = unsafe { GetWindowTextW(hwnd, &mut text) } as usize;
        let c = unsafe { GetClassNameW(hwnd, &mut class) } as usize;
        if pid == target.pid
            && String::from_utf16_lossy(&text[..n]) == "选择 Markdown 导出目录"
            && String::from_utf16_lossy(&class[..c]) == "#32770"
        {
            target.windows.push(hwnd);
        }
        BOOL(1)
    }
    struct Control {
        hwnd: HWND,
        id: i32,
        class: String,
        name: String,
    }
    unsafe extern "system" fn child(hwnd: HWND, value: LPARAM) -> BOOL {
        let controls = unsafe { &mut *(value.0 as *mut Vec<Control>) };
        let mut text = [0; 512];
        let mut class = [0; 128];
        let n = unsafe { GetWindowTextW(hwnd, &mut text) } as usize;
        let c = unsafe { GetClassNameW(hwnd, &mut class) } as usize;
        controls.push(Control {
            hwnd,
            id: unsafe { GetDlgCtrlID(hwnd) },
            class: String::from_utf16_lossy(&class[..c]),
            name: String::from_utf16_lossy(&text[..n]),
        });
        BOOL(1)
    }
    let args: Vec<_> = std::env::args().collect();
    let action = args.get(1).ok_or("Expected inspect/cancel/folder")?;
    if !["inspect", "cancel", "folder"].contains(&action.as_str()) {
        return Err("Unsupported action".into());
    }
    let mut target = Target {
        pid: args.get(2).ok_or("Expected owned PID")?.parse()?,
        windows: Vec::new(),
    };
    if target.pid == 0 {
        return Err("Invalid PID".into());
    }
    unsafe {
        EnumWindows(Some(find), LPARAM((&mut target as *mut Target) as isize))?;
    }
    if target.windows.len() != 1 {
        return Err("Expected exactly one owned Markdown folder dialog".into());
    }
    let mut controls: Vec<Control> = Vec::new();
    unsafe {
        let _ = EnumChildWindows(
            Some(target.windows[0]),
            Some(child),
            LPARAM((&mut controls as *mut Vec<Control>) as isize),
        );
    }
    if action == "inspect" {
        unsafe {
            let _ = ShowWindow(target.windows[0], SW_SHOWNORMAL);
            let _ = SetForegroundWindow(target.windows[0]);
        }
        println!(
            "{}",
            serde_json::to_string(
                &controls
                    .iter()
                    .map(
                        |c| serde_json::json!({"id":c.id.to_string(),"class":c.class,"name":c.name})
                    )
                    .collect::<Vec<_>>()
            )?
        );
        return Ok(());
    }
    if action == "cancel" {
        unsafe {
            PostMessageW(Some(target.windows[0]), WM_CLOSE, WPARAM(0), LPARAM(0))?;
        }
        return Ok(());
    }
    let folder = PathBuf::from(args.get(3).ok_or("Expected artifact folder")?).canonicalize()?;
    let root = std::env::current_dir()?.join("artifacts").canonicalize()?;
    if !folder.starts_with(&root) || folder == root || !folder.is_dir() {
        return Err("Only owned artifact folders may be selected".into());
    }
    let edits: Vec<_> = controls
        .iter()
        .filter(|c| c.class == "Edit" && [1152, 1001].contains(&c.id))
        .collect();
    let buttons: Vec<_> = controls
        .iter()
        .filter(|c| c.class == "Button" && c.id == 1)
        .collect();
    if edits.len() != 1 || buttons.len() != 1 {
        return Err(format!(
            "Unexpected folder controls: edits={}, confirm={}",
            edits.len(),
            buttons.len()
        )
        .into());
    }
    if !unsafe { IsWindowEnabled(buttons[0].hwnd) }.as_bool() {
        return Err("Folder dialog not ready".into());
    }
    let text: Vec<_> = PathBuf::from(args.get(3).ok_or("Expected folder")?)
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    // Only the already PID/title-verified test-owned picker is raised. Never send input
    // unless it owns foreground; restore its normal z-order even on an early error.
    struct Raised(HWND);
    impl Drop for Raised {
        fn drop(&mut self) {
            unsafe {
                let _ = SetWindowPos(
                    self.0,
                    Some(HWND_NOTOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }
    let _raised = Raised(target.windows[0]);
    unsafe {
        use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
        let foreground = GetWindowThreadProcessId(GetForegroundWindow(), None);
        let current = GetCurrentThreadId();
        let attached = foreground != 0
            && foreground != current
            && AttachThreadInput(current, foreground, true).as_bool();
        let _ = ShowWindow(target.windows[0], SW_RESTORE);
        let raised = SetWindowPos(
            target.windows[0],
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE,
        );
        let _ = SetForegroundWindow(target.windows[0]);
        if attached {
            let _ = AttachThreadInput(current, foreground, false);
        }
        raised?;
        if !IsWindowVisible(edits[0].hwnd).as_bool() || !IsWindowVisible(buttons[0].hwnd).as_bool()
        {
            return Err(
                "Native folder controls are hidden; not confirming a different directory".into(),
            );
        }
        let mut foreground_pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut foreground_pid));
        if foreground_pid != target.pid {
            return Err("Owned folder picker is not foreground; refusing keyboard input".into());
        }
        let key = |vk, flags| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        };
        // Navigate the actual dialog address bar; changing the folder-name Edit alone
        // can leave IFileDialog's selected directory unchanged. The JS test also
        // requires the returned export path to match the intended artifact parent.
        let mut inputs = vec![
            key(VK_CONTROL, KEYBD_EVENT_FLAGS(0)),
            key(VIRTUAL_KEY(b'L' as u16), KEYBD_EVENT_FLAGS(0)),
            key(VIRTUAL_KEY(b'L' as u16), KEYEVENTF_KEYUP),
            key(VK_CONTROL, KEYEVENTF_KEYUP),
        ];
        for ch in text.iter().take(text.len() - 1) {
            for flags in [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP] {
                inputs.push(INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wScan: *ch,
                            dwFlags: flags,
                            ..Default::default()
                        },
                    },
                });
            }
        }
        inputs.extend([
            key(VK_RETURN, KEYBD_EVENT_FLAGS(0)),
            key(VK_RETURN, KEYEVENTF_KEYUP),
        ]);
        if SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) != inputs.len() as u32 {
            return Err("Scoped folder navigation input failed".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(700));
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut foreground_pid));
        if foreground_pid != target.pid {
            return Err("Folder picker lost focus; not confirming".into());
        }
        SendMessageW(buttons[0].hwnd, BM_CLICK, None, None);
    }
    Ok(())
}
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Windows-only native acceptance helper");
    std::process::exit(1);
}
