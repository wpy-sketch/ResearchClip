import {RevisionHistoryButton} from './RevisionHistory';
import {MathOcrButton} from '../math-ocr/MathOcr';
import { useCallback, useEffect, useRef, useState } from 'react';
import { pinEditor } from '../pins/api';
import { ArrowLeft, Grid2X2, Droplets, ListOrdered, Pin, ArrowUpRight, Circle, ClipboardCopy, Download, FolderOpen, Hand, MousePointer2, Pencil, Redo2, Save, Square, Trash2, Type, Undo2, X, ZoomIn, ZoomOut } from 'lucide-react';
import { useUiStore } from '../../app/ui-store';
import { useEditorStore } from './store';
import { AnchorWorkspace } from '../anchors/AnchorWorkspace';
import { Modal } from '../../shared/ui/Modal';
import { createAnnotation } from './model';
import type { EditorTool, Point } from './types';
import { exportImage, openProject, saveProject } from './api';
import { saveEditorToLibrary } from '../../app/library-actions';

import {InsertButton} from '../document/InsertButton';
import {OcrButton} from '../ocr/OcrButton';
import {renderComposite} from './export';
const tools = [{id:'select',label:'选择',icon:MousePointer2},{id:'pan',label:'平移',icon:Hand},{id:'rectangle',label:'矩形',icon:Square},{id:'ellipse',label:'椭圆',icon:Circle},{id:'arrow',label:'箭头',icon:ArrowUpRight},{id:'pen',label:'自由画笔',icon:Pencil},{id:'text',label:'文字',icon:Type},{id:'mosaic',label:'马赛克',icon:Grid2X2},{id:'blur',label:'局部模糊',icon:Droplets},{id:'number',label:'编号',icon:ListOrdered}] as const;
export function EditorView({sessionId}:{sessionId?:string}={}) {
  const { sessions, activeId:globalActiveId, tool, style, error, activate, close, setTool, fit, zoom, setStyle, removeSelected, select } = useEditorStore();
  const activeId=sessionId??globalActiveId;
  const host=useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 1, height: 1 });
  const [textTarget,setTextTarget] = useState<{point:Point;id?:string}|null>(null);
  const [closeTarget,setCloseTarget] = useState<string|null>(null);
  const [revisionDescriptions,setRevisionDescriptions]=useState<Record<string,string>>({});
  const revisionDescription=activeId?revisionDescriptions[activeId]??'':'';
  const [busy, setBusy] = useState(false); const locked = useRef(false);
  const [ioMessage, setIoMessage] = useState<string | null>(null); const [ioError, setIoError] = useState<string | null>(null);
  const session = activeId ? sessions[activeId] : null;
  const selected=session?.document.objects.find((a)=>a.id===session.selectedId);
  const editText=useCallback((point:Point,id?:string)=>setTextTarget({point,id}),[]);
  const operation = async (action: 'open' | 'project' | 'save' | 'copy' | 'library' | 'pin') => {
    if (locked.current) return; locked.current=true; setBusy(true); setIoError(null); setIoMessage(null); const finishIo=useEditorStore.getState().beginIo();useEditorStore.getState().cancelGesture();
    try {
      if (action==='open') {
        const data=await openProject(); if(data) { const id=crypto.randomUUID();useEditorStore.getState().open({id,name:'已加载项目',widthPx:data.widthPx,heightPx:data.heightPx,blob:data.blob},data.project.document); if(useEditorStore.getState().activeId!==id)throw new Error(useEditorStore.getState().error??'无法打开项目。');setIoMessage('项目已恢复，全部标注可以继续编辑。'); }
      } else if(action==='pin'&&session&&!session.loadError) {
        await pinEditor(session);setIoMessage('标注合成图已创建为独立桌面贴图。');
      } else if(action==='library'&&session&&!session.loadError) {
        await saveEditorToLibrary(session,revisionDescription);setRevisionDescriptions(previous=>({...previous,[session.image.id]:''}));setIoMessage('标注已保存；内容未改变时保留现有版本。');
      } else if(session&&!session.loadError&&action!=='library'&&action!=='pin') {
        const result=action==='project'?await saveProject(session):await exportImage(session,action);
        if(result){if(action==='project')useEditorStore.getState().markSaved(session.image.id,session.document);setIoMessage(action==='copy'?result:action==='project'?`可编辑项目已保存：${result}。请将 JSON 与 original PNG 一起保留。`:`合成 PNG 已导出：${result}`);}
      }
    } catch(e) { setIoError(e instanceof Error?e.message:e&&typeof e==='object'&&'message' in e?String(e.message):'操作失败，请重试。'); }
    finally { finishIo();locked.current=false;setBusy(false); }
  };
  useEffect(()=>{
    const key=(event:KeyboardEvent)=>{
      if(locked.current||sessionId&&(useEditorStore.getState().activeId!==sessionId||!(event.target instanceof Node)||!host.current?.contains(event.target)))return;
      if(document.querySelector('dialog[open]'))return;
      const target=event.target;if(target instanceof HTMLElement&&(target.closest('[data-markdown-panel]')||target.isContentEditable||['INPUT','TEXTAREA','SELECT'].includes(target.tagName)))return;
      if((event.ctrlKey||event.metaKey)&&!event.altKey){const k=event.key.toLowerCase();if(k==='z'||k==='y'){event.preventDefault();const state=useEditorStore.getState();if(k==='y'||event.shiftKey)state.redo();else state.undo();return;}}
      if(event.key==='Delete'||event.key==='Backspace'){event.preventDefault();useEditorStore.getState().removeSelected();}
      if(event.key==='Escape')useEditorStore.getState().cancelGesture();
      const toolKeys:Record<string,EditorTool>={v:'select',h:'pan',r:'rectangle',e:'ellipse',a:'arrow',p:'pen',t:'text'};
      if(!event.ctrlKey&&!event.altKey&&!event.metaKey&&toolKeys[event.key.toLowerCase()]){event.preventDefault();useEditorStore.getState().setTool(toolKeys[event.key.toLowerCase()]);}
    };window.addEventListener('keydown',key);return()=>{window.removeEventListener('keydown',key);useEditorStore.getState().cancelGesture();};
  },[sessionId]);
  const textObject=textTarget?.id?session?.document.objects.find((a)=>a.id===textTarget.id):null;
  return <div ref={host} className="editor-page" inert={busy}>
    <div className="editor-heading"><button className="secondary-button" onClick={() => useUiStore.getState().navigate('screenshots')}><ArrowLeft size={15} />返回截图</button><h2>图片编辑器</h2><span className="text-xs text-muted">原图保持不变</span></div>
    <div className="editor-toolbar" role="group" aria-label="项目和导出"><MathOcrButton disabled={!session||Boolean(session.loadError)||Boolean(useEditorStore.getState().gesture)} libraryId={session?.image.libraryId} name={session?.image.name??'公式图片'} image={async()=>{if(!session)throw Error('没有可识别图片。');return session.image.blob;}}/><OcrButton disabled={!session||Boolean(session.loadError)||Boolean(useEditorStore.getState().gesture)} name={session?.image.name??"编辑器图片"} image={async()=>{if(!session)throw new Error("没有可识别的图片。");return renderComposite(session.image.blob,session.document);}}/><button className="secondary-button" disabled={!session||Boolean(session.loadError)} onClick={()=>void operation('pin')}><Pin size={15}/>贴图</button><InsertButton editor disabled={!session||Boolean(session.loadError)||Boolean(useEditorStore.getState().gesture)}/><button className="secondary-button" disabled={!session||Boolean(session.loadError)} onClick={()=>void operation('library')}><Save size={15}/>保存到素材库</button>{session&&<RevisionHistoryButton session={session}/>}<button className="secondary-button" onClick={()=>void operation('open')}><FolderOpen size={15}/>打开项目</button><button className="secondary-button" disabled={!session||Boolean(session.loadError)} onClick={()=>void operation('project')}><Save size={15}/>另存可编辑项目</button><button className="secondary-button" disabled={!session||Boolean(session.loadError)} onClick={()=>void operation('save')}><Download size={15}/>导出标注 PNG</button><button className="secondary-button" disabled={!session||Boolean(session.loadError)} onClick={()=>void operation('copy')}><ClipboardCopy size={15}/>复制合成图片</button></div>
    {session&&<div className="revision-save-bar"><span className="revision-version">{session.image.annotationVersion?`编辑基准 v${session.image.annotationVersion}`:"尚未存入素材库"}{JSON.stringify(session.document)!==session.savedDocument?" · 未保存修改":""}</span><label>版本说明<input aria-label="版本说明" maxLength={500} disabled={!session.image.libraryId} value={revisionDescription} onChange={e=>{const value=e.target.value;setRevisionDescriptions(previous=>({...previous,[session.image.id]:value}));}} placeholder={session.image.libraryId?"可选，例如：调整论文图注":"首次保存自动创建 v1"}/></label><span>仅显式保存创建版本，原图始终保留</span></div>}
    {busy&&<p className="mx-3 text-xs text-muted" role="status">正在处理图片或等待系统文件对话框…</p>}
    {ioMessage&&<p className="mx-3 break-all text-xs text-secondary" role="status">{ioMessage}</p>}
    {ioError&&<p className="error-banner m-3" role="alert">{ioError}</p>}
    {error && <p className="error-banner m-3" role="alert">{error}</p>}
    {!sessionId&&<div className="editor-tabs" role="tablist" aria-label="编辑中的图片">{Object.values(sessions).map((s) => <div className="flex items-center" key={s.image.id}><button role="tab" aria-selected={s.image.id === activeId} onClick={() => activate(s.image.id)} className={s.image.id === activeId ? 'is-selected' : ''}>{s.image.name}</button><button className="icon-button" aria-label={`关闭图片 ${s.image.name}`} onClick={() => {if(!close(s.image.id))setCloseTarget(s.image.id);}}><X size={13} /></button></div>)}</div>}
    {session ? <>
      <div className="editor-toolbar"><div className="flex flex-wrap gap-1" role="group" aria-label="编辑工具">{tools.map(({id,label,icon:Icon})=><button key={id} className="icon-button" title={label} aria-label={`${label}工具`} aria-pressed={tool===id} disabled={Boolean(session.loadError)} onClick={()=>setTool(id as EditorTool)}><Icon size={17}/></button>)}</div>
        <button className="icon-button" aria-label="撤销" title="撤销 (Ctrl+Z)" disabled={!session.history.past.length} onClick={()=>useEditorStore.getState().undo()}><Undo2 size={16}/></button><button className="icon-button" aria-label="重做" title="重做 (Ctrl+Y / Ctrl+Shift+Z)" disabled={!session.history.future.length} onClick={()=>useEditorStore.getState().redo()}><Redo2 size={16}/></button>
        <div className="flex items-center gap-2 text-xs text-muted"><label className="flex items-center gap-1">颜色<input type="color" aria-label="标注颜色" value={selected?.color??style.color} onChange={(e)=>setStyle({color:e.target.value})} className="h-7 w-7 cursor-pointer rounded border border-edge bg-surface"/></label>
          <label className="flex items-center gap-1">线宽<input type="number" aria-label="标注线宽" min={1} max={32} value={selected?.strokeWidth??style.strokeWidth} onChange={(e)=>setStyle({strokeWidth:Number(e.target.value)})} className="editor-number"/></label>
          <label className="flex items-center gap-1">字号<input type="number" aria-label="文字字号" min={8} max={200} value={selected?.type==='text'?selected.fontSize:style.fontSize} onChange={(e)=>setStyle({fontSize:Number(e.target.value)})} className="editor-number"/></label>
          <button className="icon-button" aria-label="删除所选标注" title="删除所选标注 (Delete)" disabled={!selected} onClick={removeSelected}><Trash2 size={16}/></button>
        </div>
        <div className="ml-auto flex items-center gap-2"><button className="icon-button" aria-label="缩小画布" onClick={() => zoom(session.viewport.scale / 1.25, { x: size.width / 2, y: size.height / 2 })}><ZoomOut size={16} /></button><span className="min-w-12 text-center text-xs" aria-label="画布缩放比例">{new Intl.NumberFormat('zh-CN',{maximumSignificantDigits:3}).format(session.viewport.scale * 100)}%</span><button className="icon-button" aria-label="放大画布" onClick={() => zoom(session.viewport.scale * 1.25, { x: size.width / 2, y: size.height / 2 })}><ZoomIn size={16} /></button><button className="secondary-button" onClick={() => fit(size.width, size.height)}>适应窗口</button></div>
      </div>
      <div className="editor-toolbar" role="group" aria-label="高级标注属性">
        {(tool==='mosaic'||selected?.type==='mosaic')&&<label className="text-xs text-muted">马赛克块大小 <input className="editor-number" type="number" aria-label="马赛克强度" min={2} max={64} value={selected?.type==='mosaic'?selected.strength:style.mosaicSize??12} onChange={e=>setStyle({mosaicSize:Number(e.target.value)})}/> 像素</label>}
        {(tool==='blur'||selected?.type==='blur')&&<label className="text-xs text-muted">模糊半径 <input className="editor-number" type="number" aria-label="模糊强度" min={1} max={64} value={selected?.type==='blur'?selected.strength:style.blurRadius??8} onChange={e=>setStyle({blurRadius:Number(e.target.value)})}/> 像素</label>}
        {(tool==='number'||selected?.type==='number')&&<label className="text-xs text-muted">编号大小 <input className="editor-number" type="number" aria-label="编号大小" min={16} max={200} value={selected?.type==='number'?selected.size:style.numberSize??36} onChange={e=>setStyle({numberSize:Number(e.target.value)})}/></label>}
        {selected?.type==='number'&&<label className="text-xs text-muted">编号数值 <input className="editor-number" type="number" aria-label="编号数值" min={1} max={9999} value={selected.value} onChange={e=>useEditorStore.getState().update({...selected,value:Number(e.target.value)})}/></label>}
        <span className="text-xs text-muted">滤镜作用于原图区域，导出 PNG 后应用效果；可编辑项目仍保留原图。</span>
      </div>
      <AnchorWorkspace key={session.image.id} session={session} onSize={setSize} onText={editText}/>
      <div className="editor-object-list" aria-label="标注对象"><span className="text-xs text-muted">标注 {session.document.objects.length}</span>{session.document.objects.map((a,index)=><button key={a.id} className={session.selectedId===a.id?'is-selected':''} onClick={()=>{setTool('select');select(a.id);}} title={`X ${a.x.toFixed(1)} · Y ${a.y.toFixed(1)}`} data-annotation-id={a.id} aria-pressed={session.selectedId===a.id}>{tools.find((t)=>t.id===a.type)?.label??a.type} {index+1}</button>)}</div>
      <p className="editor-session-notice">Ctrl+Z 撤销 · Ctrl+Y / Ctrl+Shift+Z 重做 · 空格/中键平移 · Esc 取消动作。退出前请保存到素材库或另存可编辑项目；撤销记录仅在本次会话；素材库版本历史持久保留，原图未改动。</p>
      {textTarget&&<Modal title={textTarget.id?'编辑文字':'插入文字'} onClose={()=>setTextTarget(null)}><form className="p-5" onSubmit={(e)=>{e.preventDefault();const text=String(new FormData(e.currentTarget).get('text')??'').trim();if(!text)return;const store=useEditorStore.getState();const old=textTarget.id?store.sessions[store.activeId!]?.document.objects.find((a)=>a.id===textTarget.id):null;const ok=old?.type==='text'?store.update({...old,text}):store.add({...createAnnotation('text',textTarget.point,store.style),type:'text',text,fontSize:store.style.fontSize});if(ok)setTextTarget(null);}}>
        <label className="block text-sm text-muted">文字内容<textarea key={textTarget.id??'new'} name="text" aria-label="标注文字内容" defaultValue={textObject?.type==='text'?textObject.text:''} maxLength={2000} required autoFocus data-initial-focus className="mt-3 block min-h-28 w-full rounded-md border border-edge bg-canvas p-3 text-primary"/></label><div className="mt-4 flex justify-end gap-2"><button type="button" className="secondary-button" onClick={()=>setTextTarget(null)}>取消文字编辑</button><button className="capture-button" type="submit">确认文字</button></div>
      </form></Modal>}
    </> : <div className="flex flex-1 flex-col items-center justify-center gap-4 p-6 text-muted"><p>请在截图完成后选择“打开编辑器”。</p><button className="secondary-button" onClick={() => useUiStore.getState().navigate('screenshots')}>前往截图</button></div>}
    {closeTarget&&sessions[closeTarget]&&<Modal title="关闭未保存图片" onClose={()=>setCloseTarget(null)}><div className="p-5"><p className="text-sm leading-relaxed">“{sessions[closeTarget].image.name}”有未保存标注。请返回保存到素材库或另存可编辑项目；关闭后无法恢复这些编辑。</p><div className="mt-4 flex justify-end gap-2"><button className="secondary-button" onClick={()=>setCloseTarget(null)}>继续编辑</button><button className="secondary-button text-error" onClick={()=>{close(closeTarget,true);setCloseTarget(null);}}>放弃标注并关闭</button></div></div></Modal>}
  </div>;
}
