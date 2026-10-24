import {useEffect,useRef,useState} from 'react';
import {applyDomText,rawOffset,displayOffset} from './projection';
import {ArrowDown,ArrowUp,Plus,Trash2} from 'lucide-react';
import {useDocumentApi,useDocumentStore} from './store';
import {serializeDocument} from './model';
import {MarkdownPreview} from './MarkdownPreview';
import {textTarget} from './text-target';
import {LibraryPicker} from './LibraryPicker';

export function DocumentView({visual=false,inlinePreview=true}:{visual?:boolean;inlinePreview?:boolean}){
  const api=useDocumentApi(),s=useDocumentStore(),[picker,setPicker]=useState(false),[preview,setPreview]=useState(false),[active,setActive]=useState<string|null>(null),host=useRef<HTMLDivElement>(null),drag=useRef<{id:string;order:string[]}|null>(null),busy=s.phase!=='idle';
  useEffect(()=>{const doc=api.getState().document,caret=api.getState().caret,node=Array.from(host.current?.querySelectorAll<HTMLTextAreaElement>('textarea[data-markdown-target]')??[]).find(n=>n.dataset.markdownTarget===caret.blockId);if(node&&(!visual||active===caret.blockId)){const target=doc?textTarget(doc,node):null;if(!target)return;node.focus({preventScroll:true});node.setSelectionRange(displayOffset(target.raw,caret.start),displayOffset(target.raw,caret.end));}},[s.focusRequest,visual,active,api]);
  const move=(before:string|null)=>{const current=drag.current;drag.current=null;if(current)s.reorderBlocks(current.id,before,current.order);};
  let markdown='';try{if(s.document)markdown=serializeDocument(s.document);}catch{/* Invalid input remains editable; save reports its validation error. */}
  return <div ref={host} className="page-container document-page">
    {!s.document?<div className="rounded-xl border border-edge p-8 text-center text-muted">新建文档，添加已标注的素材图片；也可打开已保存的 .md 文件。</div>:<>
      <label className="mb-4 block text-sm">文档标题<input className="mt-2 w-full rounded border border-edge bg-surface p-3" aria-label="Markdown 文档标题" maxLength={200} value={s.document.title} disabled={busy} onChange={event=>s.editTitle(event.target.value)}/></label>
      <div className="mb-4 flex flex-wrap gap-2"><button className="secondary-button" disabled={busy} onClick={()=>setPicker(true)}><Plus size={14}/>添加图片</button><button className="secondary-button" disabled={busy} onClick={()=>s.addText()}>添加 Markdown 文本</button>{inlinePreview&&<button className="secondary-button" aria-pressed={preview} onClick={()=>setPreview(!preview)}>{preview?'隐藏预览':'实时预览'}</button>}</div>
      {preview&&<MarkdownPreview markdown={markdown} assets={s.assets}/>}<fieldset disabled={busy} className="space-y-4">
        <p className="text-xs text-muted">支持标题、粗体、列表、代码和 $行内公式$、$$独立公式$$。点击实时预览查看离线排版，LaTeX 源码原样保留。预览不执行 HTML、不加载远程图片或打开链接。</p>
        {s.document.blocks.map((b,index)=><article key={b.id} className="rounded-xl border border-edge p-4" data-block-id={b.id}><div className="md-block-drop" data-structured-drop={b.id} onDragOver={e=>{if(drag.current)e.preventDefault();}} onDrop={e=>{if(drag.current){e.preventDefault();e.stopPropagation();move(b.id);}}}/><button type="button" className="md-drag-structured" draggable={!busy} aria-label={`拖动图文块 ${index+1}`} title="整体移动图片和注释" onDragStart={e=>{drag.current={id:b.id,order:s.document!.blocks.map(item=>item.id)};e.dataTransfer.setData('application/x-researchclip-markdown','structured-block');e.dataTransfer.effectAllowed='move';}} onDragEnd={()=>{drag.current=null;}}>⠿</button>
          <div className="mb-3 flex items-center justify-between gap-2"><h3 className="text-sm font-medium">{index+1}. {b.type==='image'?b.alt:'Markdown 文本'}</h3><div className="flex gap-1"><button className="icon-button" aria-label={`上移内容块 ${index+1}`} disabled={index===0} onClick={()=>s.move(b.id,-1)}><ArrowUp size={15}/></button><button className="icon-button" aria-label={`下移内容块 ${index+1}`} disabled={index===s.document!.blocks.length-1} onClick={()=>s.move(b.id,1)}><ArrowDown size={15}/></button><button className="icon-button" aria-label={`删除内容块 ${index+1}`} onClick={()=>s.remove(b.id)}><Trash2 size={15}/></button></div></div>
          {b.type==='image'&&<img src={s.assets[b.assetName]?.url} alt={b.alt} className="mx-auto mb-4 max-h-96 max-w-full object-contain"/>}
          {visual&&active!==b.id?<div className="md-structured-preview" role="button" tabIndex={0} aria-label={`编辑图文块 ${index+1}`} onClick={e=>{if((e.target as Element).closest('button,a,input,select'))return;s.setCaret(0,0,b.id);setActive(b.id);}} onKeyDown={e=>{if((e.target as Element).closest('button,a,input,select'))return;if(e.key==='Enter'){s.setCaret(0,0,b.id);setActive(b.id);}}}><MarkdownPreview markdown={(b.type==='image'?b.caption:b.markdown)||'（点击输入文字）'} assets={s.assets}/></div>:<label className="block text-xs text-muted">{b.type==='image'?'图片文字注释':'Markdown 文本'}<textarea data-markdown-target={b.id} onSelect={e=>s.setCaret(rawOffset(b.type==='image'?b.caption:b.markdown,e.currentTarget.selectionStart),rawOffset(b.type==='image'?b.caption:b.markdown,e.currentTarget.selectionEnd),b.id)} onBlur={e=>s.setCaret(rawOffset(b.type==='image'?b.caption:b.markdown,e.currentTarget.selectionStart),rawOffset(b.type==='image'?b.caption:b.markdown,e.currentTarget.selectionEnd),b.id)} className="mt-2 min-h-32 w-full resize-y rounded border border-edge bg-surface p-3 font-mono text-sm text-primary" aria-label={`${b.type==='image'?'图片注释':'Markdown 文本'} ${index+1}`} value={b.type==='image'?b.caption:b.markdown} onChange={event=>{const edit=applyDomText(b.type==='image'?b.caption:b.markdown,event.target.value,event.target.selectionStart,event.target.selectionEnd);s.editBlock(b.id,edit.text,edit.start,edit.end);}}/></label>}{visual&&active===b.id&&<button type="button" className="secondary-button" onClick={()=>setActive(null)}>完成块编辑</button>}
          <button className="secondary-button mt-3" onClick={()=>s.addText(b.id)}>在此块后插入文本</button>
        </article>)}
        {!s.document.blocks.length&&<p className="py-8 text-center text-sm text-muted">文档为空。添加图片或Markdown文本开始整理。</p>}
        <div className="md-drop-end" data-structured-drop="end" onDragOver={e=>{if(drag.current)e.preventDefault();}} onDrop={e=>{if(drag.current){e.preventDefault();e.stopPropagation();move(null);}}}>拖到这里放在文末</div>
      </fieldset>
    </>}
    {picker&&s.document&&<LibraryPicker busy={busy} onClose={()=>setPicker(false)} onAdd={s.addImages} operationError={s.error}/>}
  </div>;
}
