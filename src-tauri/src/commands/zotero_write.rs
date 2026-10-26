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
use serde::{Deserialize, Serialize};
use std::sync::{Arc, atomic::Ordering};
use tauri::{State, WebviewWindow};
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
pub enum WriteRequest {
    Capabilities,
    Authorize,
    Forget,
    Submit { input: AnnotationWriteInput },
    Verify,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteResponse {
    capabilities: Option<AnnotationWriteCapabilities>,
    outcome: Option<ZoteroWriteOutcome>,
    pending: bool,
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
                response.pending = pending;
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
            WriteRequest::Submit { input } => {
                input.validate().map_err(fail)?;
                let intent = client
                    .prepare_annotation_write(&source, &original, &input, &cancel)
                    .map_err(write_error)?;
                let grant = service
                    .start_write(&token, &record.content_hash, intent.clone(), &cancel)
                    .map_err(fail)?;
                match client.perform_annotation_write(&intent, &grant, &original, &cancel) {
                    Ok(outcome) => {
                        if outcome.verified {
                            service
                                .finish_write(&token, intent.annotation_key())
                                .map_err(fail)?;
                        }
                        response.pending = !outcome.verified;
                        response.outcome = Some(outcome);
                    }
                    Err(e) => {
                        // These failures prove no mutation was accepted. All ambiguous transport
                        // failures preserve the intent for read-only verification, never resend.
                        if matches!(
                            e.code,
                            "ZOTERO_WRITE_AUTH_REQUIRED"
                                | "ZOTERO_WRITE_FORBIDDEN"
                                | "ZOTERO_WRITE_CONFLICT"
                                | "ZOTERO_WRITE_PRECONDITION"
                                | "ZOTERO_WRITE_RATE_LIMIT"
                                | "ZOTERO_WRITE_INVALID"
                                | "ZOTERO_WRITE_CANCELLED"
                                | "ZOTERO_WRITE_UNSUPPORTED"
                                | "ZOTERO_WRITE_UNAVAILABLE"
                        ) {
                            service
                                .finish_write(&token, intent.annotation_key())
                                .map_err(fail)?;
                        }
                        if matches!(
                            e.code,
                            "ZOTERO_WRITE_AUTH_REQUIRED" | "ZOTERO_WRITE_SOURCE_MISMATCH"
                        ) {
                            service.forget_write_grant(&token).map_err(fail)?;
                        }
                        return Err(write_error(e));
                    }
                }
            }
            WriteRequest::Verify => {
                let intent = service
                    .pending_write(&token, &source, &record.content_hash)
                    .map_err(fail)?;
                let outcome = client
                    .verify_annotation_write(&intent, &original, &cancel)
                    .map_err(write_error)?;
                if outcome.verified {
                    service
                        .finish_write(&token, intent.annotation_key())
                        .map_err(fail)?;
                }
                response.pending = !outcome.verified;
                response.outcome = Some(outcome);
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
            response.pending = pending;
        }
        Ok::<_, CommandError>(response)
    })
    .await
    .map_err(|_| fail("批注写入任务异常结束，请保留待处理内容并复核Zotero。"));
    owner.end(&lookup_id).map_err(fail)?;
    result?
}
