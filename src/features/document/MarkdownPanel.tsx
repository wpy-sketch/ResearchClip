import {lazy,Suspense,useEffect,useRef,useState} from 'react';
import {Save,X,Undo2,Redo2,FileOutput,Eye} from 'lucide-react';
import {useDocumentStore,documentDirty} from './store';
import {serializeDocument,sourceText} from './model';
import {MarkdownPreview} from './MarkdownPreview';
import {DocumentFileName} from './DocumentFileName';
const DocumentView=lazy(()=>import('./DocumentView').then(m=>({default:m.DocumentView})));
export function MarkdownPanel(){
  const s=useDocumentStore(),[preview,setPreview]=useState(false),input=useRef<HTMLTextAreaElement>(null),busy=s.phase!=='idle';
  const document=s.document,source=document?.mode==='source';
  useEffect(()=>{
    if(!source||preview||busy||!input.current)return;
    const node=input.current,nodeState=useDocumentStore.getState();node.focus();node.setSelectionRange(nodeState.caret.start,nodeState.caret.end);
  },[s.focusRequest,source,preview,busy]);
  if(!document)return null;
  let markdown='';try{markdown=serializeDocument(document);}catch{/* Save reports validation; edits remain available. */}
  const file=s.fileName??s.path?.split(/[\\/]/).at(-1)??'untitled.md';
  return <section className="markdown-panel" data-markdown-panel aria-label="Markdown 编辑面板" onKeyDown={event=>{
    if(!(event.ctrlKey||event.metaKey)||event.altKey||event.nativeEvent.isComposing)return;
    const key=event.key.toLowerCase();if(key==='s'){event.preventDefault();event.stopPropagation();void s.save(event.shiftKey);}
    if(source&&(key==='z'||key==='y')){event.preventDefault();event.stopPropagation();if(key==='y'||event.shiftKey)s.redoText();else s.undoText();}
  }}>
    <header className="markdown-panel-header"><DocumentFileName name={file} path={s.path} disabled={busy}/><span className="text-xs text-muted" role="status">{documentDirty(s)?'未保存修改':'已保存'}</span><button className="icon-button" title="关闭 Markdown" aria-label="关闭文档" disabled={busy} onClick={()=>void s.request('close')}><X size={16}/></button></header>
    <div className="markdown-panel-actions"><button className="secondary-button" aria-label="保存 Markdown" title="保存 Markdown (Ctrl+S)" disabled={busy} onClick={()=>void s.save()}><Save size={14}/><span className="document-action-label">保存 Markdown</span></button><button className="secondary-button" aria-label="另存为 Markdown" title="另存为 Markdown" disabled={busy} onClick={()=>void s.save(true)}><FileOutput size={14}/><span className="document-action-label">另存为 Markdown</span></button><button className="secondary-button" aria-label={preview?'继续编辑':'预览 Markdown'} title={preview?'继续编辑':'预览 Markdown'} aria-pressed={preview} onClick={()=>setPreview(!preview)}><Eye size={14}/><span className="document-action-label">{preview?'继续编辑':'预览 Markdown'}</span></button>{source?<><button className="icon-button" aria-label="撤销 Markdown" disabled={busy||!s.past.length} onClick={s.undoText}><Undo2 size={15}/></button><button className="icon-button" aria-label="重做 Markdown" disabled={busy||!s.future.length} onClick={s.redoText}><Redo2 size={15}/></button></>:<button className="secondary-button" disabled={busy} title="转为普通 Markdown 文本后，可直接编辑全文；图文顺序和图片资源保留。" onClick={s.toSource}>编辑 Markdown 源文</button>}</div>
    {s.error&&<p role="alert" className="error-banner m-2">{s.error}</p>}{s.notice&&<p role="status" title={s.notice} className="markdown-notice m-2 text-xs text-muted">{s.notice}</p>}
    {preview?<div className="markdown-panel-body"><MarkdownPreview markdown={markdown} assets={s.assets}/></div>:source?<textarea ref={input} aria-label="Markdown 源文" className="markdown-source" spellCheck={false} disabled={busy} placeholder="在这里输入 Markdown…" value={sourceText(document)} onChange={e=>s.editSource(e.target.value,e.target.selectionStart,e.target.selectionEnd)} onBlur={e=>s.setCaret(e.currentTarget.selectionStart,e.currentTarget.selectionEnd)} onSelect={e=>s.setCaret(e.currentTarget.selectionStart,e.currentTarget.selectionEnd)}/>:<div className="markdown-panel-body"><Suspense fallback={<p role="status">正在加载图文编排…</p>}><DocumentView/></Suspense></div>}
    <footer className="markdown-panel-footer">{busy?'正在处理文件…':'Markdown · UTF-8 · Ctrl+S 保存'}{s.path&&<span title={s.path}>{s.path}</span>}</footer>
  </section>;
}
