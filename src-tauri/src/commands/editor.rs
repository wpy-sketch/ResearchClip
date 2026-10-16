use crate::{
    capture::{CaptureError, ErrorCode, MAX_PNG_BYTES},
    services::{
        editor::{MAX_DOCUMENT_BYTES, open_project, save_project, unpack_upload},
        screen_capture::ScreenCaptureService,
        settings::SettingsService,
        workflow::{decode_png, export_png_new},
    },
};
use std::sync::Arc;
use tauri::{
    AppHandle, State, WebviewWindow,
    ipc::{InvokeBody, Request, Response},
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;
fn check(window: &WebviewWindow) -> Result<(), CaptureError> {
    if window.label() != "main" {
        return Err(CaptureError::new(
            ErrorCode::WindowState,
            "该窗口无权操作编辑项目。",
        ));
    }
    Ok(())
}
fn raw(request: Request<'_>) -> Result<Vec<u8>, CaptureError> {
    match request.body() {
        InvokeBody::Raw(bytes) if bytes.len() <= 4 + MAX_DOCUMENT_BYTES + MAX_PNG_BYTES => {
            Ok(bytes.clone())
        }
        _ => Err(CaptureError::new(
            ErrorCode::SaveFailed,
            "编辑图片数据无效或超过大小上限。",
        )),
    }
}
fn task_error() -> CaptureError {
    CaptureError::new(ErrorCode::Internal, "编辑文件操作异常结束，请重试。")
}
#[tauri::command]
pub async fn save_editor_project(
    window: WebviewWindow,
    app: AppHandle,
    captures: State<'_, Arc<ScreenCaptureService>>,
    settings: State<'_, Arc<SettingsService>>,
    request: Request<'_>,
) -> Result<Option<String>, CaptureError> {
    check(&window)?;
    let permit = captures.try_begin()?;
    let bytes = raw(request)?;
    let directory = settings.available_directory()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let (doc, png) = unpack_upload(&bytes)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| task_error())?
            .as_nanos();
        let mut dialog = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("保存可编辑项目")
            .add_filter("ResearchClip 项目", &["json"])
            .set_file_name(format!("ResearchClip-{stamp}.researchclip.json"));
        if let Some(dir) = directory {
            dialog = dialog.set_directory(dir);
        }
        let Some(selected) = dialog.blocking_save_file() else {
            return Ok(None);
        };
        let path = selected.into_path().map_err(|_| task_error())?;
        save_project(&path, doc, png)?;
        Ok(Some(path.to_string_lossy().into()))
    })
    .await
    .map_err(|_| task_error())?
}
#[tauri::command]
pub async fn open_editor_project(
    window: WebviewWindow,
    app: AppHandle,
    captures: State<'_, Arc<ScreenCaptureService>>,
    settings: State<'_, Arc<SettingsService>>,
) -> Result<Response, CaptureError> {
    check(&window)?;
    let permit = captures.try_begin()?;
    let directory = settings.available_directory()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let mut dialog = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("打开可编辑项目")
            .add_filter("ResearchClip 项目", &["json"]);
        if let Some(dir) = directory {
            dialog = dialog.set_directory(dir);
        }
        let Some(selected) = dialog.blocking_pick_file() else {
            return Ok(Response::new(Vec::<u8>::new()));
        };
        let path = selected.into_path().map_err(|_| task_error())?;
        Ok(Response::new(open_project(&path)?))
    })
    .await
    .map_err(|_| task_error())?
}
#[tauri::command]
pub async fn export_editor_png(
    window: WebviewWindow,
    app: AppHandle,
    captures: State<'_, Arc<ScreenCaptureService>>,
    settings: State<'_, Arc<SettingsService>>,
    request: Request<'_>,
) -> Result<Option<String>, CaptureError> {
    check(&window)?;
    let permit = captures.try_begin()?;
    let bytes = raw(request)?;
    let directory = settings.available_directory()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let (_, png) = unpack_upload(&bytes)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| task_error())?
            .as_nanos();
        let mut dialog = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("导出标注 PNG")
            .add_filter("PNG 图片", &["png"])
            .set_file_name(format!("ResearchClip-export-{stamp}.png"));
        if let Some(dir) = directory {
            dialog = dialog.set_directory(dir);
        }
        let Some(selected) = dialog.blocking_save_file() else {
            return Ok(None);
        };
        let path = selected.into_path().map_err(|_| task_error())?;
        export_png_new(&path, png)?;
        Ok(Some(path.to_string_lossy().into()))
    })
    .await
    .map_err(|_| task_error())?
}
#[tauri::command]
pub async fn copy_editor_png(
    window: WebviewWindow,
    app: AppHandle,
    captures: State<'_, Arc<ScreenCaptureService>>,
    request: Request<'_>,
) -> Result<(), CaptureError> {
    check(&window)?;
    let permit = captures.try_begin()?;
    let bytes = raw(request)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let (_, png) = unpack_upload(&bytes)?;
        let frame = decode_png(png)?;
        app.clipboard()
            .write_image(&tauri::image::Image::new(
                &frame.rgba,
                frame.width_px,
                frame.height_px,
            ))
            .map_err(|_| {
                CaptureError::new(
                    ErrorCode::ClipboardUnavailable,
                    "无法写入合成图片剪贴板，可能正被其他应用占用，请稍后重试。",
                )
            })
    })
    .await
    .map_err(|_| task_error())?
}
