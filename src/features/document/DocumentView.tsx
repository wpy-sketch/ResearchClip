import {useState} from 'react';
import {ArrowDown,ArrowUp,Plus,Trash2} from 'lucide-react';
import {useDocumentStore} from './store';
import {serializeDocument} from './model';
import {MarkdownPreview} from './MarkdownPreview';
import {LibraryPicker} from './LibraryPicker';

export function DocumentView(){
  const s=useDocumentStore(),[picker,setPicker]=useState(false),[preview,setPreview]=useState(false),busy=s.phase!=='idle';
  let markdown='';try{if(s.document)markdown=serializeDocument(s.document);}catch{/* Invalid input remains editable; save reports its validation error. */}
  return <div className="page-container document-page">
    {!s.document?<div className="rounded-xl border border-edge p-8 text-center text-muted">新建文档，添加已标注的素材图片；也可打开已保存的 .md 文件。</div>:<>
      <label className="mb-4 block text-sm">文档标题<input className="mt-2 w-full rounded border border-edge bg-surface p-3" aria-label="Markdown 文档标题" maxLength={200} value={s.document.title} disabled={busy} onChange={event=>s.editTitle(event.target.value)}/></label>
      <div className="mb-4 flex flex-wrap gap-2"><button className="secondary-button" disabled={busy} onClick={()=>setPicker(true)}><Plus size={14}/>添加图片</button><button className="secondary-button" disabled={busy} onClick={()=>s.addText()}>添加 Markdown 文本</button><button className="secondary-button" aria-pressed={preview} onClick={()=>setPreview(!preview)}>{preview?'隐藏预览':'实时预览'}</button></div>
      {preview&&<MarkdownPreview markdown={markdown} assets={s.assets}/>}<fieldset disabled={busy} className="space-y-4">
        <p className="text-xs text-muted">支持标题、粗体、列表、代码和 $行内公式$、$$独立公式$$。点击实时预览查看离线排版，LaTeX 源码原样保留。预览不执行 HTML、不加载远程图片或打开链接。</p>
        {s.document.blocks.map((b,index)=><article key={b.id} className="rounded-xl border border-edge p-4" data-block-id={b.id}>
          <div className="mb-3 flex items-center justify-between gap-2"><h3 className="text-sm font-medium">{index+1}. {b.type==='image'?b.alt:'Markdown 文本'}</h3><div className="flex gap-1"><button className="icon-button" aria-label={`上移内容块 ${index+1}`} disabled={index===0} onClick={()=>s.move(b.id,-1)}><ArrowUp size={15}/></button><button className="icon-button" aria-label={`下移内容块 ${index+1}`} disabled={index===s.document!.blocks.length-1} onClick={()=>s.move(b.id,1)}><ArrowDown size={15}/></button><button className="icon-button" aria-label={`删除内容块 ${index+1}`} onClick={()=>s.remove(b.id)}><Trash2 size={15}/></button></div></div>
          {b.type==='image'&&<img src={s.assets[b.assetName]?.url} alt={b.alt} className="mx-auto mb-4 max-h-96 max-w-full object-contain"/>}
          <label className="block text-xs text-muted">{b.type==='image'?'图片文字注释':'Markdown 文本'}<textarea data-markdown-target={b.id} onSelect={e=>s.setCaret(e.currentTarget.selectionStart,e.currentTarget.selectionEnd,b.id)} onBlur={e=>s.setCaret(e.currentTarget.selectionStart,e.currentTarget.selectionEnd,b.id)} className="mt-2 min-h-32 w-full resize-y rounded border border-edge bg-surface p-3 font-mono text-sm text-primary" aria-label={`${b.type==='image'?'图片注释':'Markdown 文本'} ${index+1}`} value={b.type==='image'?b.caption:b.markdown} onChange={event=>s.editBlock(b.id,event.target.value,event.target.selectionStart,event.target.selectionEnd)}/></label>
          <button className="secondary-button mt-3" onClick={()=>s.addText(b.id)}>在此块后插入文本</button>
        </article>)}
        {!s.document.blocks.length&&<p className="py-8 text-center text-sm text-muted">文档为空。添加图片或Markdown文本开始整理。</p>}
      </fieldset>
    </>}
    {picker&&s.document&&<LibraryPicker busy={busy} onClose={()=>setPicker(false)} onAdd={s.addImages} operationError={s.error}/>}
  </div>;
}
