import {useProjectStore} from '../projects/store';
import {LinkDetails} from '../links/LinkDetails';
import {LinkPicker} from '../links/LinkPicker';
import {wikiTrigger} from '../links/model';
import {openInternalTarget} from '../links/navigation';
import {errorMessage} from '../../shared/api/errors';
import {DocumentProject} from '../projects/ProjectBar';
import {ReferenceButton} from '../references/References';
import {lazy,Suspense,useDeferredValue,useCallback,useEffect,useMemo,useRef,useState} from 'react';
import {ExternalOpenButton} from './ExternalOpenButton';
import {Save,X,Undo2,Redo2,FileOutput,Eye,BookTemplate,ListTree,BookmarkPlus} from 'lucide-react';
import {useDocumentStore,documentDirty} from './store';
import {sourceText} from './model';
import {MarkdownPreview} from './MarkdownPreview';
import {templateMarkdown} from './templates';
import {documentOutline,revealSourceHeading,type Heading} from './outline';
import {useImageDrop} from '../import/use-image-drop';
import {ClipboardImportButton} from '../import/ImportControls';
import {DocumentFileName} from './DocumentFileName';
const DocumentView=lazy(()=>import('./DocumentView').then(m=>({default:m.DocumentView})));
export function MarkdownPanel(){
  const drop=useImageDrop('markdown');
  const s=useDocumentStore(),[previewState,setPreviewState]=useState<{token:string|null;value:boolean}>({token:null,value:false}),[live,setLive]=useState(true),[outline,setOutline]=useState(false),[previewTarget,setPreviewTarget]=useState<{offset:number}|null>(null),previewBody=useRef<HTMLDivElement>(null),input=useRef<HTMLTextAreaElement>(null),busy=s.phase!=='idle';
  const preview=previewState.token===s.token&&previewState.value,setPreview=(value:boolean)=>setPreviewState({token:s.token,value});
  const project=useProjectStore(),currentId=project.token===s.token&&s.token?project.current?.id:null;
  const document=s.document,source=document?.mode==='source';
  useEffect(()=>{
    if(!source||preview||busy||!input.current)return;
    const node=input.current,nodeState=useDocumentStore.getState();node.focus();node.setSelectionRange(nodeState.caret.start,nodeState.caret.end);
    revealSourceHeading(node,nodeState.caret.start);
  },[s.focusRequest,source,preview,busy]);
  let markdown='';try{if(document)markdown=templateMarkdown(document);}catch{/* Save reports validation; edits remain available. */}
  const liveMarkdown=useDeferredValue(markdown);
  const trigger=useMemo(()=>source&&!preview?wikiTrigger(markdown,s.caret.start):null,[source,preview,markdown,s.caret.start]);
  const internalOpen=useCallback((target:Parameters<typeof openInternalTarget>[0])=>{void openInternalTarget(target).catch(e=>useDocumentStore.setState({error:errorMessage(e,'内部引用打开失败。')}));},[]);
  const headings=useMemo(()=>outline?documentOutline(markdown):[],[outline,markdown]);
  useEffect(()=>{if(preview&&previewTarget)previewBody.current?.querySelector<HTMLElement>(`[data-heading-offset="${previewTarget.offset}"]`)?.scrollIntoView({block:'start'});},[preview,previewTarget]);
  const jump=(heading:Heading)=>{if(preview||!source){setPreview(true);setPreviewTarget({offset:heading.offset});}else{s.jumpToSource(heading.offset);if(input.current)revealSourceHeading(input.current,heading.offset);}};
  if(!document)return null;
  const file=s.fileName??s.path?.split(/[\\/]/).at(-1)??'untitled.md';
  return <section className={`markdown-panel image-drop-target ${drop.over?'is-drag-over':''}`} {...drop.handlers} data-image-drop-target="markdown" data-markdown-panel aria-label="Markdown 编辑面板" onKeyDown={event=>{
    if(!(event.ctrlKey||event.metaKey)||event.altKey||event.nativeEvent.isComposing)return;
    const key=event.key.toLowerCase();if(key==='s'){event.preventDefault();event.stopPropagation();void s.save(event.shiftKey);}
    if(source&&(key==='z'||key==='y')){event.preventDefault();event.stopPropagation();if(key==='y'||event.shiftKey)s.redoText();else s.undoText();}
  }}>
    <header className="markdown-panel-header"><DocumentFileName name={file} path={s.path} disabled={busy}/><span className="text-xs text-muted" role="status">{documentDirty(s)?'未保存修改':'已保存'}</span><button className="icon-button" title="关闭 Markdown" aria-label="关闭文档" disabled={busy} onClick={()=>void s.request('close')}><X size={16}/></button></header>
    <div className="markdown-panel-actions"><ClipboardImportButton markdown/><button className="icon-button" aria-label="新建模板文档" title="新建模板文档" disabled={busy} onClick={()=>s.setTemplateDialog('choose')}><BookTemplate size={15}/></button><button className="icon-button" aria-label={outline?'隐藏大纲':'显示大纲'} title={outline?'隐藏大纲':'显示大纲'} aria-pressed={outline} onClick={()=>setOutline(!outline)}><ListTree size={15}/></button><button className="icon-button" aria-label="保存为模板" title="保存为模板" disabled={busy} onClick={()=>s.setTemplateDialog('save')}><BookmarkPlus size={15}/></button><button className="secondary-button" aria-label="保存 Markdown" title="保存 Markdown (Ctrl+S)" disabled={busy} onClick={()=>void s.save()}><Save size={14}/><span className="document-action-label">保存 Markdown</span></button><button className="secondary-button" aria-label="另存为 Markdown" title="另存为 Markdown" disabled={busy} onClick={()=>void s.save(true)}><FileOutput size={14}/><span className="document-action-label">另存为 Markdown</span></button><button className="secondary-button" aria-label={preview?'继续编辑':'预览 Markdown'} title={preview?'继续编辑':'预览 Markdown'} aria-pressed={preview} onClick={()=>setPreview(!preview)}><Eye size={14}/><span className="document-action-label">{preview?'继续编辑':'预览 Markdown'}</span></button>{source?<><button className="icon-button" aria-label="撤销 Markdown" disabled={busy||!s.past.length} onClick={s.undoText}><Undo2 size={15}/></button><button className="icon-button" aria-label="重做 Markdown" disabled={busy||!s.future.length} onClick={s.redoText}><Redo2 size={15}/></button></>:<button className="secondary-button" disabled={busy} title="转为普通 Markdown 文本后，可直接编辑全文；图文顺序和图片资源保留。" onClick={s.toSource}>编辑 Markdown 源文</button>}</div>
    {s.error&&<p role="alert" className="error-banner m-2">{s.error}</p>}{s.notice&&<p role="status" title={s.notice} className="markdown-notice m-2 text-xs text-muted">{s.notice}</p>}
    <div className="markdown-preview-options">{source&&<button aria-pressed={live} className="text-xs" onClick={()=>setLive(!live)}>{live?'关闭实时预览':'开启实时预览'}</button>}<span>LaTeX · 离线排版</span><ExternalOpenButton/></div>
    <div className="markdown-reference-row"><DocumentProject/><ReferenceButton/></div>
    <div className="markdown-editing-workspace">{outline&&<nav className="markdown-outline" aria-label="文档大纲">{headings.length?headings.map(h=><button key={h.offset} type="button" title={`H${h.depth} · 第 ${h.line} 行 · ${h.text}`} disabled={busy} data-outline-offset={h.offset} className="outline-heading" style={{paddingLeft:8+(h.depth-1)*10}} onClick={()=>jump(h)}>{h.text}</button>):<p className="p-3 text-xs text-muted">暂无标题。使用 # 至 ###### 创建大纲。</p>}</nav>}<div className="markdown-editing-content">
    {preview?<div ref={previewBody} className="markdown-panel-body"><MarkdownPreview markdown={markdown} assets={s.assets} onInternalOpen={internalOpen}/></div>:source?<textarea ref={input} aria-label="Markdown 源文" className="markdown-source" spellCheck={false} disabled={busy} placeholder="在这里输入 Markdown…" value={sourceText(document)} onChange={e=>s.editSource(e.target.value,e.target.selectionStart,e.target.selectionEnd)} onBlur={e=>s.setCaret(e.currentTarget.selectionStart,e.currentTarget.selectionEnd)} onSelect={e=>s.setCaret(e.currentTarget.selectionStart,e.currentTarget.selectionEnd)}/>:<div className="markdown-panel-body"><Suspense fallback={<p role="status">正在加载图文编排…</p>}><DocumentView/></Suspense></div>}
    {trigger&&!busy&&<LinkPicker key={`${trigger.start}:${trigger.query}`} trigger={trigger} input={input}/>}
    {!preview&&source&&live&&<div className="markdown-live-preview" aria-label="实时预览区域"><MarkdownPreview markdown={liveMarkdown} assets={s.assets} onInternalOpen={internalOpen}/></div>}
    </div></div>
    {currentId&&<LinkDetails key={currentId} kind="document" id={currentId} savedVersion={s.savedFingerprint}/>}
    <footer className="markdown-panel-footer">{busy?'正在处理文件…':'Markdown · UTF-8 · Ctrl+S 保存'}{s.path&&<span title={s.path}>{s.path}</span>}</footer>
  </section>;
}
