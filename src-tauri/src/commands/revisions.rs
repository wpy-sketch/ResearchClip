use super::{
    CommandError,
    library::{LibraryService, run},
};
use crate::storage::{
    library::Screenshot,
    revisions::{AnnotationRevision, RevisionDocument},
};
use std::sync::Arc;
use tauri::{State, WebviewWindow};

#[tauri::command]
pub async fn annotation_history(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    before: i64,
    limit: u32,
) -> Result<Vec<AnnotationRevision>, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.annotation_history(&id, before, limit)
    })
    .await
}
#[tauri::command]
pub async fn annotation_revision(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    revision_id: String,
) -> Result<RevisionDocument, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.annotation_revision(&id, &revision_id)
    })
    .await
}
#[tauri::command]
pub async fn save_annotation_revision(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    expected: i64,
    document: serde_json::Value,
    description: String,
) -> Result<Screenshot, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.save_annotation_revision(&id, expected, document, &description)
    })
    .await
}
#[tauri::command]
pub async fn restore_annotation_revision(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    expected: i64,
    revision_id: String,
    description: String,
) -> Result<Screenshot, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.restore_annotation_revision(&id, expected, &revision_id, &description)
    })
    .await
}
