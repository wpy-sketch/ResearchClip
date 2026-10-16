// Test helper only: the one owned Markdown folder picker and artifact directories.
#[cfg(target_os = "windows")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;
    use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
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
    unsafe {
        if !IsWindowVisible(edits[0].hwnd).as_bool() || !IsWindowVisible(buttons[0].hwnd).as_bool()
        {
            return Err(
                "Native folder controls are hidden; not confirming a different directory".into(),
            );
        }
        SendMessageW(
            edits[0].hwnd,
            WM_SETTEXT,
            Some(WPARAM(0)),
            Some(LPARAM(text.as_ptr() as isize)),
        );
        let mut observed = vec![0; text.len() + 1];
        // GetWindowText does not retrieve another process's Edit content.
        let n = SendMessageW(
            edits[0].hwnd,
            WM_GETTEXT,
            Some(WPARAM(observed.len())),
            Some(LPARAM(observed.as_mut_ptr() as isize)),
        )
        .0 as usize;
        if observed[..n] != text[..text.len() - 1] {
            return Err(format!("Native folder field did not accept test path: expected length {}, observed length {}",text.len()-1,n).into());
        }
        // The real folder text field now contains the verified artifact path.
        SendMessageW(buttons[0].hwnd, BM_CLICK, None, None);
    }
    Ok(())
}
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Windows-only native acceptance helper");
    std::process::exit(1);
}
