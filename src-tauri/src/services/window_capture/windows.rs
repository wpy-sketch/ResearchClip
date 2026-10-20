//! Window content comes from WGC, never from a crop of the desktop.
use super::{Bounds, Target, failure};
use crate::capture::{CaptureError, ErrorCode, RawFrame, rgba_len};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use windows::{
    Graphics::{
        Capture::*,
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
    Win32::{
        Foundation::{HMODULE, HWND, LPARAM, POINT, RECT},
        Graphics::{
            Direct3D::*,
            Direct3D11::*,
            Dwm::*,
            Dxgi::{Common::*, DXGI_ERROR_WAS_STILL_DRAWING, IDXGIDevice},
        },
        System::WinRT::{Direct3D11::*, Graphics::Capture::IGraphicsCaptureItemInterop},
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
    core::{AgileReference, BOOL, Interface, factory},
};

struct Dpi(DPI_AWARENESS_CONTEXT);
impl Dpi {
    fn enter() -> Result<Self, CaptureError> {
        let previous =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        if previous.0.is_null() {
            return Err(failure("无法读取物理像素窗口边界。"));
        }
        Ok(Self(previous))
    }
}
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe {
            SetThreadDpiAwarenessContext(self.0);
        }
    }
}
fn hwnd(target: &Target) -> HWND {
    HWND(target.handle as *mut _)
}
pub fn cursor() -> Result<(i32, i32), CaptureError> {
    let _dpi = Dpi::enter()?;
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p) }.map_err(|_| failure("无法读取鼠标位置。"))?;
    Ok((p.x, p.y))
}
fn target(w: HWND) -> Option<Target> {
    if !unsafe { IsWindowVisible(w) }.as_bool() || unsafe { IsIconic(w) }.as_bool() {
        return None;
    }
    let mut pid = 0;
    let thread = unsafe { GetWindowThreadProcessId(w, Some(&mut pid)) };
    if pid == std::process::id() || pid == 0 {
        return None;
    }
    let mut cloaked = 0u32;
    if unsafe { DwmGetWindowAttribute(w, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, 4) }
        .is_err()
        || cloaked != 0
    {
        return None;
    }
    let mut rect = RECT::default();
    if unsafe {
        DwmGetWindowAttribute(
            w,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )
    }
    .is_err()
    {
        return None;
    }
    if rect.right <= rect.left || rect.bottom <= rect.top {
        return None;
    }
    let mut name = [0u16; 512];
    let n = unsafe { GetWindowTextW(w, &mut name) };
    let mut class = [0u16; 256];
    let c = unsafe { GetClassNameW(w, &mut class) };
    let class = String::from_utf16_lossy(&class[..c.max(0) as usize]);
    if matches!(
        class.as_str(),
        "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
    ) {
        return None;
    }
    let mut affinity = 0;
    // An unavailable query is not evidence of capture permission. WGC remains the authority.
    let protected = unsafe { GetWindowDisplayAffinity(w, &mut affinity) }.is_ok() && affinity != 0;
    Some(Target {
        handle: w.0 as usize,
        pid,
        thread,
        class,
        title: if n > 0 {
            String::from_utf16_lossy(&name[..n as usize])
        } else {
            "无标题窗口".into()
        },
        bounds: Bounds {
            x: rect.left,
            y: rect.top,
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
        },
        dpi: unsafe { GetDpiForWindow(w) },
        protected,
    })
}
pub fn list() -> Result<Vec<Target>, CaptureError> {
    struct Handles {
        items: [usize; 512],
        len: usize,
        overflow: bool,
    }
    unsafe extern "system" fn visit(w: HWND, p: LPARAM) -> BOOL {
        let a = unsafe { &mut *(p.0 as *mut Handles) };
        if !unsafe { IsWindowVisible(w) }.as_bool() {
            return BOOL(1);
        }
        if a.len == a.items.len() {
            a.overflow = true;
            return BOOL(0);
        }
        a.items[a.len] = w.0 as usize;
        a.len += 1;
        BOOL(1)
    }
    let _dpi = Dpi::enter()?;
    let mut handles = Handles {
        items: [0; 512],
        len: 0,
        overflow: false,
    };
    let result = unsafe { EnumWindows(Some(visit), LPARAM(&mut handles as *mut _ as isize)) };
    if handles.overflow {
        return Err(failure("窗口数量超过安全上限，请关闭部分窗口后重试。"));
    }
    result.map_err(|_| failure("无法枚举桌面窗口。"))?;
    Ok(handles.items[..handles.len]
        .iter()
        .filter_map(|w| target(HWND(*w as *mut _)))
        .collect())
}
pub fn validate(expected: &Target) -> Result<(), CaptureError> {
    let _dpi = Dpi::enter()?;
    let actual = target(hwnd(expected))
        .ok_or_else(|| failure("目标窗口已关闭、最小化或不可见，请重新选择。"))?;
    if actual.pid != expected.pid
        || actual.thread != expected.thread
        || actual.class != expected.class
        || actual.title != expected.title
        || actual.bounds != expected.bounds
        || actual.dpi != expected.dpi
    {
        return Err(failure("目标窗口或其边界已变化，请重新选择。"));
    }
    if actual.protected {
        return Err(CaptureError::new(
            ErrorCode::PermissionDenied,
            "目标窗口受系统捕获保护，无法截图。",
        ));
    }
    Ok(())
}
fn api_error(error: windows::core::Error) -> CaptureError {
    if error.code().0 == 0x80070005u32 as i32 {
        CaptureError::new(
            ErrorCode::PermissionDenied,
            "Windows 拒绝访问此窗口。请检查权限或选择普通桌面窗口；不会绕过保护。",
        )
    } else {
        failure(&format!(
            "Windows Graphics Capture 无法获取此窗口图像（0x{:08X}）。请重试或使用普通桌面窗口；不会降级为桌面裁剪。",
            error.code().0 as u32
        ))
    }
}
struct Pool(Direct3D11CaptureFramePool);
impl Drop for Pool {
    fn drop(&mut self) {
        let _ = self.0.Close();
    }
}
struct Session(GraphicsCaptureSession);
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.0.Close();
    }
}
// Device::new has already activated the process factory/MTA lifetime. Explicitly
// initialize this worker so each new WGC item does not create an implicit apartment.
struct Apartment(bool);
impl Apartment {
    fn enter() -> Result<Self, CaptureError> {
        match unsafe {
            windows::Win32::System::WinRT::RoInitialize(
                windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
            )
        } {
            Ok(()) => Ok(Self(true)),
            Err(error) if error.code().0 == 0x80010106u32 as i32 => Ok(Self(false)),
            Err(error) => Err(api_error(error)),
        }
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                windows::Win32::System::WinRT::RoUninitialize();
            }
        }
    }
}
struct Surface(windows::Graphics::DirectX::Direct3D11::IDirect3DSurface);
impl Drop for Surface {
    fn drop(&mut self) {
        let _ = self.0.Close();
    }
}
struct DirectDevice(IDirect3DDevice);
impl Drop for DirectDevice {
    fn drop(&mut self) {
        let _ = self.0.Close();
    }
}
struct Frame(Direct3D11CaptureFrame);
impl Drop for Frame {
    fn drop(&mut self) {
        let _ = self.0.Close();
    }
}
struct Mapping<'a> {
    context: &'a ID3D11DeviceContext,
    texture: &'a ID3D11Texture2D,
}
impl Drop for Mapping<'_> {
    fn drop(&mut self) {
        unsafe {
            self.context.Unmap(self.texture, 0);
        }
    }
}
fn interrupted(cancel: &AtomicBool, start: Instant) -> Result<(), CaptureError> {
    if cancel.load(Ordering::Acquire) {
        Err(CaptureError::new(ErrorCode::Cancelled, "已取消窗口截图。"))
    } else if start.elapsed() > Duration::from_secs(3) {
        Err(CaptureError::new(
            ErrorCode::Timeout,
            "窗口图像获取超时，可能已最小化、受保护或暂不兼容。",
        ))
    } else {
        Ok(())
    }
}
pub(super) struct Device {
    interop: AgileReference<IGraphicsCaptureItemInterop>,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
}
impl Device {
    pub(super) fn new() -> Result<Self, CaptureError> {
        let mut device = None;
        let mut context = None;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
        }
        .map_err(api_error)?;
        let device = device.ok_or_else(|| failure("无法创建图形设备。"))?;
        let context = context.ok_or_else(|| failure("无法创建图形上下文。"))?;
        let factory =
            factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().map_err(api_error)?;
        let interop = AgileReference::new(&factory).map_err(api_error)?;
        Ok(Self {
            interop,
            device,
            context,
        })
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.context.ClearState();
            self.context.Flush();
        }
    }
}
pub fn capture(
    gpu: &Device,
    target: &Target,
    cancel: &AtomicBool,
) -> Result<RawFrame, CaptureError> {
    let _apartment = Apartment::enter()?;
    validate(target)?;
    if !GraphicsCaptureSession::IsSupported().map_err(api_error)? {
        return Err(CaptureError::new(
            ErrorCode::Unsupported,
            "此系统不支持 Windows Graphics Capture。需要 Windows 10 1903 或更新版本。",
        ));
    }
    // The worker owns its WinRT factory lifetime; it never opens application data.
    let interop = gpu.interop.resolve().map_err(api_error)?;
    let item: GraphicsCaptureItem =
        unsafe { interop.CreateForWindow(hwnd(target)) }.map_err(api_error)?;
    let size = item.Size().map_err(api_error)?;
    rgba_len(size.Width as u32, size.Height as u32)?;
    let device = &gpu.device;
    let context = &gpu.context;
    let dxgi: IDXGIDevice = device.cast().map_err(api_error)?;
    let direct = DirectDevice(
        unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi) }
            .map_err(api_error)?
            .cast()
            .map_err(api_error)?,
    );
    let pool = Pool(
        Direct3D11CaptureFramePool::CreateFreeThreaded(
            &direct.0,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )
        .map_err(api_error)?,
    );
    let session = Session(pool.0.CreateCaptureSession(&item).map_err(api_error)?);
    session.0.StartCapture().map_err(api_error)?;
    let start = Instant::now();
    let frame = loop {
        interrupted(cancel, start)?;
        match pool.0.TryGetNextFrame() {
            Ok(frame) => break Frame(frame),
            // windows-rs 0.62 maps S_OK + null frame to Error::empty (HRESULT 0).
            Err(error) if awaiting_frame(&error) => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => return Err(api_error(error)),
        }
    };
    let content = frame.0.ContentSize().map_err(api_error)?;
    if content != size {
        return Err(failure("窗口在截图时改变了尺寸，请重新选择。"));
    }
    let surface = Surface(frame.0.Surface().map_err(api_error)?);
    let access: IDirect3DDxgiInterfaceAccess = surface.0.cast().map_err(api_error)?;
    let texture: ID3D11Texture2D = unsafe { access.GetInterface() }.map_err(api_error)?;
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { texture.GetDesc(&mut desc) };
    if desc.Width < size.Width as u32
        || desc.Height < size.Height as u32
        || desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM
    {
        return Err(failure("窗口图像格式或缓冲区尺寸不兼容。"));
    }
    rgba_len(desc.Width, desc.Height)?;
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    desc.MiscFlags = 0;
    let mut staging = None;
    unsafe { device.CreateTexture2D(&desc, None, Some(&mut staging)) }.map_err(api_error)?;
    let staging = staging.ok_or_else(|| failure("无法创建图像读取缓冲区。"))?;
    unsafe { context.CopyResource(&staging, &texture) };
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    loop {
        interrupted(cancel, start)?;
        match unsafe {
            context.Map(
                &staging,
                0,
                D3D11_MAP_READ,
                D3D11_MAP_FLAG_DO_NOT_WAIT.0 as u32,
                Some(&mut mapped),
            )
        } {
            Ok(()) => break,
            Err(error) if error.code() == DXGI_ERROR_WAS_STILL_DRAWING => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(error) => return Err(api_error(error)),
        }
    }
    let _mapping = Mapping {
        context,
        texture: &staging,
    };
    let width = size.Width as u32;
    let height = size.Height as u32;
    if mapped.pData.is_null() || mapped.RowPitch < width * 4 {
        return Err(failure("窗口图像行长度无效。"));
    }
    let len = (mapped.RowPitch as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| failure("图像缓冲区超出上限。"))?;
    let bytes = unsafe { std::slice::from_raw_parts(mapped.pData as *const u8, len) };
    let rgba = super::convert_bgra(bytes, mapped.RowPitch as usize, width, height)?;
    validate(target)?;
    interrupted(cancel, start)?;
    Ok(RawFrame {
        width_px: width,
        height_px: height,
        rgba,
    })
}

fn awaiting_frame(error: &windows::core::Error) -> bool {
    error.code().0 == 0
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_success_with_no_frame_is_retried() {
        assert!(awaiting_frame(&windows::core::Error::empty()));
        assert!(!awaiting_frame(&windows::core::Error::from_hresult(
            windows::core::HRESULT(0x80070005u32 as i32)
        )));
    }
}
