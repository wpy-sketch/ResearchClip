use super::{
    CommandError,
    library::{LibraryService, check, run},
};
use crate::{
    services::{
        markdown_document::{DocumentFiles, failure},
        screen_capture::ScreenCaptureService,
    },
    storage::document_organization::{DocumentDirectory, MarkdownRevision},
};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State, WebviewWindow, ipc::Response};
use tauri_plugin_dialog::DialogExt;
#[tauri::command]
pub async fn document_directory(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
) -> Result<DocumentDirectory, CommandError> {
    run(window, Arc::clone(service.inner()), |s| {
        s.document_directory()
    })
    .await
}
#[tauri::command]
pub async fn create_document_folder(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    parent: Option<String>,
    name: String,
) -> Result<String, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.create_document_folder(parent.as_deref(), &name)
    })
    .await
}
#[tauri::command]
pub async fn delete_document_folder(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
) -> Result<(), CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.delete_document_folder(&id)
    })
    .await
}
#[tauri::command]
pub async fn organize_document(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    folder: Option<String>,
    favorite: bool,
) -> Result<(), CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.organize_document(&id, folder.as_deref(), favorite)
    })
    .await
}
#[tauri::command]
pub async fn markdown_history(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    before: i64,
) -> Result<Vec<MarkdownRevision>, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.markdown_history(&id, before)
    })
    .await
}
#[tauri::command]
pub async fn load_markdown_revision(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    files: State<'_, Arc<Mutex<DocumentFiles>>>,
    captures: State<'_, Arc<ScreenCaptureService>>,
    token: String,
    id: String,
    revision_id: String,
) -> Result<Response, crate::capture::CaptureError> {
    check(&window).map_err(|e| failure(&e.message))?;
    if token.len() > 64 {
        return Err(failure("文档授权无效。"));
    }
    let permit = captures.try_begin()?;
    let service = Arc::clone(service.inner());
    let files = Arc::clone(files.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let files = files.lock().map_err(|_| failure("文档授权状态异常。"))?;
        let path = files.external_path(&token)?;
        let revision = service
            .with(|s| {
                if s.document_location(&id)?.0 != path {
                    return Err(crate::storage::StorageError::Conflict);
                }
                s.markdown_revision(&id, &revision_id)
            })
            .map_err(|e| failure(&e.message))?;
        files
            .revision_packet(&token, &revision.markdown, &revision.assets)
            .map(Response::new)
    })
    .await
    .map_err(|_| failure("历史读取任务失败。"))?
}
#[tauri::command]
pub async fn relink_markdown_document(
    window: WebviewWindow,
    app: AppHandle,
    service: State<'_, Arc<LibraryService>>,
    files: State<'_, Arc<Mutex<DocumentFiles>>>,
    captures: State<'_, Arc<ScreenCaptureService>>,
    id: String,
) -> Result<bool, crate::capture::CaptureError> {
    check(&window).map_err(|e| failure(&e.message))?;
    let permit = captures.try_begin()?;
    let service = Arc::clone(service.inner());
    let files = Arc::clone(files.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let Some(selected) = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("定位已移动的 Markdown（请一并保留 assets）")
            .add_filter("Markdown", &["md"])
            .blocking_pick_file()
        else {
            return Ok(false);
        };
        let selected = selected
            .into_path()
            .map_err(|_| failure("必须选择本地文件。"))?;
        let files = files.lock().map_err(|_| failure("文档授权状态异常。"))?;
        service
            .with(|s| {
                let (path, hash) = s.document_location(&id)?;
                if files.path_is_open(&path) {
                    return Err(crate::storage::StorageError::Invalid(
                        "请先关闭原文档的全部标签页再重新定位。",
                    ));
                }
                s.relink_markdown_resource(&id, &hash, &selected)
            })
            .map_err(|e| failure(&e.message))?;
        Ok(true)
    })
    .await
    .map_err(|_| failure("重新定位失败，原登记保留。"))?
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionPreview {
    revision: MarkdownRevision,
    markdown: String,
}
#[tauri::command]
pub async fn markdown_revision_text(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    revision_id: String,
) -> Result<RevisionPreview, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        let r = s.markdown_revision(&id, &revision_id)?;
        Ok(RevisionPreview {
            revision: r.revision,
            markdown: r.markdown,
        })
    })
    .await
}
