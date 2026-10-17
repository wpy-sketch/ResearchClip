use crate::services::templates::{TemplateFiles, TemplateInfo};
use crate::{
    capture::{CaptureError, ErrorCode},
    services::{
        markdown_document::{
            DocumentFiles, MAX_ASSETS, MAX_HEADER, SavedDocument, failure, normalized_name, unpack,
        },
        screen_capture::ScreenCaptureService,
        settings::SettingsService,
    },
};
use std::sync::{Arc, Mutex};
use tauri::{
    AppHandle, State, WebviewWindow,
    ipc::{InvokeBody, Request, Response},
};
use tauri_plugin_dialog::DialogExt;

async fn template_task<T: Send + 'static>(
    window: WebviewWindow,
    captures: &Arc<ScreenCaptureService>,
    templates: &Arc<Mutex<TemplateFiles>>,
    action: impl FnOnce(&TemplateFiles) -> Result<T, CaptureError> + Send + 'static,
) -> Result<T, CaptureError> {
    check(&window)?;
    let permit = captures.try_begin()?;
    let templates = Arc::clone(templates);
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let files = templates
            .lock()
            .map_err(|_| failure("模板状态异常，请保留草稿后重启。"))?;
        action(&files)
    })
    .await
    .map_err(|_| failure("模板任务异常结束，当前文档未修改。"))?
}
#[tauri::command]
pub async fn list_markdown_templates(
    window: WebviewWindow,
    captures: State<'_, Arc<ScreenCaptureService>>,
    templates: State<'_, Arc<Mutex<TemplateFiles>>>,
) -> Result<Vec<TemplateInfo>, CaptureError> {
    template_task(window, captures.inner(), templates.inner(), |t| t.list()).await
}
#[tauri::command]
pub async fn load_markdown_template(
    window: WebviewWindow,
    captures: State<'_, Arc<ScreenCaptureService>>,
    templates: State<'_, Arc<Mutex<TemplateFiles>>>,
    id: String,
) -> Result<Response, CaptureError> {
    template_task(window, captures.inner(), templates.inner(), move |t| {
        t.load(&id).map(Response::new)
    })
    .await
}
#[tauri::command]
pub async fn create_markdown_template(
    window: WebviewWindow,
    captures: State<'_, Arc<ScreenCaptureService>>,
    templates: State<'_, Arc<Mutex<TemplateFiles>>>,
    request: Request<'_>,
) -> Result<(), CaptureError> {
    check(&window)?;
    let bytes = match request.body() {
        InvokeBody::Raw(bytes) if bytes.len() <= 4 + MAX_HEADER + MAX_ASSETS => bytes.clone(),
        _ => return Err(failure("模板封包超过限制。")),
    };
    // ID is generated in the backend, without accepting any frontend file paths.
    template_task(window, captures.inner(), templates.inner(), move |t| {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| failure("无法分配模板 ID。"))?
            .as_nanos();
        let id = format!("{stamp:032x}");
        t.create(&id, &bytes)
    })
    .await
}
#[tauri::command]
pub async fn rename_markdown_template(
    window: WebviewWindow,
    captures: State<'_, Arc<ScreenCaptureService>>,
    templates: State<'_, Arc<Mutex<TemplateFiles>>>,
    id: String,
    name: String,
) -> Result<(), CaptureError> {
    template_task(window, captures.inner(), templates.inner(), move |t| {
        t.rename(&id, &name)
    })
    .await
}
#[tauri::command]
pub async fn delete_markdown_template(
    window: WebviewWindow,
    captures: State<'_, Arc<ScreenCaptureService>>,
    templates: State<'_, Arc<Mutex<TemplateFiles>>>,
    id: String,
) -> Result<(), CaptureError> {
    template_task(window, captures.inner(), templates.inner(), move |t| {
        t.delete(&id)
    })
    .await
}
fn check(window: &WebviewWindow) -> Result<(), CaptureError> {
    if window.label() != "main" {
        return Err(CaptureError::new(
            ErrorCode::WindowState,
            "此窗口不能操作Markdown文档。",
        ));
    }
    Ok(())
}
#[tauri::command]
pub async fn save_markdown_document(
    window: WebviewWindow,
    app: AppHandle,
    captures: State<'_, Arc<ScreenCaptureService>>,
    files: State<'_, Arc<Mutex<DocumentFiles>>>,
    settings: State<'_, Arc<SettingsService>>,
    request: Request<'_>,
) -> Result<Option<SavedDocument>, CaptureError> {
    check(&window)?;
    let permit = captures.try_begin()?;
    let bytes = match request.body() {
        InvokeBody::Raw(bytes) if bytes.len() <= 4 + MAX_HEADER + MAX_ASSETS => bytes.clone(),
        _ => return Err(failure("Markdown封包超过限制。")),
    };
    let files = Arc::clone(files.inner());
    let settings = Arc::clone(settings.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let (input, pngs) = unpack(&bytes)?;
        let selected = if input.save_as || input.token.is_none() {
            let directory = settings.available_directory()?;
            let mut dialog = app
                .dialog()
                .file()
                .set_parent(&window)
                .set_title("保存 Markdown 文档")
                .add_filter("Markdown", &["md"])
                .set_file_name(normalized_name(
                    input.file_name.as_deref().unwrap_or("untitled.md"),
                )?);
            if let Some(directory) = directory {
                dialog = dialog.set_directory(directory);
            }
            let Some(path) = dialog.blocking_save_file() else {
                return Ok(None);
            };
            Some(
                path.into_path()
                    .map_err(|_| failure("所选文件不是本地路径。"))?,
            )
        } else {
            None
        };
        let result = files
            .lock()
            .map_err(|_| failure("文档文件状态异常，请保留草稿后重启。"))?
            .save(selected.as_deref(), &input, &pngs)?;
        Ok(Some(result))
    })
    .await
    .map_err(|_| failure("Markdown保存任务异常结束，请保留草稿后重试。"))?
}
#[tauri::command]
pub async fn open_markdown_document(
    window: WebviewWindow,
    app: AppHandle,
    captures: State<'_, Arc<ScreenCaptureService>>,
    files: State<'_, Arc<Mutex<DocumentFiles>>>,
    settings: State<'_, Arc<SettingsService>>,
) -> Result<Response, CaptureError> {
    check(&window)?;
    let permit = captures.try_begin()?;
    let files = Arc::clone(files.inner());
    let directory = settings.available_directory()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let mut dialog = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("打开 Markdown 文档")
            .add_filter("Markdown", &["md"]);
        if let Some(directory) = directory {
            dialog = dialog.set_directory(directory);
        }
        let Some(path) = dialog.blocking_pick_file() else {
            return Ok(Response::new(Vec::<u8>::new()));
        };
        let path = path
            .into_path()
            .map_err(|_| failure("所选文件不是本地路径。"))?;
        let bytes = files
            .lock()
            .map_err(|_| failure("文档文件状态异常。"))?
            .open(&path)?;
        Ok(Response::new(bytes))
    })
    .await
    .map_err(|_| failure("Markdown打开任务异常结束，当前文档已保留。"))?
}
#[tauri::command]
pub fn release_markdown_document(
    window: WebviewWindow,
    files: State<'_, Arc<Mutex<DocumentFiles>>>,
    token: String,
) -> Result<(), CaptureError> {
    check(&window)?;
    if token.len() > 64 {
        return Err(failure("文件授权无效。"));
    }
    files
        .lock()
        .map_err(|_| failure("文档文件状态异常。"))?
        .release(&token);
    Ok(())
}

#[tauri::command]
pub async fn rename_markdown_document(
    window: WebviewWindow,
    captures: State<'_, Arc<ScreenCaptureService>>,
    files: State<'_, Arc<Mutex<DocumentFiles>>>,
    token: String,
    file_name: String,
) -> Result<SavedDocument, CaptureError> {
    check(&window)?;
    if token.len() > 64 {
        return Err(failure("文件授权无效。"));
    }
    let name = normalized_name(&file_name)?;
    let permit = captures.try_begin()?;
    let files = Arc::clone(files.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        files
            .lock()
            .map_err(|_| failure("文档状态异常。"))?
            .rename(&token, &name)
    })
    .await
    .map_err(|_| failure("重命名任务异常，请检查原文件并保留草稿。"))?
}
