//! Bounded native sync orchestration; data locks are released during HTTP operations.
use super::{
    CommandError,
    library::{LibraryService, check, error},
    zotero::fail,
};
use crate::{
    services::{
        pdf::PdfDocumentService,
        zotero_annotations::{AnnotationReadCache, AnnotationSource},
        zotero_local::{ZoteroClient, ZoteroWriteError, ZoteroWriteOutcome},
        zotero_session::ZoteroSessions,
        zotero_sync_model::{AnnotationSyncEntry, AnnotationSyncSnapshot, AnnotationSyncState},
        zotero_write_model::AnnotationWriteInput,
    },
    storage::revision_meta::RevisionMetadata,
};
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{State, WebviewWindow};
#[derive(Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum AnnotationSyncRequest {
    List,
    Stage {
        change_id: Option<String>,
        revision: Option<i64>,
        input: AnnotationWriteInput,
    },
    Refresh,
    Submit {
        change_id: String,
        revision: i64,
    },
    Verify {
        change_id: String,
        revision: i64,
    },
    Resolve {
        change_id: String,
        revision: i64,
        remote_version: Option<u64>,
        choice: ConflictChoice,
    },
    SetRefresh {
        interval: u32,
    },
    History {
        change_id: String,
    },
    Revision {
        change_id: String,
        revision_id: String,
    },
    AcceptRead,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictChoice {
    Remote,
    Local,
}
#[derive(Serialize)]
pub struct SyncDiagnostic {
    pub code: &'static str,
    pub message: &'static str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationSyncResponse {
    pub snapshot: AnnotationSyncSnapshot,
    pub outcome: Option<ZoteroWriteOutcome>,
    pub cache: Option<AnnotationReadCache>,
    pub diagnostic: Option<SyncDiagnostic>,
    pub history: Option<Vec<RevisionMetadata>>,
    pub historical_entry: Option<AnnotationSyncEntry>,
    pub active_id: Option<String>,
}
fn write_error(e: ZoteroWriteError) -> CommandError {
    CommandError {
        code: e.code,
        message: e.message.into(),
    }
}
fn diagnostic(e: ZoteroWriteError) -> SyncDiagnostic {
    SyncDiagnostic {
        code: e.code,
        message: e.message,
    }
}
fn timestamp() -> Result<i64, CommandError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|v| v.as_millis().try_into().ok())
        .ok_or_else(|| fail("系统时间无效，请保留草稿。"))
}
fn read_error(e: &str) -> ZoteroWriteError {
    if e.contains("实例") || e == crate::services::zotero_annotations::SOURCE_MISMATCH {
        ZoteroWriteError {
            code: "ZOTERO_WRITE_SOURCE_MISMATCH",
            message: "实例、资料库或附件不一致；待处理修改保留，未跨来源写入。",
        }
    } else if e.contains("取消") {
        ZoteroWriteError {
            code: "ZOTERO_WRITE_CANCELLED",
            message: "操作已取消，已落盘修改保留。",
        }
    } else {
        ZoteroWriteError {
            code: "ZOTERO_WRITE_UNAVAILABLE",
            message: "Zotero连接或读取失败；旧缓存与待处理修改保留。",
        }
    }
}
fn save_failure(
    library: &LibraryService,
    mut entry: AnnotationSyncEntry,
    e: ZoteroWriteError,
    uncertain: bool,
) -> Result<AnnotationSyncEntry, CommandError> {
    entry.needs_verification = uncertain;
    entry.fail(e.code, e.message, timestamp()?);
    let rev = entry.revision;
    library.with(|s| s.save_annotation_sync_entry(entry, rev, true))
}
#[allow(clippy::too_many_arguments)]
pub(super) fn attempt_change(
    library: &LibraryService,
    pdf: &PdfDocumentService,
    sessions: &ZoteroSessions,
    client: &ZoteroClient,
    token: &str,
    source: &AnnotationSource,
    id: &str,
    revision: i64,
    cancel: &AtomicBool,
    verify_only: bool,
) -> Result<(Option<ZoteroWriteOutcome>, Option<SyncDiagnostic>), CommandError> {
    let record = pdf.record(token).map_err(error)?;
    let mut entry = library.with(|s| s.annotation_sync_entry(&record.document_id, source, id))?;
    if entry.revision != revision {
        return Err(error(crate::storage::StorageError::Conflict));
    }
    if entry.local.is_none() {
        return Ok((None, None));
    }
    let (file, original, current) = match pdf.verified_source(token) {
        Ok(v) => v,
        Err(_) => {
            let e = ZoteroWriteError {
                code: "ZOTERO_WRITE_UNAVAILABLE",
                message: "缺少有效原件授权或内容摘要改变；本地修改已保留，请重新选择原件核对。",
            };
            let uncertain = entry.needs_verification;
            save_failure(library, entry, e, uncertain)?;
            return Ok((None, Some(diagnostic(e))));
        }
    };
    let _file = file;
    if current.content_hash != entry.pdf_content_hash {
        return Err(error(crate::storage::StorageError::Conflict));
    }
    library.with(|s| s.annotation_source(&record.document_id, source))?;
    let remote =
        match client.read_annotation_for_sync(source, &original, &entry.annotation_key, cancel) {
            Ok(v) => v,
            Err(e) => {
                let uncertain = entry.needs_verification;
                save_failure(library, entry, e, uncertain)?;
                return Ok((None, Some(diagnostic(e))));
            }
        };
    entry.observe(remote, true, timestamp()?);
    if entry.local.is_none() {
        let annotation = entry.remote.clone();
        let key = entry.annotation_key.clone();
        let rev = entry.revision;
        library.with(|s| s.save_annotation_sync_entry(entry, rev, true))?;
        sessions.finish_write(token, &key).map_err(fail)?;
        return Ok((
            Some(ZoteroWriteOutcome {
                verified: true,
                annotation_key: key,
                annotation,
                message: "已逐项复读确认目标一致，没有重复发送写入。",
            }),
            None,
        ));
    }
    if entry.status == AnnotationSyncState::Conflict {
        let rev = entry.revision;
        library.with(|s| s.save_annotation_sync_entry(entry, rev, true))?;
        return Ok((
            None,
            Some(SyncDiagnostic {
                code: "ZOTERO_WRITE_CONFLICT",
                message: "两端版本或删除状态冲突，本地和远端均已保留；请明确解决。",
            }),
        ));
    }
    // A targeted reread found the original state. A later explicit retry is safe only
    // with the SAME create key/version0 or original object-version precondition.
    sessions
        .finish_write(token, &entry.annotation_key)
        .map_err(fail)?;
    entry.needs_verification = false;
    entry.status = AnnotationSyncState::LocalDirty;
    entry.error_code = None;
    entry.message = "已逐项核对原状态；可按同一key和版本条件提交，未自动重发。".into();
    let rev = entry.revision;
    entry = library.with(|s| s.save_annotation_sync_entry(entry, rev, true))?;
    if verify_only {
        return Ok((None, None));
    }
    let input = entry
        .local
        .as_ref()
        .ok_or_else(|| fail("待处理意图不存在。"))?;
    if let Err(message) = input.validate() {
        let e = ZoteroWriteError {
            code: "ZOTERO_WRITE_INVALID",
            message,
        };
        save_failure(library, entry, e, false)?;
        return Ok((None, Some(diagnostic(e))));
    }
    if cancel.load(Ordering::Relaxed) {
        let e = ZoteroWriteError {
            code: "ZOTERO_WRITE_CANCELLED",
            message: "发送前取消；本地修改已保留。",
        };
        save_failure(library, entry, e, false)?;
        return Ok((None, Some(diagnostic(e))));
    }
    let intent = match client.prepare_annotation_write_reserved(
        source,
        &original,
        input,
        Some(&entry.annotation_key),
        cancel,
    ) {
        Ok(v) => v,
        Err(e) => {
            if e.code == "ZOTERO_WRITE_CONFLICT"
                && let Ok(remote) = client.read_annotation_for_sync(
                    source,
                    &original,
                    &entry.annotation_key,
                    cancel,
                )
            {
                entry.observe(remote, true, timestamp()?);
                if entry.local.is_none() {
                    let key = entry.annotation_key.clone();
                    let annotation = entry.remote.clone();
                    let rev = entry.revision;
                    library.with(|s| s.save_annotation_sync_entry(entry, rev, true))?;
                    return Ok((
                        Some(ZoteroWriteOutcome {
                            verified: true,
                            annotation_key: key,
                            annotation,
                            message: "复读确认同一目标已存在，未重复创建。",
                        }),
                        None,
                    ));
                }
            }
            save_failure(library, entry, e, false)?;
            return Ok((None, Some(diagnostic(e))));
        }
    };
    entry.needs_verification = true;
    entry.message = "提交标记和稳定key已落盘，正在受控写入并复读。".into();
    let rev = entry.revision;
    entry = library.with(|s| s.save_annotation_sync_entry(entry, rev, true))?;
    let grant = match sessions.start_write(token, &entry.pdf_content_hash, intent.clone(), cancel) {
        Ok(v) => v,
        Err(_) => {
            let e = ZoteroWriteError {
                code: "ZOTERO_WRITE_AUTH_REQUIRED",
                message: "当前来源没有可用写入授权；请主动授权，本地修改已保留。",
            };
            save_failure(library, entry, e, false)?;
            return Ok((None, Some(diagnostic(e))));
        }
    };
    let key = entry.annotation_key.clone();
    let result = match client.perform_annotation_write(&intent, &grant, &original, cancel) {
        Ok(outcome) => {
            if outcome.verified {
                entry.remote = outcome.annotation.clone();
                entry.accept_remote(timestamp()?, outcome.message);
            } else {
                entry.fail("ZOTERO_WRITE_UNKNOWN", outcome.message, timestamp()?);
                entry.needs_verification = true;
            }
            let rev = entry.revision;
            library.with(|s| s.save_annotation_sync_entry(entry, rev, true))?;
            Ok((Some(outcome), None))
        }
        Err(e) => {
            if matches!(e.code, "ZOTERO_WRITE_CONFLICT" | "ZOTERO_WRITE_NOT_FOUND")
                && let Ok(remote) = client.read_annotation_for_sync(
                    source,
                    &original,
                    &key,
                    &AtomicBool::new(false),
                )
            {
                entry.observe(remote, true, timestamp()?);
            }
            let uncertain = matches!(
                e.code,
                "ZOTERO_WRITE_UNKNOWN" | "ZOTERO_WRITE_PROTOCOL" | "ZOTERO_WRITE_SOURCE_MISMATCH"
            );
            save_failure(library, entry, e, uncertain)?;
            if matches!(
                e.code,
                "ZOTERO_WRITE_AUTH_REQUIRED" | "ZOTERO_WRITE_SOURCE_MISMATCH"
            ) {
                sessions.forget_write_grant(token).map_err(fail)?;
            }
            Ok((None, Some(diagnostic(e))))
        }
    };
    // The durable submitted row remains authoritative across restart. The ephemeral
    // native mutex can be released after its result has safely reached SQLite.
    sessions.finish_write(token, &key).map_err(fail)?;
    result
}
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn zotero_annotation_sync(
    window: WebviewWindow,
    library: State<'_, Arc<LibraryService>>,
    pdf: State<'_, Arc<PdfDocumentService>>,
    sessions: State<'_, Arc<ZoteroSessions>>,
    token: String,
    source: AnnotationSource,
    lookup_id: String,
    request: AnnotationSyncRequest,
) -> Result<AnnotationSyncResponse, CommandError> {
    check(&window)?;
    source.validate().map_err(fail)?;
    let cancel = sessions.begin(&lookup_id, &token).map_err(fail)?;
    let library = Arc::clone(library.inner());
    let pdf = Arc::clone(pdf.inner());
    let service = Arc::clone(sessions.inner());
    let owner = service.clone();
    let result=tauri::async_runtime::spawn_blocking(move || {
  let record=pdf.record(&token).map_err(error)?;library.with(|s|s.annotation_source(&record.document_id,&source))?;
  let client=ZoteroClient::new().map_err(fail)?;let mut outcome=None;let mut cache=None;let mut diag=None;let mut history=None;let mut historical_entry=None;let mut active_id=None;
  let verify_only_request=matches!(&request,AnnotationSyncRequest::Verify{..});
  match request {
   AnnotationSyncRequest::List=>{library.with(|s|{let snapshot=s.annotation_sync_snapshot(&record.document_id,&source)?;if let Some(c)=s.annotation_cache(&record.document_id,&source)?&&snapshot.scope.last_refreshed_at.unwrap_or(0)<c.fetched_at {s.observe_annotation_sync(&record.document_id,&c)?;}Ok(())})?;},
   AnnotationSyncRequest::Stage{change_id,revision,input}=>{active_id=Some(library.with(|s|s.stage_annotation_change(&record.document_id,&source,change_id.as_deref(),revision,input))?.id);},
   AnnotationSyncRequest::SetRefresh{interval}=>{let mut scope=library.with(|s|s.annotation_sync_snapshot(&record.document_id,&source))?.scope;scope.refresh_interval=interval;library.with(|s|s.save_annotation_sync_scope(&record.document_id,&source,&scope))?;},
   AnnotationSyncRequest::History{change_id}=>{history=Some(library.with(|s|s.annotation_sync_history(&record.document_id,&source,&change_id))?);},
   AnnotationSyncRequest::Revision{change_id,revision_id}=>{historical_entry=Some(library.with(|s|s.annotation_sync_revision(&record.document_id,&source,&change_id,&revision_id))?);},
   AnnotationSyncRequest::Refresh=>{
    let read=(||{let (_file,path,current)=pdf.verified_source(&token).map_err(|_|ZoteroWriteError {code:"ZOTERO_WRITE_UNAVAILABLE",message:"请重新选择原件并核对摘要；旧缓存与待处理记录保留。"})?;client.read_annotations(&source,&path,&current.content_hash,&cancel).map_err(|e|read_error(&e))})();
    match read {Ok(fresh)=>{if cancel.load(Ordering::Relaxed){return Err(fail("刷新已取消，本地修改保留。"));}library.with(|s|{s.save_annotation_cache(&record.document_id,&fresh)?;s.observe_annotation_sync(&record.document_id,&fresh)})?;cache=Some(fresh);},Err(e)=>{let snapshot=library.with(|s|s.annotation_sync_snapshot(&record.document_id,&source))?;for entry in snapshot.entries.into_iter().filter(|e|e.local.is_some()){let uncertain=entry.needs_verification;save_failure(&library,entry,e,uncertain)?;}let mut scope=snapshot.scope;scope.message=e.message.into();library.with(|s|s.save_annotation_sync_scope(&record.document_id,&source,&scope))?;diag=Some(diagnostic(e));}}
   },
   AnnotationSyncRequest::Submit{change_id,revision}|AnnotationSyncRequest::Verify{change_id,revision}=>{
    let verify_only=verify_only_request;let mut scope=library.with(|s|s.annotation_sync_snapshot(&record.document_id,&source))?.scope;scope.last_attempt_at=Some(timestamp()?);library.with(|s|s.save_annotation_sync_scope(&record.document_id,&source,&scope))?;
    let (o,d)=attempt_change(&library,&pdf,&service,&client,&token,&source,&change_id,revision,&cancel,verify_only)?;outcome=o;diag=d;
   },
   AnnotationSyncRequest::Resolve{change_id,revision,remote_version,choice}=>{
    let mut entry=library.with(|s|s.annotation_sync_entry(&record.document_id,&source,&change_id))?;if entry.revision!=revision{return Err(error(crate::storage::StorageError::Conflict));}
    let (_file,path,_)=pdf.verified_source(&token).map_err(error)?;let remote=client.read_annotation_for_sync(&source,&path,&entry.annotation_key,&cancel).map_err(write_error)?;
    if remote.as_ref().map(|a|a.version)!=remote_version{return Err(fail("远端在解决冲突期间再次变化，请重新读取；双方内容保留。"));}entry.remote=remote;
    match choice {
     ConflictChoice::Remote=>entry.accept_remote(timestamp()?,"已明确采用刚读取的远端状态；原本地输入已保留于处理修订。"),
     ConflictChoice::Local=>{let remote=entry.remote.clone().ok_or_else(||fail("远端已删除，不能向不存在的对象盲目写回。请保留或复制本地内容，重新选区创建，或明确接受删除。"))?;let local=entry.local.as_mut().ok_or_else(||fail("没有待保留的本地修改。"))?;match local {AnnotationWriteInput::Update{expected_version,..}|AnnotationWriteInput::Delete{expected_version,..}=>*expected_version=remote.version,AnnotationWriteInput::Create{..}=>return Err(fail("新建key已被其他对象占用，不能覆盖；请保留输入另行创建。"))};entry.base=Some(remote);entry.needs_verification=false;entry.status=AnnotationSyncState::LocalDirty;entry.error_code=None;entry.updated_at=timestamp()?;entry.message="本地意图已按刚核对的远端版本重新排队；仍需显式提交，不会立即覆盖。".into();}
    }
    library.with(|s|s.save_annotation_sync_entry(entry,revision,true))?;
   },
   AnnotationSyncRequest::AcceptRead=>{let snapshot=library.with(|s|s.annotation_sync_snapshot(&record.document_id,&source))?;let time=timestamp()?;for mut entry in snapshot.entries.into_iter().filter(|e|e.local.is_none()&&e.status==AnnotationSyncState::RemoteChanged){let rev=entry.revision;entry.accept_remote(time,"已接受最近读取的远端变化，未向Zotero写入。");library.with(|s|s.save_annotation_sync_entry(entry,rev,true))?;}}
  }
  let mut snapshot=library.with(|s|s.annotation_sync_snapshot(&record.document_id,&source))?;
  if outcome.as_ref().is_some_and(|o|o.verified)&&snapshot.entries.iter().all(|e|e.local.is_none()){snapshot.scope.last_synced_at=Some(timestamp()?);library.with(|s|s.save_annotation_sync_scope(&record.document_id,&source,&snapshot.scope))?;}
  Ok::<_,CommandError>(AnnotationSyncResponse {snapshot,outcome,cache,diagnostic:diag,history,historical_entry,active_id})
 }).await.map_err(|_|fail("批注同步任务异常结束；已落盘意图保留，下次先复核。"));
    owner.end(&lookup_id).map_err(fail)?;
    result?
}
