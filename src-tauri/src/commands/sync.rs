use super::{
    CommandError,
    library::{LibraryService, check, error},
};
use crate::services::sync::{
    model::Config,
    runtime::{Status, SyncService},
};
use std::sync::Arc;
use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
#[tauri::command]
pub async fn sync_status(
    window: WebviewWindow,
    store: State<'_, Arc<LibraryService>>,
    service: State<'_, Arc<SyncService>>,
) -> Result<Status, CommandError> {
    check(&window)?;
    let store = Arc::clone(store.inner());
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || service.status(&store).map_err(error))
        .await
        .map_err(|_| CommandError {
            code: "SYNC_TASK",
            message: "读取同步状态失败。".into(),
        })?
}
#[tauri::command]
pub async fn sync_configure(
    window: WebviewWindow,
    store: State<'_, Arc<LibraryService>>,
    service: State<'_, Arc<SyncService>>,
    config: Config,
    username: Option<String>,
    password: Option<String>,
) -> Result<Status, CommandError> {
    check(&window)?;
    let store = Arc::clone(store.inner());
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        service
            .configure(&store, config, username, password)
            .map_err(error)?;
        service.status(&store).map_err(error)
    })
    .await
    .map_err(|_| CommandError {
        code: "SYNC_TASK",
        message: "同步配置失败，未重置资料。".into(),
    })?
}
#[tauri::command]
pub async fn sync_start(
    window: WebviewWindow,
    store: State<'_, Arc<LibraryService>>,
    service: State<'_, Arc<SyncService>>,
    test: bool,
) -> Result<(), CommandError> {
    check(&window)?;
    let store = Arc::clone(store.inner());
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        service
            .start(store, if test { "test" } else { "sync" }, None)
            .map_err(error)
    })
    .await
    .map_err(|_| CommandError {
        code: "SYNC_TASK",
        message: "启动同步失败，已有资料保留。".into(),
    })?
}
#[tauri::command]
pub fn sync_stop(
    window: WebviewWindow,
    service: State<'_, Arc<SyncService>>,
) -> Result<(), CommandError> {
    check(&window)?;
    service.stop();
    Ok(())
}
#[tauri::command]
pub async fn sync_resolve(
    window: WebviewWindow,
    store: State<'_, Arc<LibraryService>>,
    service: State<'_, Arc<SyncService>>,
    id: String,
    choice: String,
    confirmed: bool,
) -> Result<(), CommandError> {
    check(&window)?;
    if !confirmed {
        return Err(CommandError {
            code: "SYNC_CONFIRM",
            message: "请先明确确认该资源的冲突处理；远端删除不会自动执行。".into(),
        });
    }
    let store = Arc::clone(store.inner());
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        service
            .start(store, "resolve", Some((id, choice)))
            .map_err(error)
    })
    .await
    .map_err(|_| CommandError {
        code: "SYNC_TASK",
        message: "启动冲突处理失败，双方版本保留。".into(),
    })?
}
#[tauri::command]
pub async fn sync_authorize_documents(
    window: WebviewWindow,
    store: State<'_, Arc<LibraryService>>,
    service: State<'_, Arc<SyncService>>,
) -> Result<usize, CommandError> {
    check(&window)?;
    if service.busy() {
        return Err(CommandError {
            code: "SYNC_BUSY",
            message: "请先停止同步再授权文档。".into(),
        });
    }
    let store = Arc::clone(store.inner());
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = app
            .dialog()
            .file()
            .set_title("授权此目录下已登记的Markdown及受管assets只读同步")
            .blocking_pick_folder()
        else {
            return Ok(0);
        };
        let path = path.into_path().map_err(|_| CommandError {
            code: "SYNC_DIRECTORY",
            message: "选择的目录不可用。".into(),
        })?;
        store.with(|s| s.sync_authorize_documents(&path))
    })
    .await
    .map_err(|_| CommandError {
        code: "SYNC_TASK",
        message: "文档授权任务失败。".into(),
    })?
}
