// Test-only helper: raise the known fixture window owned by the supplied browser process.
// It never changes other applications or ResearchClip's production window configuration.
#[cfg(target_os = "windows")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use windows::{
        Win32::{
            Foundation::{HWND, LPARAM},
            Graphics::Dwm::DwmFlush,
            UI::WindowsAndMessaging::{
                EnumWindows, GetWindowTextW, GetWindowThreadProcessId, HWND_TOPMOST,
                IsWindowVisible, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
            },
        },
        core::BOOL,
    };
    struct Fixture<'a> {
        pid: u32,
        title: &'a [u16],
        handle: Option<HWND>,
        multiple: bool,
    }
    unsafe extern "system" fn find_fixture(window: HWND, data: LPARAM) -> BOOL {
        let fixture = unsafe { &mut *(data.0 as *mut Fixture<'_>) };
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
        if pid == fixture.pid && unsafe { IsWindowVisible(window) }.as_bool() {
            let mut title = [0u16; 512];
            let len = unsafe { GetWindowTextW(window, &mut title) };
            if len > 0 && title[..len as usize].starts_with(fixture.title) {
                fixture.multiple |= fixture.handle.is_some();
                fixture.handle = Some(window);
            }
        }
        BOOL(1)
    }
    let pid = std::env::args()
        .nth(1)
        .ok_or("Missing fixture browser PID")?
        .parse::<u32>()?;
    if pid == 0 {
        return Err("Invalid fixture PID".into());
    }
    let title: Vec<u16> = "ResearchClip capture acceptance fixture"
        .encode_utf16()
        .collect();
    let mut fixture = Fixture {
        pid,
        title: &title,
        handle: None,
        multiple: false,
    };
    unsafe { EnumWindows(Some(find_fixture), LPARAM(&mut fixture as *mut _ as isize)) }?;
    if fixture.multiple {
        return Err("Multiple fixture windows; refuse to alter them".into());
    }
    let window = fixture
        .handle
        .ok_or("Fixture window was not found in this browser process")?;
    // A foreground transition can raise another desktop window when main hides.
    // Reassert only our fixture after verifying that our application's main HWND is hidden.
    if let Some(app_pid) = std::env::args().nth(2) {
        struct AppWindow {
            pid: u32,
            found: usize,
            visible: bool,
        }
        unsafe extern "system" fn app_window(window: HWND, data: LPARAM) -> BOOL {
            let state = unsafe { &mut *(data.0 as *mut AppWindow) };
            let mut pid = 0;
            unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
            if pid == state.pid {
                let mut title = [0u16; 64];
                let n = unsafe { GetWindowTextW(window, &mut title) };
                if n > 0 && String::from_utf16_lossy(&title[..n as usize]) == "ResearchClip" {
                    state.found += 1;
                    state.visible = unsafe { IsWindowVisible(window) }.as_bool();
                }
            }
            BOOL(1)
        }
        let pid = app_pid.parse::<u32>()?;
        if pid == 0 || pid == fixture.pid {
            return Err("Invalid app PID".into());
        }
        let start = std::time::Instant::now();
        loop {
            let mut state = AppWindow {
                pid,
                found: 0,
                visible: true,
            };
            unsafe { EnumWindows(Some(app_window), LPARAM(&mut state as *mut _ as isize)) }?;
            if state.found == 1 && !state.visible {
                break;
            }
            if start.elapsed() > std::time::Duration::from_secs(2) {
                return Err("The owned app did not hide before native capture".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    unsafe {
        SetWindowPos(
            window,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }?;
    unsafe { DwmFlush() }?;
    println!("Owned capture fixture is topmost");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn main() {
    panic!("The native capture fixture is Windows-only");
}
