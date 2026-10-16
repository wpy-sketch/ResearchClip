// Real desktop acceptance probe. Prints only metadata/counters, never screenshot pixels.
use researchclip_lib::{capture, services::screen_capture::ScreenCaptureService};

#[cfg(target_os = "windows")]
struct CounterAnchor(windows::Win32::Graphics::Gdi::HDC);
#[cfg(target_os = "windows")]
impl Drop for CounterAnchor {
    fn drop(&mut self) {
        assert!(unsafe { windows::Win32::Graphics::Gdi::DeleteDC(self.0) }.as_bool());
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = ScreenCaptureService::default();
    let displays = service.enumerate_displays()?;
    println!("{}", serde_json::to_string(&displays)?);
    let selected = displays
        .iter()
        .find(|display| display.is_primary)
        .unwrap_or(&displays[0]);
    let count = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "1".into())
        .parse::<usize>()?
        .min(50);
    #[cfg(target_os = "windows")]
    let _anchor = {
        let dc = unsafe { windows::Win32::Graphics::Gdi::CreateCompatibleDC(None) };
        assert!(!dc.is_invalid());
        CounterAnchor(dc)
    };
    #[cfg(target_os = "windows")]
    let initial = unsafe {
        windows::Win32::System::Threading::GetGuiResources(
            windows::Win32::System::Threading::GetCurrentProcess(),
            windows::Win32::System::Threading::GR_GDIOBJECTS,
        )
    };
    #[cfg(target_os = "windows")]
    assert!(initial > 0, "GDI counter must observe the live test DC");
    for _ in 0..count {
        let _permit = service.try_begin()?;
        let bytes = service.capture_display(&selected.id)?;
        assert!(bytes.len() <= capture::MAX_PNG_BYTES);
        assert_eq!(
            u32::from_be_bytes(bytes[16..20].try_into()?),
            selected.width_px
        );
        assert_eq!(
            u32::from_be_bytes(bytes[20..24].try_into()?),
            selected.height_px
        );
    }
    assert_eq!(
        capture::capture_display("missing-test-monitor")
            .err()
            .unwrap()
            .code,
        capture::ErrorCode::DisplayGone
    );
    #[cfg(target_os = "windows")]
    {
        let final_count = unsafe {
            windows::Win32::System::Threading::GetGuiResources(
                windows::Win32::System::Threading::GetCurrentProcess(),
                windows::Win32::System::Threading::GR_GDIOBJECTS,
            )
        };
        println!(
            "captures={count}; GDI objects before={initial}, after={final_count}; invalid display rejected"
        );
        assert_eq!(initial, final_count, "GDI resource count changed");
    }
    Ok(())
}
