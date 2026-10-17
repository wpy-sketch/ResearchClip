use super::CommandError;
use crate::{
    capture::MAX_PNG_BYTES,
    services::editor::{MAX_DOCUMENT_BYTES, validate_document},
    storage::{
        Storage, StorageError,
        library::{
            DeleteResult, LibrarySort, LibraryStatus, Note, Screenshot, ScreenshotPage,
            ScreenshotPatch, ScreenshotSummary, ScreenshotTag, Tag,
        },
    },
};
use serde::Deserialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{
    State, WebviewWindow,
    ipc::{InvokeBody, Request, Response},
};

pub struct LibraryService {
    root: PathBuf,
    storage: Mutex<Option<Storage>>,
}
impl LibraryService {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            storage: Mutex::new(None),
        }
    }
    pub fn with<T>(
        &self,
        f: impl FnOnce(&mut Storage) -> Result<T, StorageError>,
    ) -> Result<T, CommandError> {
        let mut guard = self.storage.lock().map_err(|_| CommandError {
            code: "STORAGE_UNAVAILABLE",
            message: "资料服务锁异常，请保留未保存内容并重启。".into(),
        })?;
        if guard.is_none() {
            *guard = Some(Storage::open(self.root.clone()).map_err(error)?);
        }
        f(guard.as_mut().unwrap()).map_err(error)
    }
}
pub(super) fn error(e: StorageError) -> CommandError {
    let (code,message)=match e {
        StorageError::Invalid(m)=>("INVALID_DATA",m.into()),
        StorageError::NotFound=>("NOT_FOUND","资料不存在或已删除。".into()),
        StorageError::Conflict=>("REVISION_CONFLICT",e.to_string()),
        StorageError::NewerSchema(_)=>("FUTURE_SCHEMA",e.to_string()),
        StorageError::Io(_)=>("FILE_UNAVAILABLE","资料文件读写失败，请检查文件是否齐全、权限与剩余空间；已有文件未被覆盖。".into()),
        StorageError::Database(ref db) if db.sqlite_error_code()==Some(rusqlite::ErrorCode::ConstraintViolation)=>("DATA_CONSTRAINT","资料约束不满足，标签可能重名或关联资料不存在；现有数据未重置。".into()),
        StorageError::Database(_)=>("DATABASE_UNAVAILABLE","数据库操作失败，未重置资料。已落盘的未完成截图会保留以便恢复，请检查版本、权限与磁盘空间。".into()),
    };
    CommandError { code, message }
}
pub(super) fn check(window: &WebviewWindow) -> Result<(), CommandError> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(CommandError {
            code: "WINDOW_DENIED",
            message: "该窗口无权访问资料库。".into(),
        })
    }
}
pub(super) async fn run<T: Send + 'static>(
    window: WebviewWindow,
    service: Arc<LibraryService>,
    f: impl FnOnce(&mut Storage) -> Result<T, StorageError> + Send + 'static,
) -> Result<T, CommandError> {
    check(&window)?;
    tauri::async_runtime::spawn_blocking(move || service.with(f))
        .await
        .map_err(|_| CommandError {
            code: "STORAGE_TASK",
            message: "资料操作异常结束，请保留未保存内容后重试。".into(),
        })?
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Upload {
    title: String,
    document: serde_json::Value,
}
#[tauri::command]
pub async fn create_screenshot(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    request: Request<'_>,
) -> Result<Screenshot, CommandError> {
    check(&window)?;
    let bytes = match request.body() {
        InvokeBody::Raw(b) if b.len() <= 4 + MAX_DOCUMENT_BYTES + MAX_PNG_BYTES => b.clone(),
        _ => return Err(error(StorageError::Invalid("截图上传数据无效。"))),
    };
    run(window, Arc::clone(service.inner()), move |s| {
        if bytes.len() < 4 {
            return Err(StorageError::Invalid("截图上传数据不完整。"));
        }
        let n = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
        if n == 0 || n > MAX_DOCUMENT_BYTES || 4 + n >= bytes.len() {
            return Err(StorageError::Invalid("截图上传数据不完整。"));
        }
        let upload: Upload = serde_json::from_slice(&bytes[4..4 + n])
            .map_err(|_| StorageError::Invalid("截图JSON无效。"))?;
        validate_document(&upload.document)
            .map_err(|_| StorageError::Invalid("标注数据或版本无效。"))?;
        s.create_screenshot(&upload.title, upload.document, &bytes[4 + n..])
    })
    .await
}
#[tauri::command]
pub async fn library_status(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
) -> Result<LibraryStatus, CommandError> {
    run(window, Arc::clone(service.inner()), |s| s.library_status()).await
}
#[tauri::command]
pub async fn reconcile_library(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
) -> Result<LibraryStatus, CommandError> {
    run(window, Arc::clone(service.inner()), |s| s.reconcile()).await
}
#[tauri::command]
pub async fn list_screenshots(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    offset: u32,
    limit: u32,
    favorites: bool,
) -> Result<Vec<ScreenshotSummary>, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.list_screenshots(offset, limit, favorites)
    })
    .await
}
#[tauri::command]
pub async fn screenshot_page(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    page: u32,
    limit: u32,
    favorites: bool,
    sort: LibrarySort,
    filter: Option<crate::storage::search::LibraryFilter>,
) -> Result<ScreenshotPage, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| match filter {
        Some(filter) => s.search_screenshot_page(page, limit, favorites, sort, &filter),
        None => s.screenshot_page(page, limit, favorites, sort),
    })
    .await
}
#[tauri::command]
pub async fn get_screenshot(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
) -> Result<Screenshot, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.get_screenshot(&id)
    })
    .await
}
#[tauri::command]
pub async fn get_knowledge_card(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
) -> Result<crate::storage::knowledge::KnowledgeCard, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.knowledge_card(&id)
    })
    .await
}
#[tauri::command]
pub async fn save_knowledge_card(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    input: crate::storage::knowledge::CardInput,
) -> Result<crate::storage::knowledge::KnowledgeCard, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.save_knowledge_card(input)
    })
    .await
}
#[tauri::command]
pub async fn open_card_source(
    window: WebviewWindow,
    app: tauri::AppHandle,
    service: State<'_, Arc<LibraryService>>,
    id: String,
) -> Result<(), CommandError> {
    use tauri_plugin_opener::OpenerExt;
    let url = run(window, Arc::clone(service.inner()), move |s| {
        let card = s.knowledge_card(&id)?;
        crate::storage::knowledge::source_url(
            card.note.as_ref().and_then(|n| n.source_url.as_deref()),
        )?
        .ok_or(StorageError::Invalid("该卡片还没有已保存的来源链接。"))
    })
    .await?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|_| CommandError {
            code: "LINK_UNAVAILABLE",
            message: "无法打开默认浏览器，请检查系统浏览器设置。".into(),
        })
}
#[tauri::command]
pub async fn screenshot_image(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    thumbnail: bool,
) -> Result<Response, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        Ok(Response::new(s.screenshot_image(&id, thumbnail)?))
    })
    .await
}
#[tauri::command]
pub async fn update_screenshot(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    revision: i64,
    patch: ScreenshotPatch,
) -> Result<Screenshot, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.update_screenshot(&id, revision, patch)
    })
    .await
}
#[tauri::command]
pub async fn delete_screenshot(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    revision: i64,
) -> Result<DeleteResult, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.delete_screenshot(&id, revision)
    })
    .await
}
#[tauri::command]
pub async fn list_notes(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    screenshot_id: String,
) -> Result<Vec<Note>, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.list_notes(&screenshot_id)
    })
    .await
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoteInput {
    screenshot_id: String,
    id: Option<String>,
    revision: Option<i64>,
    content: String,
    source_url: Option<String>,
}
#[tauri::command]
pub async fn save_note(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    input: NoteInput,
) -> Result<Note, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.save_note(
            &input.screenshot_id,
            input.id.as_deref(),
            input.revision,
            &input.content,
            input.source_url.as_deref(),
        )
    })
    .await
}
#[tauri::command]
pub async fn delete_note(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    revision: i64,
) -> Result<(), CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.delete_note(&id, revision)
    })
    .await
}
#[tauri::command]
pub async fn list_tags(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
) -> Result<Vec<Tag>, CommandError> {
    run(window, Arc::clone(service.inner()), |s| s.list_tags()).await
}
#[tauri::command]
pub async fn save_tag(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: Option<String>,
    name: String,
) -> Result<Tag, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.save_tag(id.as_deref(), &name)
    })
    .await
}
#[tauri::command]
pub async fn delete_tag(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
) -> Result<(), CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.delete_tag(&id)
    })
    .await
}
#[tauri::command]
pub async fn get_screenshot_tags(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
) -> Result<Vec<ScreenshotTag>, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.screenshot_tags(&id)
    })
    .await
}
#[tauri::command]
pub async fn set_screenshot_tags(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    id: String,
    revision: i64,
    tag_ids: Vec<String>,
) -> Result<Screenshot, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.set_screenshot_tags(&id, revision, &tag_ids)
    })
    .await
}
