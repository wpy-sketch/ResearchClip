use super::{
    CommandError,
    library::{LibraryService, check, run},
};
use crate::{
    services::{markdown_document::DocumentFiles, screen_capture::ScreenCaptureService},
    storage::links::{GraphFilter, IndexedLink, LinkDetails, LinkGraph, LinkSource, TargetPage},
};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
#[tauri::command]
pub async fn link_targets(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    query: String,
    page: u32,
) -> Result<TargetPage, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.link_targets(&query, page)
    })
    .await
}
#[tauri::command]
pub async fn link_document_source(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
) -> Result<LinkSource, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.link_source(&id)
    })
    .await
}
#[tauri::command]
pub async fn replace_document_links(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    stamp: String,
    links: Vec<IndexedLink>,
) -> Result<(), CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.replace_links(&id, &stamp, &links)
    })
    .await
}
#[tauri::command]
pub async fn link_details(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    kind: String,
    id: String,
) -> Result<LinkDetails, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.link_details(&kind, &id)
    })
    .await
}
#[tauri::command]
pub async fn link_graph(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    filter: GraphFilter,
) -> Result<LinkGraph, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.link_graph(&filter)
    })
    .await
}
#[tauri::command]
pub async fn set_document_link_tags(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    tags: Vec<String>,
) -> Result<(), CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.set_document_link_tags(&id, &tags)
    })
    .await
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkExportReport {
    path: String,
    warnings: Vec<String>,
}
#[tauri::command]
pub async fn export_linked_document(
    window: WebviewWindow,
    app: AppHandle,
    service: State<'_, Arc<LibraryService>>,
    files: State<'_, Arc<Mutex<DocumentFiles>>>,
    captures: State<'_, Arc<ScreenCaptureService>>,
    token: String,
) -> Result<Option<LinkExportReport>, CommandError> {
    check(&window)?;
    let permit = captures.try_begin().map_err(|e| failure(&e.message))?;
    let service = Arc::clone(service.inner());
    let files = Arc::clone(files.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        // Validate the existing live grant before opening the bounded native save picker.
        let source = files
            .lock()
            .map_err(|_| failure("文档状态异常。"))?
            .external_path(&token)
            .map_err(|_| failure("文档授权失效或文件已被外部修改，请重新打开。"))?;
        let Some(selected) = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("导出标准Markdown副本（不覆盖已有文件）")
            .set_file_name("研究记录-导出.md")
            .add_filter("Markdown", &["md"])
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let destination = selected
            .into_path()
            .map_err(|_| failure("请选择本机Markdown文件。"))?;
        if !destination.is_absolute()
            || !destination
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("md"))
        {
            return Err(failure("请选择.md文件。"));
        }
        let (text, warnings) = service.with(|s| {
            let doc = s.index_document(&source, None)?;
            s.portable_links(&doc.id, &destination)
        })?;
        files
            .lock()
            .map_err(|_| failure("文档状态异常。"))?
            .export_link_copy(&token, &destination, text)
            .map_err(|e| failure(&e.message))?;
        Ok(Some(LinkExportReport {
            path: destination.to_string_lossy().into(),
            warnings,
        }))
    })
    .await
    .map_err(|_| failure("导出任务异常，原文档未修改。"))?
}
fn failure(message: &str) -> CommandError {
    CommandError {
        code: "LINK_EXPORT",
        message: message.into(),
    }
}
