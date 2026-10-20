use super::{
    CommandError,
    library::{LibraryService, check},
};
use crate::services::similarity_ai::{AiStatus, SimilarityAi};
use std::sync::Arc;
use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
#[tauri::command]
pub fn similarity_ai_status(
    window: WebviewWindow,
    service: State<'_, Arc<SimilarityAi>>,
) -> Result<AiStatus, CommandError> {
    check(&window)?;
    service.status()
}
#[tauri::command]
pub fn cancel_similarity_ai(
    window: WebviewWindow,
    service: State<'_, Arc<SimilarityAi>>,
) -> Result<AiStatus, CommandError> {
    check(&window)?;
    service.cancel();
    service.status()
}
#[tauri::command]
pub async fn choose_similarity_ai_bundle(
    window: WebviewWindow,
    service: State<'_, Arc<SimilarityAi>>,
) -> Result<AiStatus, CommandError> {
    check(&window)?;
    let app = window.app_handle().clone();
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        if service.busy() {
            return Err(CommandError {
                code: "AI_BUSY",
                message: "请先取消或等待AI任务完成。".into(),
            });
        }
        let Some(path) = app
            .dialog()
            .file()
            .set_title("选择完整离线包中的 similarity-ai 文件夹")
            .blocking_pick_folder()
        else {
            return service.status();
        };
        let path = path.into_path().map_err(|_| CommandError {
            code: "AI_DIRECTORY",
            message: "所选模型目录不可用。".into(),
        })?;
        service.select_bundle(path)
    })
    .await
    .map_err(|_| CommandError {
        code: "AI_TASK",
        message: "模型目录校验任务异常结束，原配置保留。".into(),
    })?
}
#[tauri::command]
pub async fn start_similarity_ai(
    window: WebviewWindow,
    store: State<'_, Arc<LibraryService>>,
    service: State<'_, Arc<SimilarityAi>>,
    a: String,
    b: String,
) -> Result<AiStatus, CommandError> {
    check(&window)?;
    if a == b {
        return Err(CommandError {
            code: "AI_INPUT",
            message: "请选择两张不同素材。".into(),
        });
    }
    let store = Arc::clone(store.inner());
    let service = Arc::clone(service.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let pair = [a.clone(), b.clone()];
        let (a, b) =
            store.with(|s| Ok((s.screenshot_image(&a, true)?, s.screenshot_image(&b, true)?)))?;
        service.start(a, b, pair)
    })
    .await
    .map_err(|_| CommandError {
        code: "AI_TASK",
        message: "AI任务启动失败，请重试；原图未修改。".into(),
    })?
}
