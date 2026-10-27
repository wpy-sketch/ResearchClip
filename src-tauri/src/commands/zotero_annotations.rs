use super::{
    CommandError,
    library::{LibraryService, check, run},
    zotero::fail,
};
use crate::{
    services::{
        pdf::PdfDocumentService,
        zotero_annotations::{AnnotationReadCache, AnnotationSource, SOURCE_MISMATCH},
        zotero_local::ZoteroClient,
        zotero_session::ZoteroSessions,
    },
    storage::StorageError,
};
use std::sync::{Arc, atomic::Ordering};
use tauri::{State, WebviewWindow};
#[tauri::command]
pub async fn cached_zotero_annotations(
    window: WebviewWindow,
    library: State<'_, Arc<LibraryService>>,
    pdf: State<'_, Arc<PdfDocumentService>>,
    token: String,
    source: AnnotationSource,
) -> Result<Option<AnnotationReadCache>, CommandError> {
    let pdf = Arc::clone(pdf.inner());
    run(window, Arc::clone(library.inner()), move |s| {
        let record = pdf.record(&token)?;
        s.annotation_cache(&record.document_id, &source)
    })
    .await
}
#[tauri::command]
pub async fn refresh_zotero_annotations(
    window: WebviewWindow,
    library: State<'_, Arc<LibraryService>>,
    pdf: State<'_, Arc<PdfDocumentService>>,
    sessions: State<'_, Arc<ZoteroSessions>>,
    token: String,
    source: AnnotationSource,
    lookup_id: String,
) -> Result<AnnotationReadCache, CommandError> {
    check(&window)?;
    source.validate().map_err(fail)?;
    let cancel = sessions.begin(&lookup_id, &token).map_err(fail)?;
    let pdf = Arc::clone(pdf.inner());
    let library = Arc::clone(library.inner());
    let owner = Arc::clone(sessions.inner());
    let id = lookup_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let (_original, original_path, record) =
            pdf.verified_source(&token).map_err(super::library::error)?;
        library.with(|s| s.annotation_source(&record.document_id, &source))?;
        let cache = ZoteroClient::new()
            .map_err(fail)?
            .read_annotations(&source, &original_path, &record.content_hash, &cancel)
            .map_err(|e| {
                if e == SOURCE_MISMATCH || e.starts_with("Zotero数据实例已经切换") {
                    CommandError {
                        code: "ZOTERO_ANNOTATION_SOURCE_MISMATCH",
                        message:
                            "Zotero实例、库或附件与当前来源不一致，未显示旧批注；请重新核对来源。"
                                .into(),
                    }
                } else {
                    fail(e)
                }
            })?;
        if cancel.load(Ordering::Relaxed) {
            return Err(fail("批注读取已取消，旧缓存保留。"));
        }
        pdf.source(&token).map_err(super::library::error)?;
        library.with(|s| {
            s.save_annotation_cache(&record.document_id, &cache)?;
            s.observe_annotation_sync(&record.document_id, &cache)?;
            Ok(())
        })?;
        Ok::<_, CommandError>(cache)
    })
    .await
    .map_err(|_| {
        super::library::error(StorageError::Invalid(
            "Zotero批注读取任务异常结束，缓存保留。",
        ))
    });
    owner.end(&id).map_err(fail)?;
    result?
}
