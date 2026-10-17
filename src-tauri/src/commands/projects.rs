use super::{
    CommandError,
    library::{LibraryService, check, run},
};
use crate::{
    capture::CaptureError,
    services::{markdown_document::DocumentFiles, screen_capture::ScreenCaptureService},
    storage::{
        library::LibrarySort,
        projects::{BatchAction, BatchResult, Project, ProjectDocument, SelectedScreenshot},
        search::LibraryFilter,
    },
};
use std::sync::{Arc, Mutex};
use tauri::{State, WebviewWindow, ipc::Response};
#[tauri::command]
pub async fn list_projects(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
) -> Result<Vec<Project>, CommandError> {
    run(window, Arc::clone(service.inner()), |s| s.projects()).await
}
#[tauri::command]
pub async fn save_project(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: Option<String>,
    name: String,
) -> Result<(), CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.save_project(id.as_deref(), &name)
    })
    .await
}
#[tauri::command]
pub async fn delete_empty_project(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
) -> Result<(), CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.delete_empty_project(&id)
    })
    .await
}
#[tauri::command]
pub async fn select_library_screenshots(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    favorites: bool,
    sort: LibrarySort,
    filter: LibraryFilter,
) -> Result<Vec<SelectedScreenshot>, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.select_screenshots(favorites, sort, &filter)
    })
    .await
}
#[tauri::command]
pub async fn batch_library_screenshots(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    items: Vec<SelectedScreenshot>,
    action: BatchAction,
) -> Result<BatchResult, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.batch_screenshots(&items, action)
    })
    .await
}
#[tauri::command]
pub async fn list_project_documents(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    project_id: Option<String>,
    unclassified: bool,
) -> Result<Vec<ProjectDocument>, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.project_documents(project_id.as_deref(), unclassified)
    })
    .await
}
#[tauri::command]
pub async fn assign_project_document(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    project_id: Option<String>,
) -> Result<(), CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.assign_document(&id, project_id.as_deref())
    })
    .await
}
#[tauri::command]
pub async fn sync_project_document(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    files: State<'_, Arc<Mutex<DocumentFiles>>>,
    token: String,
) -> Result<ProjectDocument, CommandError> {
    check(&window)?;
    let service = Arc::clone(service.inner());
    let files = Arc::clone(files.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let mut files = files.lock().map_err(|_| CommandError {
            code: "DOCUMENT_STATE",
            message: "文档授权状态异常。".into(),
        })?;
        let (path, old) = files.index_paths(&token).map_err(|e| CommandError {
            code: "DOCUMENT_GRANT",
            message: e.message,
        })?;
        let result = service.with(|s| s.index_document(&path, old.as_deref()))?;
        files.mark_indexed(&token, path);
        Ok(result)
    })
    .await
    .map_err(|_| CommandError {
        code: "PROJECT_TASK",
        message: "文档已保存，但项目索引任务异常；请重试登记。".into(),
    })?
}
#[tauri::command]
pub async fn open_project_document(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    files: State<'_, Arc<Mutex<DocumentFiles>>>,
    captures: State<'_, Arc<ScreenCaptureService>>,
    id: String,
) -> Result<Response, CaptureError> {
    check(&window).map_err(|e| crate::services::markdown_document::failure(&e.message))?;
    let permit = captures.try_begin()?;
    let service = Arc::clone(service.inner());
    let files = Arc::clone(files.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let path = service
            .with(|s| s.indexed_document_path(&id))
            .map_err(|e| crate::services::markdown_document::failure(&e.message))?;
        files
            .lock()
            .map_err(|_| crate::services::markdown_document::failure("文档状态异常。"))?
            .open(&path)
            .map(Response::new)
    })
    .await
    .map_err(|_| crate::services::markdown_document::failure("打开项目文档失败，原文件未改动。"))?
}
