use super::{
    CommandError,
    library::{LibraryService, check, error},
    zotero::fail,
};
use crate::services::{
    pdf::PdfDocumentService,
    zotero_annotations::AnnotationSource,
    zotero_local::{ZoteroClient, ZoteroWriteError, ZoteroWriteOutcome},
    zotero_session::ZoteroSessions,
    zotero_write_model::{AnnotationWriteCapabilities, AnnotationWriteInput},
};
use crate::storage::StorageError;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, atomic::Ordering};
use tauri::{State, WebviewWindow};
#[derive(Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum WriteRequest {
    Capabilities,
    Authorize,
    Forget,
    Submit {
        input: AnnotationWriteInput,
        #[serde(default)]
        change_id: Option<String>,
        #[serde(default)]
        revision: Option<i64>,
    },
    Verify {
        #[serde(default)]
        change_id: Option<String>,
        #[serde(default)]
        revision: Option<i64>,
    },
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteResponse {
    capabilities: Option<AnnotationWriteCapabilities>,
    outcome: Option<ZoteroWriteOutcome>,
    pending: bool,
    sync: Option<crate::services::zotero_sync_model::AnnotationSyncSnapshot>,
}
fn write_error(e: ZoteroWriteError) -> CommandError {
    CommandError {
        code: e.code,
        message: e.message.into(),
    }
}
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn zotero_annotation_write(
    window: WebviewWindow,
    library: State<'_, Arc<LibraryService>>,
    pdf: State<'_, Arc<PdfDocumentService>>,
    sessions: State<'_, Arc<ZoteroSessions>>,
    token: String,
    source: AnnotationSource,
    lookup_id: String,
    request: WriteRequest,
) -> Result<WriteResponse, CommandError> {
    check(&window)?;
    source.validate().map_err(fail)?;
    let cancel = sessions.begin(&lookup_id, &token).map_err(fail)?;
    let pdf = Arc::clone(pdf.inner());
    let library = Arc::clone(library.inner());
    let owner = Arc::clone(sessions.inner());
    let service = owner.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let (_original, original, record) = pdf.verified_source(&token).map_err(error)?;
        library.with(|s| s.annotation_source(&record.document_id, &source))?;
        let client = ZoteroClient::new().map_err(fail)?;
        let mut response = WriteResponse {
            capabilities: None,
            outcome: None,
            pending: false,
            sync: None,
        };
        match request {
            WriteRequest::Capabilities => {
                let mut caps = client
                    .annotation_write_capabilities(&source, &original, &cancel)
                    .map_err(write_error)?;
                let (authorized, remembered, pending) = service
                    .write_authorized(&token, &source, &record.content_hash)
                    .map_err(fail)?;
                caps.authorized = authorized;
                caps.remembered = remembered;
                response.capabilities = Some(caps);
                response.pending = response.pending || pending;
            }
            WriteRequest::Authorize => {
                let grant = client
                    .authorize_annotation_writes(&source, &original, &cancel)
                    .map_err(write_error)?;
                // A user may close/revoke the PDF or replace its source during the dialog.
                let (_current, _, current) = pdf.verified_source(&token).map_err(error)?;
                if current.content_hash != record.content_hash {
                    return Err(fail("PDF版本已改变，未保存授权。"));
                }
                library.with(|s| s.annotation_source(&current.document_id, &source))?;
                service
                    .save_write_grant(&token, &source, &record.content_hash, grant, &cancel)
                    .map_err(fail)?;
            }
            WriteRequest::Forget => {
                service.forget_write_grant(&token).map_err(fail)?;
            }
            WriteRequest::Submit {
                input,
                change_id,
                revision,
            } => {
                let id = change_id
                    .ok_or_else(|| fail("写入必须先保存为稳定待处理记录，请重新打开同步面板。"))?;
                let revision =
                    revision.ok_or_else(|| fail("缺少本地待处理修订号，未执行写入。"))?;
                let entry =
                    library.with(|s| s.annotation_sync_entry(&record.document_id, &source, &id))?;
                if entry.local.as_ref() != Some(&input) {
                    return Err(super::library::error(StorageError::Conflict));
                }
                let (outcome, diagnostic) = super::zotero_sync::attempt_change(
                    &library, &pdf, &service, &client, &token, &source, &id, revision, &cancel,
                    false,
                )?;
                let snapshot =
                    library.with(|s| s.annotation_sync_snapshot(&record.document_id, &source))?;
                response.pending = snapshot.entries.iter().any(|e| e.needs_verification);
                response.outcome = outcome;
                response.sync = Some(snapshot);
                if let Some(d) = diagnostic {
                    return Err(CommandError {
                        code: d.code,
                        message: d.message.into(),
                    });
                }
            }
            WriteRequest::Verify {
                change_id,
                revision,
            } => {
                let id = change_id.ok_or_else(|| fail("复核必须指定已持久化的批注记录。"))?;
                let revision = revision.ok_or_else(|| fail("缺少本地待处理修订号。"))?;
                let (outcome, diagnostic) = super::zotero_sync::attempt_change(
                    &library, &pdf, &service, &client, &token, &source, &id, revision, &cancel,
                    true,
                )?;
                let snapshot =
                    library.with(|s| s.annotation_sync_snapshot(&record.document_id, &source))?;
                response.pending = snapshot.entries.iter().any(|e| e.needs_verification);
                response.outcome = outcome;
                response.sync = Some(snapshot);
                if let Some(d) = diagnostic {
                    return Err(CommandError {
                        code: d.code,
                        message: d.message.into(),
                    });
                }
            }
        }
        if !cancel.load(Ordering::Relaxed) && response.capabilities.is_none() {
            let mut caps = client
                .annotation_write_capabilities(&source, &original, &cancel)
                .ok();
            let (authorized, remembered, pending) = service
                .write_authorized(&token, &source, &record.content_hash)
                .map_err(fail)?;
            if let Some(caps) = caps.as_mut() {
                caps.authorized = authorized;
                caps.remembered = remembered;
            }
            response.capabilities = caps;
            response.pending = response.pending || pending;
        }
        Ok::<_, CommandError>(response)
    })
    .await
    .map_err(|_| fail("批注写入任务异常结束，请保留待处理内容并复核Zotero。"));
    owner.end(&lookup_id).map_err(fail)?;
    result?
}
