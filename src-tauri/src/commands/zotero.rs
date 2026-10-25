use super::{
    CommandError,
    library::{LibraryService, check, run},
};
use crate::{
    services::{
        pdf::PdfDocumentService,
        zotero_local::{ZoteroClient, ZoteroScope},
        zotero_session::{PreviewResponse, ZoteroSessions},
    },
    storage::zotero::ZoteroSource,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{State, WebviewWindow};
pub(super) fn fail(message: impl Into<String>) -> CommandError {
    CommandError {
        code: "ZOTERO_READ_ERROR",
        message: message.into(),
    }
}
#[tauri::command]
pub async fn preview_pdf_zotero(
    window: WebviewWindow,
    pdf: State<'_, Arc<PdfDocumentService>>,
    sessions: State<'_, Arc<ZoteroSessions>>,
    token: String,
    lookup_id: String,
    scope: ZoteroScope,
    query: String,
) -> Result<PreviewResponse, CommandError> {
    check(&window)?;
    scope.validate_request().map_err(fail)?;
    let cancel = sessions.begin(&lookup_id, &token).map_err(fail)?;
    let pdf = Arc::clone(pdf.inner());
    let service = Arc::clone(sessions.inner());
    let jobs = service.clone();
    let id = lookup_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let (_file, _source, record) =
            pdf.verified_source(&token).map_err(super::library::error)?;
        if cancel.load(Ordering::Relaxed) {
            return Err(fail("Zotero查询已取消。"));
        }
        let (source, _) = pdf.source(&token).map_err(super::library::error)?;
        let result = ZoteroClient::new()
            .map_err(fail)?
            .lookup(&source, &scope, &query, &cancel)
            .map_err(fail)?;
        pdf.source(&token).map_err(super::library::error)?;
        jobs.store(
            &token,
            &record.document_id,
            &record.content_hash,
            result,
            &cancel,
        )
        .map_err(fail)
    })
    .await
    .map_err(|_| fail("Zotero只读查询任务异常结束。"));
    service.end(&id).map_err(fail)?;
    result?
}
#[tauri::command]
pub fn cancel_zotero_lookup(
    window: WebviewWindow,
    sessions: State<'_, Arc<ZoteroSessions>>,
    lookup_id: String,
) -> Result<(), CommandError> {
    check(&window)?;
    sessions.cancel(&lookup_id).map_err(fail)
}
#[tauri::command]
pub async fn confirm_pdf_zotero(
    window: WebviewWindow,
    pdf: State<'_, Arc<PdfDocumentService>>,
    sessions: State<'_, Arc<ZoteroSessions>>,
    library: State<'_, Arc<LibraryService>>,
    token: String,
    preview_id: String,
) -> Result<String, CommandError> {
    check(&window)?;
    let pdf = Arc::clone(pdf.inner());
    let sessions = Arc::clone(sessions.inner());
    let library = Arc::clone(library.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let (_file, source, record) = pdf.verified_source(&token).map_err(super::library::error)?;
        let candidate = sessions
            .take(
                &preview_id,
                &token,
                &record.document_id,
                &record.content_hash,
            )
            .map_err(fail)?;
        let cancel = AtomicBool::new(false);
        ZoteroClient::new()
            .map_err(fail)?
            .revalidate(&candidate, &source, &cancel)
            .map_err(fail)?;
        pdf.source(&token).map_err(super::library::error)?;
        library.with(|s| s.bind_pdf_zotero(&record.document_id, &record.content_hash, &candidate))
    })
    .await
    .map_err(|_| fail("来源关联任务异常结束，未修改原PDF。"))?
}
#[tauri::command]
pub async fn pdf_zotero_sources(
    window: WebviewWindow,
    library: State<'_, Arc<LibraryService>>,
    document_id: String,
) -> Result<Vec<ZoteroSource>, CommandError> {
    run(window, Arc::clone(library.inner()), move |s| {
        s.pdf_zotero_sources(&document_id)
    })
    .await
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoteroSetupInfo {
    executable_path: String,
    api_url: String,
    read_only: bool,
    api_key_required: bool,
}
#[tauri::command]
pub fn zotero_setup_info(window: WebviewWindow) -> Result<ZoteroSetupInfo, CommandError> {
    check(&window)?;
    let path =
        std::env::current_exe().map_err(|_| fail("无法读取当前程序位置，请手动选择安装目录。"))?;
    Ok(ZoteroSetupInfo {
        executable_path: path.to_string_lossy().into_owned(),
        api_url: crate::services::zotero_local::local_api_base(),
        read_only: true,
        api_key_required: false,
    })
}
#[tauri::command]
pub async fn check_zotero_connection(
    window: WebviewWindow,
    sessions: State<'_, Arc<ZoteroSessions>>,
    lookup_id: String,
) -> Result<crate::services::zotero_local::ZoteroHealth, CommandError> {
    check(&window)?;
    let cancel = sessions.begin(&lookup_id, "setup").map_err(fail)?;
    let service = Arc::clone(sessions.inner());
    let result = tauri::async_runtime::spawn_blocking(move || {
        ZoteroClient::new()
            .map_err(fail)?
            .health(&cancel, std::time::Instant::now())
            .map_err(fail)
    })
    .await
    .map_err(|_| fail("Zotero连接测试任务异常结束。"));
    service.end(&lookup_id).map_err(fail)?;
    result?
}
