// Acceptance helper: limited to the test-owned foreground browser and fixed shortcuts.
// It is not linked into the application and cannot invoke arbitrary shell/UI actions.
#[cfg(target_os = "windows")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use windows::Win32::UI::{
        Input::KeyboardAndMouse::*,
        WindowsAndMessaging::{
            GetClassNameW, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
        },
    };
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).ok_or("expected hold, probe, or send")?;
    if mode == "close"
        || mode == "minimize"
        || mode == "dialog-cancel"
        || mode == "dispatch-hotkey"
        || mode == "dialog-save-native"
        || mode == "dialog-confirm"
        || mode == "dialog-open-native"
        || mode == "dialog-open-compare"
        || mode == "dialog-save-md"
        || mode == "dialog-save-linked-md"
        || mode == "dialog-open-bib"
        || mode == "dialog-open-md"
        || mode == "dialog-model-folder"
    {
        use windows::Win32::{
            Foundation::{HWND, LPARAM, WPARAM},
            UI::WindowsAndMessaging::{
                BM_CLICK, EnumChildWindows, EnumWindows, GetDlgCtrlID, IsWindowVisible,
                PostMessageW, SW_MINIMIZE, SendMessageW, SetWindowTextW, ShowWindow, WM_CHAR,
                WM_CLOSE, WM_GETTEXT, WM_HOTKEY,
            },
        };
        use windows::core::BOOL;
        struct Target {
            pid: u32,
            hwnd: Option<HWND>,
            count: u32,
            titles: Vec<Vec<u16>>,
            hotkey: bool,
        }
        unsafe extern "system" fn find(hwnd: HWND, data: LPARAM) -> BOOL {
            let target = unsafe { &mut *(data.0 as *mut Target) };
            let mut pid = 0;
            unsafe {
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
            }
            if pid == target.pid {
                let mut title = [0u16; 64];
                let len = unsafe { GetWindowTextW(hwnd, &mut title) };
                let mut class = [0u16; 64];
                let class_len = unsafe { GetClassNameW(hwnd, &mut class) };
                let expected_class = [
                    103, 108, 111, 98, 97, 108, 95, 104, 111, 116, 107, 101, 121, 95, 97, 112, 112,
                ];
                if (target.hotkey && class.get(..class_len as usize) == Some(&expected_class[..]))
                    || target
                        .titles
                        .iter()
                        .any(|expected| title.get(..len as usize) == Some(expected.as_slice()))
                {
                    target.count += 1;
                    target.hwnd = Some(hwnd);
                }
            }
            BOOL(1)
        }
        let mut target = Target {
            pid: args.get(2).ok_or("expected owned app PID")?.parse()?,
            hwnd: None,
            count: 0,
            titles: if mode == "dialog-model-folder" {
                vec![
                    "选择完整离线包中的 similarity-ai 文件夹"
                        .encode_utf16()
                        .collect(),
                ]
            } else if mode == "dialog-save-md" {
                vec!["保存 Markdown 文档".encode_utf16().collect()]
            } else if mode == "dialog-save-linked-md" {
                vec![
                    "导出标准Markdown副本（不覆盖已有文件）"
                        .encode_utf16()
                        .collect(),
                ]
            } else if mode == "dialog-open-bib" {
                vec!["导入 BibTeX 文献".encode_utf16().collect()]
            } else if mode == "dialog-open-md" {
                vec!["打开 Markdown 文档".encode_utf16().collect()]
            } else if mode == "dialog-open-compare" {
                vec!["选择对比图片".encode_utf16().collect()]
            } else if mode == "dialog-open-native" {
                vec!["打开可编辑项目".encode_utf16().collect()]
            } else if mode == "dialog-confirm" {
                vec![
                    "保存可编辑项目".encode_utf16().collect(),
                    "导出标注 PNG".encode_utf16().collect(),
                    "保存贴图 PNG".encode_utf16().collect(),
                    "选择对比图片".encode_utf16().collect(),
                    "打开可编辑项目".encode_utf16().collect(),
                    "保存 Markdown 文档".encode_utf16().collect(),
                    "打开 Markdown 文档".encode_utf16().collect(),
                    "导入 BibTeX 文献".encode_utf16().collect(),
                ]
            } else if mode == "dialog-save-native" {
                vec!["保存截图 PNG".encode_utf16().collect()]
            } else if mode == "dispatch-hotkey" {
                vec![]
            } else if mode == "dialog-cancel" {
                vec![
                    "保存截图 PNG".encode_utf16().collect(),
                    "选择截图默认保存目录".encode_utf16().collect(),
                    "保存 Markdown 文档".encode_utf16().collect(),
                    "打开 Markdown 文档".encode_utf16().collect(),
                    "导入 BibTeX 文献".encode_utf16().collect(),
                    "保存可编辑项目".encode_utf16().collect(),
                    "导出标注 PNG".encode_utf16().collect(),
                    "保存贴图 PNG".encode_utf16().collect(),
                    "选择对比图片".encode_utf16().collect(),
                    "打开可编辑项目".encode_utf16().collect(),
                ]
            } else {
                vec!["ResearchClip".encode_utf16().collect()]
            },
            hotkey: mode == "dispatch-hotkey",
        };
        if target.pid == 0 {
            return Err("invalid owned app PID".into());
        }
        unsafe {
            EnumWindows(Some(find), LPARAM((&mut target as *mut Target) as isize))?;
        }
        if target.count != 1 {
            return Err("expected exactly one owned ResearchClip main window".into());
        }
        if mode == "dialog-save-native"
            || mode == "dialog-confirm"
            || mode == "dialog-open-native"
            || mode == "dialog-open-compare"
            || mode == "dialog-save-md"
            || mode == "dialog-save-linked-md"
            || mode == "dialog-open-bib"
            || mode == "dialog-open-md"
            || mode == "dialog-model-folder"
        {
            #[derive(Default)]
            struct Fields {
                edit: Option<HWND>,
                save: Option<HWND>,
                edit_count: u32,
                save_count: u32,
                observed_edit_ids: [i32; 16],
                observed_count: usize,
            }
            unsafe extern "system" fn fields(hwnd: HWND, data: LPARAM) -> BOOL {
                let found = unsafe { &mut *(data.0 as *mut Fields) };
                let mut class = [0u16; 64];
                let len = unsafe { GetClassNameW(hwnd, &mut class) };
                let id = unsafe { GetDlgCtrlID(hwnd) };
                if class.get(..len as usize) == Some(&[69, 100, 105, 116][..])
                    && found.observed_count < found.observed_edit_ids.len()
                {
                    found.observed_edit_ids[found.observed_count] = id;
                    found.observed_count += 1;
                }
                if class.get(..len as usize) == Some(&[69, 100, 105, 116][..])
                    && [1152, 1001, 1148].contains(&id)
                    && unsafe { IsWindowVisible(hwnd) }.as_bool()
                {
                    found.edit = Some(hwnd);
                    found.edit_count += 1;
                }
                if class.get(..len as usize) == Some(&[66, 117, 116, 116, 111, 110][..])
                    && id == 1
                    && unsafe { IsWindowVisible(hwnd) }.as_bool()
                {
                    found.save = Some(hwnd);
                    found.save_count += 1;
                }
                BOOL(1)
            }
            let path =
                std::path::PathBuf::from(args.get(3).ok_or("expected test image/project path")?);
            let root = std::env::current_dir()?.join("artifacts").canonicalize()?;
            if !path.is_absolute()
                || !path
                    .parent()
                    .ok_or("no parent")?
                    .canonicalize()?
                    .starts_with(root)
                || !(mode == "dialog-model-folder" && path.is_dir()
                    || path.extension().is_some_and(|s| {
                        if mode == "dialog-open-bib" {
                            s == "bib"
                        } else if mode == "dialog-save-md"
                            || mode == "dialog-open-md"
                            || mode == "dialog-save-linked-md"
                        {
                            s == "md"
                        } else {
                            s == "png"
                                || s == "json"
                                || (mode == "dialog-open-compare"
                                    && ["jpg", "jpeg", "webp", "JPEG"].iter().any(|ext| s == *ext))
                        }
                    }))
            {
                return Err("test saves must stay in artifacts".into());
            }
            let mut controls = Fields::default();
            unsafe {
                let _ = EnumChildWindows(
                    target.hwnd,
                    Some(fields),
                    LPARAM((&mut controls as *mut Fields) as isize),
                );
            }
            if controls.edit_count != 1 || controls.save_count != 1 {
                return Err(format!(
                    "native filename/save controls unavailable: edit={}, save={}, observed Edit IDs={:?}",
                    controls.edit_count, controls.save_count, &controls.observed_edit_ids[..controls.observed_count]
                )
                .into());
            }
            let value: Vec<u16> = path
                .to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            unsafe {
                if !IsWindowEnabled(controls.save.ok_or("no primary button")?).as_bool() {
                    return Err("native primary button is not ready".into());
                }
                let edit = controls.edit.ok_or("no filename")?;
                if mode == "dialog-open-native"
                    && (!path.is_file() || path.extension().is_none_or(|s| s != "json"))
                {
                    return Err("expected existing artifacts project JSON".into());
                }
                if mode == "dialog-open-md" && !path.is_file() {
                    return Err("expected existing artifacts Markdown".into());
                }
                SetWindowTextW(edit, windows::core::w!(""))?;
                // Character messages notify the real file-name controller for both Open and Save.
                for ch in value.iter().take(value.len() - 1) {
                    SendMessageW(edit, WM_CHAR, Some(WPARAM(*ch as usize)), None);
                }
                let mut observed = vec![0u16; value.len() + 1];
                let n = SendMessageW(
                    edit,
                    WM_GETTEXT,
                    Some(WPARAM(observed.len())),
                    Some(LPARAM(observed.as_mut_ptr() as isize)),
                )
                .0 as usize;
                if observed.get(..n) != Some(&value[..value.len() - 1]) {
                    return Err(
                        "Native file-name control did not accept the verified artifact path".into(),
                    );
                }
                SendMessageW(controls.save.ok_or("no Save button")?, BM_CLICK, None, None);
            }
        } else if mode == "dispatch-hotkey" {
            let key = match args.get(3).map(String::as_str) {
                None | Some("new") => 0x33,
                Some("default") => 0x32,
                Some("full-default") => 0x31,
                Some("full-new") => 0x34,
                _ => return Err("unsupported helper shortcut".into()),
            };
            let shortcut: tauri_plugin_global_shortcut::Shortcut = format!(
                "Ctrl+Shift+{}",
                char::from_u32(key).ok_or("invalid fixed test key")?
            )
            .parse()?;
            unsafe {
                PostMessageW(
                    target.hwnd,
                    WM_HOTKEY,
                    WPARAM(shortcut.id() as usize),
                    LPARAM(((key << 16) | 6) as isize),
                )?;
            }
        } else if mode == "minimize" {
            unsafe {
                let _ = ShowWindow(target.hwnd.ok_or("no main window")?, SW_MINIMIZE);
            }
        } else {
            unsafe {
                PostMessageW(target.hwnd, WM_CLOSE, WPARAM(0), LPARAM(0))?;
            }
        }
        println!("CLOSE_SENT");
        return Ok(());
    }
    if mode == "lock-clipboard" {
        use std::io::Write;
        use windows::Win32::System::DataExchange::{CloseClipboard, OpenClipboard};
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, WS_EX_TOOLWINDOW, WS_POPUP,
        };
        let owner = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                windows::core::w!("STATIC"),
                windows::core::w!("ResearchClip clipboard lock probe"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
        }?;
        if let Err(error) = unsafe { OpenClipboard(Some(owner)) } {
            unsafe { DestroyWindow(owner) }?;
            return Err(error.into());
        }
        println!("CLIPBOARD_LOCKED");
        std::io::stdout().flush()?;
        let mut release = String::new();
        std::io::stdin().read_line(&mut release)?;
        unsafe {
            CloseClipboard()?;
            DestroyWindow(owner)?;
        }
        return Ok(());
    }
    if mode == "probe-clipboard" {
        use windows::Win32::System::DataExchange::{
            CloseClipboard, GetOpenClipboardWindow, OpenClipboard,
        };
        if unsafe { OpenClipboard(None) }.is_ok() {
            unsafe { CloseClipboard()? };
            println!("CLIPBOARD_FREE");
        } else {
            let mut pid = 0;
            if let Ok(owner) = unsafe { GetOpenClipboardWindow() } {
                unsafe { GetWindowThreadProcessId(owner, Some(&mut pid)) };
            }
            println!("CLIPBOARD_BUSY owner_pid={pid}");
        }
        return Ok(());
    }
    if mode == "hold" || mode == "probe" {
        let key = match args.get(2).map(String::as_str) {
            Some("default") => 0x32,
            Some("new") | None => 0x33,
            Some("full-default") => 0x31,
            Some("full-new") => 0x34,
            _ => return Err("unsupported helper shortcut".into()),
        };
        unsafe {
            RegisterHotKey(None, 6321, MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT, key)?;
        }
        println!("REGISTERED");
        if mode == "hold" {
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
        unsafe {
            UnregisterHotKey(None, 6321)?;
        }
        return Ok(());
    }
    if mode != "send" && mode != "dialog" {
        return Err("unsupported helper mode".into());
    }
    let expected_pid: u32 = args.get(2).ok_or("expected foreground PID")?.parse()?;
    let mut actual_pid = 0;
    unsafe {
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut actual_pid));
    }
    if expected_pid == 0 || actual_pid != expected_pid {
        return Err("foreground belongs to another application; input not sent".into());
    }
    let send = |sequence: &[INPUT]| -> Result<(), Box<dyn std::error::Error>> {
        let written = unsafe { SendInput(sequence, std::mem::size_of::<INPUT>() as i32) };
        if written != sequence.len() as u32 {
            let release = [VK_CONTROL, VK_SHIFT, VK_MENU].map(|key| INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: key,
                        dwFlags: KEYEVENTF_KEYUP,
                        ..Default::default()
                    },
                },
            });
            unsafe {
                SendInput(&release, std::mem::size_of::<INPUT>() as i32);
            }
            return Err("Windows rejected injected acceptance input".into());
        }
        Ok(())
    };
    let virtual_input = |key, up| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                ..Default::default()
            },
        },
    };
    if mode == "dialog" {
        let hwnd = unsafe { GetForegroundWindow() };
        let mut title = [0u16; 256];
        let mut class = [0u16; 64];
        let title_len = unsafe { GetWindowTextW(hwnd, &mut title) };
        let class_len = unsafe { GetClassNameW(hwnd, &mut class) };
        let title = String::from_utf16_lossy(&title[..title_len as usize]);
        let class = String::from_utf16_lossy(&class[..class_len as usize]);
        if class != "#32770"
            || ![
                "保存截图 PNG",
                "选择截图默认保存目录",
                "确认另存为",
                "Confirm Save As",
            ]
            .contains(&title.as_str())
        {
            return Err("expected app-owned system dialog is not foreground".into());
        }
        if args.get(3).is_some_and(|s| s == "cancel") {
            send(&[
                virtual_input(VK_ESCAPE, false),
                virtual_input(VK_ESCAPE, true),
            ])?;
        } else {
            let path = std::path::PathBuf::from(args.get(3).ok_or("expected test output path")?);
            let root = std::env::current_dir()?.join("artifacts").canonicalize()?;
            if !path.is_absolute()
                || !path
                    .parent()
                    .ok_or("no parent")?
                    .canonicalize()?
                    .starts_with(&root)
                || !path.extension().is_some_and(|s| s == "png")
                || title != "保存截图 PNG"
            {
                return Err("dialog test path must be an artifacts PNG".into());
            }
            send(&[
                virtual_input(VK_CONTROL, false),
                virtual_input(VIRTUAL_KEY(0x41), false),
                virtual_input(VIRTUAL_KEY(0x41), true),
                virtual_input(VK_CONTROL, true),
            ])?;
            let unicode: Vec<INPUT> = path
                .to_string_lossy()
                .encode_utf16()
                .flat_map(|unit| {
                    [false, true].map(|up| INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: VIRTUAL_KEY(0),
                                wScan: unit,
                                dwFlags: KEYEVENTF_UNICODE
                                    | if up {
                                        KEYEVENTF_KEYUP
                                    } else {
                                        KEYBD_EVENT_FLAGS(0)
                                    },
                                ..Default::default()
                            },
                        },
                    })
                })
                .collect();
            for chunk in unicode.chunks(128) {
                send(chunk)?;
            }
            send(&[
                virtual_input(VK_RETURN, false),
                virtual_input(VK_RETURN, true),
            ])?;
        }
        println!("DIALOG_INPUT_SENT");
        return Ok(());
    }
    let key = match args.get(3).map(String::as_str) {
        Some("default") => VIRTUAL_KEY(0x32),
        Some("new") => VIRTUAL_KEY(0x33),
        Some("full-default") => VIRTUAL_KEY(0x31),
        Some("full-new") => VIRTUAL_KEY(0x34),
        Some("paste") => VIRTUAL_KEY(0x56),
        _ => return Err("unsupported helper shortcut".into()),
    };
    let mut keys = vec![VK_CONTROL];
    if key.0 != 0x56 {
        keys.push(VK_SHIFT);
    }
    keys.push(key);
    let input = |key, up| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let mut sequence = Vec::new();
    for &key in &keys {
        sequence.push(input(key, false));
    }
    for &key in keys.iter().rev() {
        sequence.push(input(key, true));
    }
    send(&sequence)?;
    println!("INPUT_SENT");
    Ok(())
}
#[cfg(not(target_os = "windows"))]
fn main() {
    panic!("workflow_input only supports Windows");
}
