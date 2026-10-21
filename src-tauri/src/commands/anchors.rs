use super::{
    CommandError,
    library::{LibraryService, run},
};
use crate::storage::anchors::{AnchorInput, AnchorPage, AnnotationAnchor};
use std::sync::Arc;
use tauri::{State, WebviewWindow};
#[tauri::command]
pub async fn annotation_anchors(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
) -> Result<AnchorPage, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.anchor_page(&id)
    })
    .await
}
#[tauri::command]
pub async fn resolve_annotation_anchor(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    screenshot_id: String,
    anchor_id: String,
) -> Result<AnnotationAnchor, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.resolve_anchor(&screenshot_id, &anchor_id)
    })
    .await
}
#[tauri::command]
pub async fn save_annotation_anchor(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    input: AnchorInput,
) -> Result<AnnotationAnchor, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.save_anchor(input)
    })
    .await
}
#[tauri::command]
pub async fn delete_annotation_anchor(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    screenshot_id: String,
    anchor_id: String,
    revision: i64,
) -> Result<(), CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.delete_anchor(&screenshot_id, &anchor_id, revision)
    })
    .await
}
