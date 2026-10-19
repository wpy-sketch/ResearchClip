//! Opt-in per-session local IPC. This is length-framed JSON, never an HTTP server.
use crate::{
    capture::{CaptureError, ErrorCode},
    commands::library::LibraryService,
    services::screen_capture::ScreenCaptureService,
    storage::{clips::ClipSaved, files},
};
use researchclip_clipper_protocol::{self as p, Endpoint, Envelope, Reply};
use serde::Serialize;
use std::{
    fs,
    io::Write,
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};
struct Runtime {
    running: Arc<AtomicBool>,
    path: PathBuf,
    token: String,
}
#[derive(Default)]
pub struct ClipperService {
    runtime: Mutex<Option<Runtime>>,
    last: Arc<Mutex<Option<ClipSaved>>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    enabled: bool,
    extension_id: &'static str,
    last: Option<ClipSaved>,
}
fn failure() -> CaptureError {
    CaptureError::new(
        ErrorCode::SaveFailed,
        "网页剪藏连接不可用，请检查本地配置目录权限后重试。",
    )
}
impl ClipperService {
    pub fn status(&self) -> Result<Status, CaptureError> {
        Ok(Status {
            enabled: self
                .runtime
                .lock()
                .map_err(|_| failure())?
                .as_ref()
                .is_some_and(|r| r.running.load(Ordering::Acquire)),
            extension_id: p::EXTENSION_ID,
            last: self.last.lock().map_err(|_| failure())?.clone(),
        })
    }
    pub fn start(&self, app: AppHandle) -> Result<Status, CaptureError> {
        if !cfg!(target_os = "windows") {
            return Err(CaptureError::new(
                ErrorCode::Unsupported,
                "网页剪藏Host第一版仅验证Windows。",
            ));
        }
        let mut guard = self.runtime.lock().map_err(|_| failure())?;
        if guard
            .as_ref()
            .is_none_or(|r| !r.running.load(Ordering::Acquire))
        {
            let root = if cfg!(debug_assertions) {
                std::env::var_os("RESEARCHCLIP_TEST_SETTINGS_DIR").map(PathBuf::from)
            } else {
                None
            }
            .unwrap_or(app.path().app_config_dir().map_err(|_| failure())?);
            files::directory(&root).map_err(|_| failure())?;
            let path = root.join("clipper-endpoint-v1.json");
            files::existing_ordinary(&path).map_err(|_| failure())?;
            let token = p::random_token().map_err(|_| failure())?;
            let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|_| failure())?;
            listener.set_nonblocking(true).map_err(|_| failure())?;
            let e = Endpoint {
                version: p::VERSION,
                port: listener.local_addr().map_err(|_| failure())?.port(),
                token: token.clone(),
            };
            let mut stage = tempfile::NamedTempFile::new_in(&root).map_err(|_| failure())?;
            stage
                .write_all(&serde_json::to_vec(&e).map_err(|_| failure())?)
                .and_then(|_| stage.as_file().sync_all())
                .map_err(|_| failure())?;
            // Only this known, non-user-document runtime discovery file is rotated.
            stage.persist(&path).map_err(|_| failure())?;
            let running = Arc::new(AtomicBool::new(true));
            let flag = running.clone();
            let secret = token.clone();
            let last = self.last.clone();
            std::thread::spawn(move || {
                while flag.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((mut stream, address)) if address.ip().is_loopback() => {
                            let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                            let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
                            let reply = serve(&mut stream, &secret, &app, &last, &flag);
                            let _ = p::write_frame(&mut stream, &reply);
                        }
                        Ok(_) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(40))
                        }
                        Err(_) => break,
                    }
                }
                flag.store(false, Ordering::Release);
            });
            *guard = Some(Runtime {
                running,
                path,
                token,
            });
        }
        drop(guard);
        self.status()
    }
    pub fn stop(&self) {
        if let Ok(mut guard) = self.runtime.lock()
            && let Some(r) = guard.take()
        {
            r.running.store(false, Ordering::Release);
            if files::ordinary(&r.path).is_ok()
                && fs::metadata(&r.path).is_ok_and(|m| m.len() <= 2048)
                && fs::read(&r.path)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<Endpoint>(&b).ok())
                    .is_some_and(|e| e.token == r.token)
            {
                let _ = fs::remove_file(&r.path);
            }
        }
    }
}
fn serve(
    stream: &mut TcpStream,
    secret: &str,
    app: &AppHandle,
    last: &Arc<Mutex<Option<ClipSaved>>>,
    running: &AtomicBool,
) -> Reply {
    let e = match p::read_frame::<Envelope>(stream) {
        Ok(e) => e,
        Err(_) => return Reply::error("INVALID_MESSAGE", "消息无效、超限或超时。"),
    };
    if e.token.len() != 64
        || e.token
            .as_bytes()
            .iter()
            .zip(secret.as_bytes())
            .fold(0u8, |d, (a, b)| d | (a ^ b))
            != 0
        || !p::origin_valid(&e.origin)
    {
        return Reply::error("UNAUTHORIZED", "剪藏来源或会话授权无效。");
    }
    if !running.load(Ordering::Acquire) {
        return Reply::error("STOPPED", "网页剪藏连接已停止。");
    }
    if let Err(m) = e.request.validate() {
        return Reply::error("INVALID_MESSAGE", m);
    }
    let service = app.state::<Arc<LibraryService>>();
    match e.request.operation.as_str() {
        "ping" => Reply::success(serde_json::json!({"version":p::VERSION})),
        "projects" => match service.with(|s| s.projects()) {
            Ok(projects) => Reply::success(serde_json::json!({"projects":projects})),
            Err(e) => Reply::error(e.code, &e.message),
        },
        "save" => {
            let captures = app.state::<Arc<ScreenCaptureService>>();
            let _permit = match captures.try_begin() {
                Ok(p) => p,
                Err(_) => {
                    return Reply::error("BUSY", "桌面正在处理截图或文件，请稍后用同一请求重试。");
                }
            };
            match service.with(|s| s.save_web_clip(&e.request)) {
                Ok(saved) => {
                    if let Ok(mut slot) = last.lock() {
                        *slot = Some(saved.clone());
                    }
                    let _ = app.emit_to("main", "clipper-saved", &saved);
                    Reply::success(serde_json::json!(saved))
                }
                Err(e) => Reply::error(e.code, &e.message),
            }
        }
        _ => Reply::error("INVALID_MESSAGE", "不支持此操作。"),
    }
}
impl Drop for ClipperService {
    fn drop(&mut self) {
        self.stop();
    }
}
