import {useEffect,useState} from 'react';
import {isTauri} from '@tauri-apps/api/core';
import {getLinkDetails,setDocumentLinkTags,exportLinkedDocument,type LinkDetails as Details} from './api';
import {indexDocumentLinks} from './indexing';
import {openInternalTarget} from './navigation';
import type {Target} from './model';
import {useTagStore} from '../tags/store';
import {useDocumentStore,documentDirty} from '../document/store';
import {errorMessage} from '../../shared/api/errors';
export function LinkDetails({kind,id,savedVersion}:{kind:Target['kind'];id:string;savedVersion?:string|null}){
 const [version,setVersion]=useState(0),[result,setResult]=useState<{key:string;data:Details}|null>(null),[status,setStatus]=useState(''),[error,setError]=useState<string|null>(null),[busy,setBusy]=useState(false),[exportReport,setExportReport]=useState<{path:string;warnings:string[]}|null>(null),[indexBusy,setIndexBusy]=useState(false),[cancel,setCancel]=useState<AbortController|null>(null);
 const tags=useTagStore(s=>s.items),document=useDocumentStore(),key=`${kind}:${id}:${version}`,data=result?.key===key?result.data:null;
 useEffect(()=>{if(!isTauri())return;void useTagStore.getState().refresh();let active=true;void getLinkDetails({kind,id}).then(data=>{if(active)setResult({key,data});}).catch(e=>{if(active)setError(errorMessage(e,'反向链接读取失败。'));});return()=>{active=false;};},[kind,id,key]);
 useEffect(()=>{
  if(kind!=='document'||!isTauri())return;
  let active=true;const controller=new AbortController(),timer=setTimeout(()=>{
   setIndexBusy(true);setCancel(controller);setStatus('正在后台更新已保存文件的链接索引…');
   void indexDocumentLinks(id,controller.signal).then(()=>{if(active){setStatus('已保存文件的链接索引已更新');setError(null);setVersion(n=>n+1);}}).catch(e=>{if(active){if(controller.signal.aborted){setError(null);setStatus('已取消索引，已有索引保留');}else{setError(errorMessage(e,'索引更新失败，已有索引保留。'));setStatus('索引未更新');}}}).finally(()=>{if(active){setIndexBusy(false);setCancel(null);}});
  },250);
  return()=>{active=false;clearTimeout(timer);controller.abort();};
 },[kind,id,savedVersion]);
 useEffect(()=>{const updated=()=>setVersion(v=>v+1);window.addEventListener('researchclip-links-updated',updated);return()=>window.removeEventListener('researchclip-links-updated',updated);},[]);
 const action=async(fn:()=>Promise<unknown>)=>{setError(null);setBusy(true);try{await fn();}catch(e){setError(errorMessage(e,'内部链接操作失败。'));}finally{setBusy(false);}};
 const open=(target:Target,offset=0)=>void action(()=>openInternalTarget(target,offset));
 return <details className="link-details" open aria-label="内部引用与反向链接"><summary>内部引用与反向链接 <span>{data?.backlinks.length??'…'} 处引用</span></summary>
 <div className="link-details-content">{error&&<p role="alert" className="error-banner">{error}<button onClick={()=>setVersion(n=>n+1)}>重试读取</button></p>}{kind==='document'&&<><p className="link-hint">输入 [[ 搜索文档与截图。索引以保存后的文件为准。</p><div className="link-status"><span role="status">{indexBusy?status:data?.stale?'索引尚未更新或文件已变化':status}</span>{indexBusy?<button onClick={()=>cancel?.abort()}>取消索引</button>:<button disabled={busy} onClick={()=>{const controller=new AbortController();setCancel(controller);setIndexBusy(true);setStatus('正在更新索引…');void action(async()=>{try{await indexDocumentLinks(id,controller.signal);setStatus('索引已更新');setVersion(n=>n+1);}catch(e){if(controller.signal.aborted)setStatus('已取消索引，已有索引保留');else throw e;}finally{setIndexBusy(false);setCancel(null);}});}}>更新当前索引</button>}</div>
 <h4>本文引用</h4>{data?.outgoing.length?data.outgoing.map(l=><button className={`link-row ${l.available?'':'is-missing'}`} key={l.start} disabled={busy||!l.available} onClick={()=>open(l)}><span>{l.label}</span><small>第{l.line}行 · {l.available?(l.kind==='document'?'文档':'截图'):'目标已删除或不可访问'}</small></button>):<p className="link-hint">尚无已索引的内部引用。</p>}</>}
 <h4>哪些文档引用了我</h4>{!data&&!error?<p role="status">正在读取反向链接…</p>:data?.backlinks.length?data.backlinks.map(l=><button className="link-row" key={`${l.sourceId}:${l.start}`} disabled={busy||!l.available} onClick={()=>open({kind:'document',id:l.sourceId},l.start)}><span>{l.title}</span><small>第{l.line}行 · {l.label}{!l.available?' · 来源文件缺失':''}</small></button>):<p className="link-hint">暂无已索引的文档引用。可在知识图谱更新全部索引。</p>}
 {kind==='document'&&<details className="document-link-tools"><summary>文档标签与标准Markdown导出</summary> <label className="document-link-tags">图谱标签<select size={Math.min(4,Math.max(2,tags.length))} multiple aria-label="当前文档图谱标签" value={data?.tagIds??[]} disabled={busy||!data} onChange={e=>{const selected=Array.from(e.target.selectedOptions,o=>o.value);void action(async()=>{await setDocumentLinkTags(id,selected);setVersion(n=>n+1);});}}>{tags.map(t=><option key={t.id} value={t.id}>{t.name}</option>)}</select></label><p className="link-hint">可在标签页创建标签；按Ctrl选择多个标签。</p>
 <button className="secondary-button" disabled={busy||indexBusy||!data||data.stale||documentDirty(document)||!document.token} onClick={()=>void action(async()=>{setExportReport(null);const report=await exportLinkedDocument(document.token!);if(report)setExportReport(report);})}>导出标准Markdown副本</button>{exportReport&&<div role="status" className="link-export-report"><p>已导出：{exportReport.path}</p><p>文档链接已转换为相对路径；目标文件不会自动复制，请保留相对目录结构。</p>{exportReport.warnings.map((w,i)=><p key={i}>{w}</p>)}</div>}
</details>}
 {data?.truncated&&<p role="status">本区最多显示500处引用，请缩小范围查看。</p>}</div></details>;
}
