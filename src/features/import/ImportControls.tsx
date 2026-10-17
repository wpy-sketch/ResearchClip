import {ClipboardPaste,X} from 'lucide-react';
import {isTauri} from '@tauri-apps/api/core';
import {useImportStore} from './store';
import {useCaptureStore} from '../capture/store';
import {useDocumentStore} from '../document/store';
export function ClipboardImportButton({markdown=false}:{markdown?:boolean}){
  const captureBusy=useCaptureStore(s=>s.phase!=='idle');
  const busy=useImportStore(s=>s.busy),documentBusy=useDocumentStore(s=>s.phase!=='idle');const label=markdown?'从剪贴板插入图片':'从剪贴板导入图片';
  return <button className="icon-button" aria-label={label} title={label} disabled={!isTauri()||busy||captureBusy||(markdown&&documentBusy)} onClick={()=>void useImportStore.getState().run('clipboard',markdown?'markdown':'library')}><ClipboardPaste size={17}/></button>;
}
export function ImportStatus(){
  const s=useImportStore(),hasDocument=useDocumentStore(s=>Boolean(s.document)),docBusy=useDocumentStore(s=>s.phase!=='idle');if(!s.busy&&!s.report&&!s.error)return null;
  const success=s.report?.items.filter(i=>i.screenshot).length??0,failed=s.report?.items.filter(i=>i.error)??[];
  return <section className="image-import-status" aria-label="图片导入结果" aria-busy={s.busy}>
    <div className="flex flex-wrap items-center gap-2"><p role="status">{s.busy?'正在导入图片…':s.report?`已导入 ${success} 张 · 失败 ${failed.length} 张${s.inserted?' · 已插入 Markdown':''}`:'图片导入未完成'}</p>{hasDocument&&success>0&&!s.inserted&&<button className="secondary-button" disabled={s.busy||docBusy} onClick={()=>void s.insertLast()}>插入当前 Markdown</button>}<button className="icon-button ml-auto" aria-label="关闭导入结果" disabled={s.busy} onClick={s.clear}><X size={14}/></button></div>
    {s.error&&<p role="alert" className="text-error">{s.error}</p>}{failed.length>0&&<ul aria-label="导入失败文件">{failed.map((item,i)=><li key={i}>{item.name}：{item.error}</li>)}</ul>}
    {success>0&&<p className="text-xs text-muted">资源已复制到素材库，可改名、写笔记和编辑标注；当前筛选可能隐藏新素材。重复导入创建独立副本。</p>}
  </section>;
}
