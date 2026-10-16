use super::{CommandError, library::LibraryService};
use crate::{
    services::screen_capture::ScreenCaptureService,
    storage::markdown::{ExportReport, export_markdown, validate_ids},
};
use std::sync::Arc;
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn export_knowledge_markdown(
    window: WebviewWindow,
    app: AppHandle,
    library: State<'_, Arc<LibraryService>>,
    captures: State<'_, Arc<ScreenCaptureService>>,
    ids: Vec<String>,
) -> Result<Option<ExportReport>, CommandError> {
    if window.label() != "main" {
        return Err(CommandError {
            code: "WINDOW_DENIED",
            message: "该窗口无权导出知识卡片。".into(),
        });
    }
    validate_ids(&ids).map_err(super::library::error)?;
    let permit = captures.try_begin().map_err(|_| CommandError {
        code: "EXPORT_BUSY",
        message: "截图或文件操作尚未完成，请稍后导出。".into(),
    })?;
    let library = Arc::clone(library.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let Some(selected) = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("选择 Markdown 导出目录")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        let parent = selected.into_path().map_err(|_| CommandError {
            code: "EXPORT_DIRECTORY",
            message: "请选择本地文件夹。".into(),
        })?;
        let snapshot = library.with(|storage| storage.markdown_snapshot(&ids))?;
        export_markdown(&parent, snapshot)
            .map(Some)
            .map_err(super::library::error)
    })
    .await
    .map_err(|_| CommandError {
        code: "EXPORT_TASK",
        message: "Markdown导出任务异常结束，请重试。资料库未修改。".into(),
    })?
}
