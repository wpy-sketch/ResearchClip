//! One bounded WGC request per same-executable worker. Some Windows/driver builds
//! retain ALPC connections after WGC Close/Release; process exit bounds that lifetime.
use super::{Target, failure, windows};
use crate::capture::{CaptureError, ErrorCode, MAX_PNG_BYTES, RawFrame};
use ::windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::JobObjects::*,
};
use std::os::windows::{io::AsRawHandle, process::CommandExt};
use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
const ARG: &str = "--researchclip-window-capture-worker";
const MAX_REQUEST: u64 = 8192;
struct Job(HANDLE);
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
impl Job {
    fn assign(child: &Child) -> Result<Self, CaptureError> {
        let handle = unsafe { CreateJobObjectW(None, None) }
            .map_err(|_| failure("无法限制窗口截图进程。"))?;
        let job = Self(handle);
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
        info.ProcessMemoryLimit = 512 * 1024 * 1024;
        unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as _,
                std::mem::size_of_val(&info) as u32,
            )
            .and_then(|()| AssignProcessToJobObject(handle, HANDLE(child.as_raw_handle())))
        }
        .map_err(|_| failure("无法隔离窗口截图进程，已停止捕获。"))?;
        Ok(job)
    }
}
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
pub(super) fn capture(target: &Target, cancel: &AtomicBool) -> Result<RawFrame, CaptureError> {
    if cancel.load(Ordering::Acquire) {
        return Err(CaptureError::new(ErrorCode::Cancelled, "已取消窗口截图。"));
    }
    windows::validate(target)?;
    let request = serde_json::to_vec(target).map_err(|_| failure("无法准备窗口截图请求。"))?;
    if request.len() > MAX_REQUEST as usize {
        return Err(failure("窗口信息超过安全上限。"));
    }
    let executable = std::env::current_exe().map_err(|_| failure("无法定位本机截图程序。"))?;
    let mut process = Process(
        Command::new(executable)
            .arg(ARG)
            .creation_flags(0x08000000)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| failure("无法启动本地窗口截图进程。"))?,
    );
    let _job = Job::assign(&process.0)?;
    let mut input = process
        .0
        .stdin
        .take()
        .ok_or_else(|| failure("窗口截图通道不可用。"))?;
    input
        .write_all(&request)
        .map_err(|_| failure("无法发送窗口截图请求。"))?;
    drop(input);
    let output = process
        .0
        .stdout
        .take()
        .ok_or_else(|| failure("窗口截图响应通道不可用。"))?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        output
            .take(MAX_PNG_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let start = Instant::now();
    let result = loop {
        if cancel.load(Ordering::Acquire) {
            break Err(CaptureError::new(ErrorCode::Cancelled, "已取消窗口截图。"));
        }
        if start.elapsed() > Duration::from_secs(6) {
            break Err(CaptureError::new(
                ErrorCode::Timeout,
                "窗口截图进程超时，已停止并释放资源。",
            ));
        }
        match process.0.try_wait() {
            Ok(Some(status)) => {
                break if status.success() {
                    Ok(())
                } else {
                    Err(failure("本地窗口截图进程异常退出，请选择普通窗口重试。"))
                };
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => break Err(failure("无法读取窗口截图进程状态。")),
        }
    };
    if result.is_err() {
        let _ = process.0.kill();
        let _ = process.0.wait();
    }
    let bytes = reader
        .join()
        .map_err(|_| failure("窗口图像读取任务异常结束。"))?
        .map_err(|_| failure("窗口图像传输失败。"))?;
    result?;
    if bytes.len() > MAX_PNG_BYTES {
        return Err(CaptureError::new(
            ErrorCode::ResourceLimit,
            "窗口图像超过安全传输上限。",
        ));
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        windows::validate(target)?;
        crate::services::workflow::decode_png(&bytes)
    } else if bytes.len() <= 4096 {
        Err(serde_json::from_slice::<CaptureError>(&bytes)
            .unwrap_or_else(|_| failure("窗口截图进程返回无效响应。")))
    } else {
        Err(failure("窗口截图进程返回无效响应。"))
    }
}
pub(super) fn entry() -> bool {
    if std::env::args_os().nth(1).as_deref() != Some(std::ffi::OsStr::new(ARG)) {
        return false;
    }
    let result = (|| {
        let mut request = Vec::new();
        std::io::stdin()
            .take(MAX_REQUEST + 1)
            .read_to_end(&mut request)
            .map_err(|_| failure("窗口截图请求读取失败。"))?;
        let target = parse_request(&request)?;
        windows::validate(&target)?;
        let device = windows::Device::new()?;
        let frame = windows::capture(&device, &target, &AtomicBool::new(false))?;
        crate::services::screen_capture::encode_png(frame)
    })();
    let bytes = match result {
        Ok(png) => png,
        Err(error) => serde_json::to_vec(&error).unwrap_or_default(),
    };
    let _ = std::io::stdout().lock().write_all(&bytes);
    true
}

fn parse_request(bytes: &[u8]) -> Result<Target, CaptureError> {
    if bytes.len() > MAX_REQUEST as usize {
        return Err(failure("窗口截图请求超出上限。"));
    }
    let target: Target =
        serde_json::from_slice(bytes).map_err(|_| failure("窗口截图请求无效。"))?;
    if target.handle == 0
        || target.pid == 0
        || target.thread == 0
        || target.class.is_empty()
        || target.class.len() > 1024
        || target.title.len() > 2048
    {
        return Err(failure("窗口截图身份无效。"));
    }
    crate::capture::rgba_len(target.bounds.width, target.bounds.height)?;
    Ok(target)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> serde_json::Value {
        serde_json::json!({"handle":123,"pid":9,"thread":8,"class":"fixture","title":"中文测试","bounds":{"x":-1920,"y":0,"width":640,"height":480},"dpi":120,"protected":false})
    }
    #[test]
    fn request_roundtrip_preserves_physical_target() {
        let target = parse_request(&serde_json::to_vec(&valid()).unwrap()).unwrap();
        assert_eq!(target.bounds.x, -1920);
        assert_eq!(target.dpi, 120);
        assert_eq!(target.title, "中文测试");
    }
    #[test]
    fn malformed_and_oversized_requests_are_refused() {
        assert!(parse_request(b"{}").is_err());
        assert!(parse_request(&vec![b' '; 8193]).is_err());
        let mut value = valid();
        value["arbitraryPath"] = "C:/private".into();
        assert!(parse_request(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    #[test]
    fn invalid_identity_and_pixel_budget_are_refused() {
        let mut value = valid();
        value["pid"] = 0.into();
        assert!(parse_request(&serde_json::to_vec(&value).unwrap()).is_err());
        let mut value = valid();
        value["bounds"]["width"] = u32::MAX.into();
        assert!(parse_request(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}
