import {useEffect,useRef,useState} from 'react';
import {History,RotateCcw} from 'lucide-react';
import {Modal} from '../../shared/ui/Modal';
import {errorMessage} from '../../shared/api/errors';
import {useEditorStore} from './store';
import {renderComposite} from './export';
import {revisionDocument,revisionHistory,type AnnotationRevision} from './revisions-api';
import type {EditorSession} from './types';
import {restoreEditorFromHistory} from '../../app/library-actions';

export function RevisionHistoryButton({session}:{session:EditorSession}){
 const [open,setOpen]=useState(false);
 return <><button className="secondary-button" disabled={!session.image.libraryId||Boolean(session.loadError)} onClick={()=>{useEditorStore.getState().cancelGesture();setOpen(true);}}><History size={15}/>版本历史{session.image.annotationVersion?` · v${session.image.annotationVersion}`:''}</button>{open&&<RevisionHistory key={session.image.id} session={session} onClose={()=>setOpen(false)}/>}</>;
}
function RevisionHistory({session,onClose}:{session:EditorSession;onClose:()=>void}){
 const id=session.image.libraryId!,[rows,setRows]=useState<AnnotationRevision[]>([]),[selected,setSelected]=useState<AnnotationRevision|null>(null),[busy,setBusy]=useState(true),[error,setError]=useState<string|null>(null),[urls,setUrls]=useState<string[]>([]),[compare,setCompare]=useState(false),[description,setDescription]=useState(''),[confirm,setConfirm]=useState(false),[more,setMore]=useState(false);
 const generation=useRef(0),locked=useRef(false);
 useEffect(()=>{const pending=generation;let live=true;void revisionHistory(id).then(result=>{if(live){setRows(result);setMore(result.length===50);}}).catch(e=>{if(live)setError(errorMessage(e,'读取版本失败。'));}).finally(()=>{if(live)setBusy(false);});return()=>{live=false;pending.current++;};},[id]);
 useEffect(()=>()=>{urls.forEach(url=>URL.revokeObjectURL(url));},[urls]);
 const preview=async(row:AnnotationRevision,comparison:boolean)=>{
  if(locked.current)return;locked.current=true;const token=++generation.current;setBusy(true);setError(null);setConfirm(false);setSelected(row);setCompare(comparison);setUrls([]);
  try{const old=await revisionDocument(id,row.revisionId);const blobs=[await renderComposite(session.image.blob,old.document)];if(comparison)blobs.push(await renderComposite(session.image.blob,session.document));if(generation.current===token)setUrls(blobs.map(blob=>URL.createObjectURL(blob)));}
  catch(e){if(generation.current===token)setError(errorMessage(e,'历史预览失败，当前草稿未改动。'));}finally{locked.current=false;if(generation.current===token)setBusy(false);}
 };
 const restore=async()=>{
  if(!selected||locked.current)return;locked.current=true;setBusy(true);setError(null);
  try{const current=useEditorStore.getState().sessions[session.image.id];if(!current||JSON.stringify(current.document)!==JSON.stringify(session.document))throw Error('当前草稿已改变，请关闭历史并重新打开后恢复。');
   await restoreEditorFromHistory(session,selected.revisionId,description.trim()||`从 v${selected.versionNumber} 恢复`);onClose();
  }catch(e){setError(errorMessage(e,'恢复失败，当前草稿和历史均已保留。'));}finally{locked.current=false;setBusy(false);}
 };
 const loadMore=async()=>{if(locked.current)return;locked.current=true;setBusy(true);setError(null);try{const next=await revisionHistory(id,rows.at(-1)!.versionNumber);setRows(previous=>[...previous,...next]);setMore(next.length===50);}catch(e){setError(errorMessage(e,'读取更早版本失败。'));}finally{locked.current=false;setBusy(false);}};
 return <Modal title="标注版本历史" onClose={()=>{if(!locked.current)onClose();else if(!confirm){generation.current++;onClose();}}}><div className="revision-history">
  <p className="revision-intro">编辑基准：{session.image.annotationVersion?`v${session.image.annotationVersion}`:'首次保存'}{JSON.stringify(session.document)!==session.savedDocument?' · 含未保存草稿':''}。预览不改动编辑状态；恢复会追加版本，原图和旧版本保留。</p>
  {error&&<p role="alert" className="error-banner">{error}</p>}{busy&&<p role="status">正在读取或合成版本…</p>}
  <div className="revision-layout"><aside aria-label="历史版本列表">{rows.map(row=><button key={row.revisionId} disabled={busy} aria-pressed={selected?.revisionId===row.revisionId} onClick={()=>void preview(row,false)}><strong>v{row.versionNumber}{row.versionNumber===session.image.annotationVersion?' · 编辑基准':''}</strong><time>{new Date(row.createdAt).toLocaleString('zh-CN')}</time><span>{row.description||'无版本说明'}</span>{row.schemaVersion!==1&&<span className="text-error">不支持的格式；记录保留</span>}</button>)}{more&&<button disabled={busy} onClick={()=>void loadMore()}>读取更早版本</button>}{!rows.length&&!busy&&<p>暂无版本；请先保存到素材库。</p>}</aside>
  <section className="revision-detail"><div className="flex flex-wrap gap-2"><button className="secondary-button" disabled={!selected||busy} onClick={()=>selected&&void preview(selected,!compare)}>{compare?'仅预览旧版本':'与当前草稿对比'}</button><button className="secondary-button" disabled={!selected||busy||urls.length===0} onClick={()=>setConfirm(true)}><RotateCcw size={14}/>恢复为新版本</button></div>
   <div className={`revision-previews ${compare?'compare':''}`}>{urls.length?urls.map((url,index)=><figure key={url}><figcaption>{index===0?`历史 v${selected?.versionNumber}`:'当前编辑草稿'}</figcaption><img src={url} alt={index===0?'历史版本标注合成图':'当前草稿标注合成图'}/></figure>):<p className="text-muted">选择一个历史版本预览。损坏版本会显示错误，不影响其他版本。</p>}</div>
   {confirm&&<div className="revision-confirm"><p>将用 v{selected?.versionNumber} 的标注创建新版本。{JSON.stringify(session.document)!==session.savedDocument?'当前未保存草稿不会自动落盘；请取消并先保存，或明确放弃草稿后恢复。':'已有版本不会被删除。'}相同内容保持现有版本。</p><label>新版本说明<input maxLength={500} value={description} onChange={e=>setDescription(e.target.value)} placeholder={`从 v${selected?.versionNumber} 恢复`}/></label><div className="flex gap-2"><button disabled={busy} className="secondary-button" onClick={()=>setConfirm(false)}>取消恢复</button><button disabled={busy} className="capture-button" onClick={()=>void restore()}>{JSON.stringify(session.document)!==session.savedDocument?'放弃草稿并恢复为新版本':'确认恢复为新版本'}</button></div></div>}
  </section></div><p className="text-xs text-muted">历史不自动清理。会话撤销/重做独立保留；恢复后撤销产生草稿，不删除任何已保存版本。</p>
 </div></Modal>;
}
