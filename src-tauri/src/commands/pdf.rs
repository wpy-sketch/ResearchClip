use super::{
    CommandError,
    library::{LibraryService, check, error, run},
};
use crate::{
    services::pdf::{PdfDocumentService, PdfProgress, import_copy, verified_copy},
    storage::{
        StorageError,
        pdf::{PdfReadingLocation, PdfRecord},
    },
};
use std::sync::Arc;
use tauri::{AppHandle, State, WebviewWindow, ipc::Response};
use tauri_plugin_dialog::DialogExt;
pub fn receive_external(app: &AppHandle, args: &[String], cwd: &std::path::Path) {
    use tauri::{Emitter, Manager};
    let result = crate::services::pdf_open::pdf_arguments(args, cwd).and_then(|paths| {
        app.state::<Arc<PdfDocumentService>>()
            .requests
            .enqueue(paths)
    });
    match result {
        Ok(count) if count > 0 => {
            if app.emit_to("main", "pdf-open-requested", ()).is_err() {
                crate::services::desktop::notice(app, "外部PDF已入队，请在工作区重试接收。");
            }
        }
        Ok(_) => {}
        Err(e) => crate::services::desktop::notice(app, &error(e).message),
    }
}
#[tauri::command]
pub fn pending_pdf_requests(
    window: WebviewWindow,
    pdf: State<'_, Arc<PdfDocumentService>>,
) -> Result<Vec<crate::services::pdf_open::PdfOpenOffer>, CommandError> {
    check(&window)?;
    pdf.requests.list().map_err(error)
}
#[tauri::command]
pub fn dismiss_pdf_request(
    window: WebviewWindow,
    pdf: State<'_, Arc<PdfDocumentService>>,
    id: String,
) -> Result<(), CommandError> {
    check(&window)?;
    pdf.requests.dismiss(&id).map_err(error)
}
#[tauri::command]
pub async fn open_pdf_document(
    window: WebviewWindow,
    app: AppHandle,
    library: State<'_, Arc<LibraryService>>,
    pdf: State<'_, Arc<PdfDocumentService>>,
    token: String,
    document_id: Option<String>,
    request_id: Option<String>,
) -> Result<Option<PdfRecord>, CommandError> {
    check(&window)?;
    if document_id.is_some() && request_id.is_some() {
        return Err(error(StorageError::Invalid(
            "PDF打开来源不能同时指定两种。",
        )));
    }
    let job = pdf.begin(&token).map_err(error)?;
    let service = Arc::clone(pdf.inner());
    let library = Arc::clone(library.inner());

    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| {
            let (record, source) = if let Some(id) = document_id {
                (library.with(|s| s.pdf_record(&id))?, None)
            } else {
                let path = if let Some(id) = request_id {
                    service.requests.take(&id).map_err(error)?
                } else {
                    let Some(selected) = app
                        .dialog()
                        .file()
                        .set_parent(&window)
                        .set_title("打开PDF（导入只读工作区副本）")
                        .add_filter("PDF", &["pdf"])
                        .blocking_pick_file()
                    else {
                        return Ok(None);
                    };
                    selected
                        .into_path()
                        .map_err(|_| error(StorageError::Invalid("请选择本地PDF。")))?
                };
                job.check().map_err(error)?;
                let title = path
                    .file_name()
                    .and_then(|p| p.to_str())
                    .unwrap_or("PDF")
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(160)
                    .collect::<String>();
                let root = library.with(|s| s.pdf_directory())?;
                let (hash, size) = import_copy(&path, &root, &job).map_err(error)?;
                job.check().map_err(error)?;
                (
                    library.with(|s| s.register_pdf(&title, &hash, size))?,
                    Some(path),
                )
            };
            job.check().map_err(error)?;
            let managed = library.with(|s| s.pdf_file_path(&record.document_id))?;
            let file = verified_copy(&managed, &record.content_hash, record.byte_size, &job)
                .map_err(error)?;
            service
                .finish(&token, file, record.clone(), source)
                .map_err(error)?;
            Ok(Some(record))
        })();
        if !matches!(result, Ok(Some(_))) {
            service.cancel(&token).map_err(error)?;
        }
        result
    })
    .await
    .map_err(|_| error(StorageError::Invalid("PDF读取任务异常结束。")))?
}
#[tauri::command]
pub fn pdf_load_progress(
    window: WebviewWindow,
    pdf: State<'_, Arc<PdfDocumentService>>,
    token: String,
) -> Result<PdfProgress, CommandError> {
    check(&window)?;
    pdf.progress(&token).map_err(error)
}
#[tauri::command]
pub fn close_pdf_document(
    window: WebviewWindow,
    sessions: State<'_, Arc<crate::services::zotero_session::ZoteroSessions>>,
    pdf: State<'_, Arc<PdfDocumentService>>,
    token: String,
) -> Result<(), CommandError> {
    check(&window)?;
    let meta = sessions.cancel_pdf(&token).map_err(super::zotero::fail);
    let file = pdf.cancel(&token).map_err(error);
    meta.and(file)
}
#[tauri::command]
pub async fn read_pdf_chunk(
    window: WebviewWindow,
    pdf: State<'_, Arc<PdfDocumentService>>,
    token: String,
    offset: u64,
    length: u32,
) -> Result<Response, CommandError> {
    check(&window)?;
    let service = Arc::clone(pdf.inner());
    tauri::async_runtime::spawn_blocking(move || {
        service
            .read(&token, offset, length)
            .map(Response::new)
            .map_err(error)
    })
    .await
    .map_err(|_| error(StorageError::Invalid("PDF分块任务失败。")))?
}
#[tauri::command]
pub async fn recent_pdf_documents(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
) -> Result<Vec<PdfRecord>, CommandError> {
    run(window, Arc::clone(service.inner()), |s| s.recent_pdfs()).await
}
#[tauri::command]
pub async fn save_pdf_reading(
    window: WebviewWindow,
    service: State<'_, Arc<LibraryService>>,
    document_id: String,
    content_hash: String,
    expected: i64,
    location: PdfReadingLocation,
) -> Result<i64, CommandError> {
    run(window, Arc::clone(service.inner()), move |s| {
        s.save_pdf_reading(&document_id, &content_hash, expected, &location)
    })
    .await
}

#[tauri::command]
pub fn adopt_pdf_source(
    window: WebviewWindow,
    pdf: State<'_, Arc<PdfDocumentService>>,
    from: String,
    to: String,
) -> Result<(), CommandError> {
    check(&window)?;
    pdf.adopt_source(&from, &to).map_err(error)
}
