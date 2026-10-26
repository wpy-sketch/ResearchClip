use super::{
    CommandError,
    library::{LibraryService, check, error, run},
};
use crate::{
    domain::research::{ResearchData, ResourceKey, ResourceKind},
    services::pdf::PdfDocumentService,
    storage::{
        StorageError,
        pdf_excerpts::{ExcerptDraft, PdfExcerpt},
    },
};
use serde::Deserialize;
use std::sync::Arc;
use tauri::{
    State, WebviewWindow,
    ipc::{InvokeBody, Request},
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Upload {
    token: String,
    title: String,
    draft: ExcerptDraft,
}
#[tauri::command]
pub async fn save_pdf_excerpt(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    pdf: State<'_, Arc<PdfDocumentService>>,
    request: Request<'_>,
) -> Result<PdfExcerpt, CommandError> {
    check(&window)?;
    let bytes = match request.body() {
        InvokeBody::Raw(b) if b.len() <= 64 * 1024 * 1024 + 16388 => b.clone(),
        _ => return Err(error(StorageError::Invalid("PDF摘录上传超过预算。"))),
    };
    if bytes.len() < 4 {
        return Err(error(StorageError::Invalid("摘录封包不完整。")));
    }
    let n = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
    if n == 0 || n > 16384 || 4 + n >= bytes.len() {
        return Err(error(StorageError::Invalid("摘录封包无效。")));
    }
    let upload: Upload = serde_json::from_slice(&bytes[4..4 + n])
        .map_err(|_| error(StorageError::Invalid("摘录参数或版本无效。")))?;
    upload.draft.validate().map_err(error)?;
    let native = Arc::clone(pdf.inner());
    run(window, Arc::clone(service.inner()), move |s| {
        let record = native.record(&upload.token)?;
        if record.attachment_id != upload.draft.location.attachment_id
            || record.content_hash != upload.draft.location.content_hash
        {
            return Err(StorageError::Conflict);
        }
        s.create_pdf_excerpt(&upload.title, &upload.draft, &bytes[4 + n..])
    })
    .await
}
#[tauri::command]
pub async fn pdf_excerpts(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    document_id: String,
) -> Result<Vec<PdfExcerpt>, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.pdf_excerpts(&document_id)
    })
    .await
}
#[tauri::command]
pub async fn get_pdf_excerpt(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    screenshot_id: String,
) -> Result<Option<PdfExcerpt>, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.pdf_excerpt(&screenshot_id)
    })
    .await
}
#[tauri::command]
pub async fn resolve_pdf_location(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    location_id: String,
) -> Result<serde_json::Value, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        let row = s.research_resource(&ResourceKey {
            kind: ResourceKind::PdfLocation,
            id: location_id,
        })?;
        let ResearchData::PdfLocation(location) = row.data()? else {
            return Err(StorageError::Invalid("资源不是PDF位置。"));
        };
        let a = s.research_resource(&ResourceKey {
            kind: ResourceKind::PdfAttachment,
            id: location.attachment_id.clone(),
        })?;
        let ResearchData::PdfAttachment(attachment) = a.data()? else {
            return Err(StorageError::Invalid("位置附件无效。"));
        };
        let record = s.pdf_record(&attachment.document_id)?;
        if record.content_hash != location.content_hash {
            return Err(StorageError::Conflict);
        }
        PdfDocumentService::verify_managed(&s.pdf_file_path(&record.document_id)?,&record)?;
        Ok(serde_json::json!({"documentId":attachment.document_id,"title":record.title,"zotero":s.pdf_zotero_sources(&record.document_id)?,"location":location}))
    })
    .await
}

#[tauri::command]
pub async fn create_pdf_text_location(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    pdf: State<'_, Arc<PdfDocumentService>>,
    token: String,
    location: crate::domain::research::PdfLocation,
) -> Result<String, CommandError> {
    let native = Arc::clone(pdf.inner());
    run(window, Arc::clone(service.inner()), move |s| {
        let record = native.record(&token)?;
        if record.attachment_id != location.attachment_id
            || record.content_hash != location.content_hash
        {
            return Err(StorageError::Conflict);
        }
        PdfDocumentService::verify_managed(&s.pdf_file_path(&record.document_id)?, &record)?;
        s.create_pdf_text_location(&location)
    })
    .await
}

#[tauri::command]
pub async fn pdf_selection_notes(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    pdf: State<'_, Arc<PdfDocumentService>>,
    token: String,
    location: crate::domain::research::PdfLocation,
) -> Result<crate::storage::pdf_notes::PdfNoteResults, CommandError> {
    let native = Arc::clone(pdf.inner());
    run(window, Arc::clone(service.inner()), move |s| {
        let record = native.record(&token)?;
        if record.attachment_id != location.attachment_id
            || record.content_hash != location.content_hash
        {
            return Err(StorageError::Conflict);
        }
        s.pdf_note_results(&location)
    })
    .await
}
