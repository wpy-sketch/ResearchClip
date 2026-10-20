use super::{
    CommandError,
    library::{LibraryService, check, run},
};
use crate::{
    services::similarity_index::{Progress, SimilarityIndex},
    storage::similarity::{FeatureStats, Groups, Matches, SearchScope},
};
use std::sync::Arc;
use tauri::{State, WebviewWindow};
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    stats: FeatureStats,
    job: Progress,
}
#[tauri::command]
pub async fn similarity_status(
    window: WebviewWindow,
    store: State<'_, Arc<LibraryService>>,
    jobs: State<'_, Arc<SimilarityIndex>>,
) -> Result<Status, CommandError> {
    let job = jobs.status()?;
    let stats = run(window, Arc::clone(store.inner()), |s| s.feature_stats()).await?;
    Ok(Status { stats, job })
}
#[tauri::command]
pub async fn start_similarity_index(
    window: WebviewWindow,
    store: State<'_, Arc<LibraryService>>,
    jobs: State<'_, Arc<SimilarityIndex>>,
    id: Option<String>,
    rebuild: bool,
) -> Result<Progress, CommandError> {
    check(&window)?;
    let library = Arc::clone(store.inner());
    let worker = Arc::clone(jobs.inner());
    tauri::async_runtime::spawn_blocking(move || worker.begin(library, id, rebuild))
        .await
        .map_err(|_| CommandError {
            code: "SIMILARITY_TASK",
            message: "无法启动相似图片索引，请重试。".into(),
        })?
}
#[tauri::command]
pub fn cancel_similarity_index(
    window: WebviewWindow,
    jobs: State<'_, Arc<SimilarityIndex>>,
) -> Result<Progress, CommandError> {
    check(&window)?;
    jobs.cancel();
    jobs.status()
}
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Preserve the existing bounded IPC contract; scope is additive.
pub async fn find_similar_images(
    window: WebviewWindow,
    store: State<'_, Arc<LibraryService>>,
    id: String,
    exact_only: bool,
    threshold: u32,
    color_limit: u32,
    page: u32,
    scope: Option<SearchScope>,
) -> Result<Matches, CommandError> {
    run(window, Arc::clone(store.inner()), move |s| {
        s.scoped_similar_images(
            &id,
            exact_only,
            threshold,
            color_limit,
            page,
            &scope.unwrap_or_default(),
        )
    })
    .await
}
#[tauri::command]
pub async fn duplicate_image_groups(
    window: WebviewWindow,
    store: State<'_, Arc<LibraryService>>,
    page: u32,
    scope: Option<SearchScope>,
) -> Result<Groups, CommandError> {
    run(window, Arc::clone(store.inner()), move |s| {
        s.scoped_duplicate_groups(page, &scope.unwrap_or_default())
    })
    .await
}
