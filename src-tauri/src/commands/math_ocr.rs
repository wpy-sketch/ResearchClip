use crate::{
    capture::{CaptureError, ErrorCode, MAX_PNG_BYTES},
    services::{
        math_ocr::{MathJob, MathOcrService, MathStatus, prepare_input},
        screen_capture::ScreenCaptureService,
    },
};
use std::sync::Arc;
use tauri::{
    AppHandle, State, WebviewWindow,
    ipc::{InvokeBody, Request},
};
use tauri_plugin_dialog::DialogExt;
fn failure(message: &str) -> CaptureError {
    CaptureError::new(ErrorCode::CaptureFailed, message)
}
fn check(window: &WebviewWindow) -> Result<(), CaptureError> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(failure("该窗口无权访问公式识别。"))
    }
}
#[tauri::command]
pub fn math_ocr_status(
    window: WebviewWindow,
    service: State<'_, Arc<MathOcrService>>,
) -> Result<MathStatus, CaptureError> {
    check(&window)?;
    service.status()
}
#[tauri::command]
pub fn cancel_math_ocr(
    window: WebviewWindow,
    service: State<'_, Arc<MathOcrService>>,
) -> Result<(), CaptureError> {
    check(&window)?;
    service.cancel()
}
#[tauri::command]
pub fn download_math_models(
    window: WebviewWindow,
    service: State<'_, Arc<MathOcrService>>,
    confirmed: bool,
) -> Result<MathJob, CaptureError> {
    check(&window)?;
    if !confirmed {
        return Err(failure("请先明确确认模型来源与179MB下载体积。"));
    }
    service.inner().start("download", None)
}
#[tauri::command]
pub async fn choose_math_runtime(
    window: WebviewWindow,
    app: AppHandle,
    captures: State<'_, Arc<ScreenCaptureService>>,
    service: State<'_, Arc<MathOcrService>>,
) -> Result<MathStatus, CaptureError> {
    check(&window)?;
    let permit = captures.try_begin()?;
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        if let Some(path) = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("选择已安装公式推理依赖的python.exe")
            .add_filter("Python", &["exe"])
            .blocking_pick_file()
        {
            service.configure(
                path.into_path()
                    .map_err(|_| failure("请选择本地Python运行时。"))?,
            )?;
        }
        service.status()
    })
    .await
    .map_err(|_| failure("公式运行时设置任务异常结束。"))?
}
#[tauri::command]
pub async fn import_math_models(
    window: WebviewWindow,
    app: AppHandle,
    captures: State<'_, Arc<ScreenCaptureService>>,
    service: State<'_, Arc<MathOcrService>>,
) -> Result<MathStatus, CaptureError> {
    check(&window)?;
    let permit = captures.try_begin()?;
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        if let Some(path) = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("选择官方四份ONNX模型所在文件夹")
            .blocking_pick_folder()
        {
            service.import_models(
                &path
                    .into_path()
                    .map_err(|_| failure("请选择本地模型目录。"))?,
            )?;
        }
        service.status()
    })
    .await
    .map_err(|_| failure("模型导入任务异常结束，请检查状态后重试。"))?
}
#[tauri::command]
pub async fn recognize_math_ocr(
    window: WebviewWindow,
    service: State<'_, Arc<MathOcrService>>,
    request: Request<'_>,
) -> Result<MathJob, CaptureError> {
    check(&window)?;
    let bytes = match request.body() {
        InvokeBody::Raw(b) if b.len() <= MAX_PNG_BYTES + 1028 => b.clone(),
        _ => return Err(failure("公式图片数据无效或过大。")),
    };
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        service.start("recognize", Some(prepare_input(&bytes)?))
    })
    .await
    .map_err(|_| failure("公式任务启动失败。"))?
}
