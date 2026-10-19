use super::{
    CommandError,
    library::{LibraryService, check, run},
};
use crate::{
    services::ocr_index::{IndexProgress, OcrIndexService},
    storage::ocr::OcrContent,
};
use std::sync::Arc;
use tauri::{State, WebviewWindow};
#[tauri::command]
pub async fn get_ocr_content(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
) -> Result<Option<OcrContent>, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.ocr_content(&id)
    })
    .await
}
#[tauri::command]
pub async fn correct_ocr_content(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    revision: i64,
    text: String,
) -> Result<OcrContent, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.correct_ocr(&id, revision, &text)
    })
    .await
}
#[tauri::command]
pub async fn start_ocr_index(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    jobs: State<'_, Arc<OcrIndexService>>,
    language: String,
    id: Option<String>,
    failed_only: bool,
) -> Result<IndexProgress, CommandError> {
    check(&window)?;
    let library = Arc::clone(service.inner());
    let worker = Arc::clone(jobs.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let ids = library.with(|s| {
            if let Some(id) = id {
                s.get_screenshot(&id)?;
                Ok(vec![id])
            } else {
                s.pending_ocr(failed_only)
            }
        })?;
        worker.begin(library, ids, language)
    })
    .await
    .map_err(|_| CommandError {
        code: "OCR_TASK",
        message: "OCR任务启动失败，请重试。".into(),
    })?
}
#[tauri::command]
pub fn ocr_index_status(
    window: WebviewWindow,
    jobs: State<'_, Arc<OcrIndexService>>,
) -> Result<IndexProgress, CommandError> {
    check(&window)?;
    jobs.status()
}
#[tauri::command]
pub fn ocr_index_action(
    window: WebviewWindow,
    jobs: State<'_, Arc<OcrIndexService>>,
    action: String,
) -> Result<IndexProgress, CommandError> {
    check(&window)?;
    jobs.control(&action)
}
#[tauri::command]
pub async fn rebuild_search_index(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
) -> Result<(), CommandError> {
    run(window, Arc::clone(service.inner()), |s| s.rebuild_search()).await
}
