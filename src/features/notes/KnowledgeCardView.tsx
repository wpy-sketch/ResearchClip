import {LinkDetails} from '../links/LinkDetails';
import {MathOcrButton} from '../math-ocr/MathOcr';
import {ReferenceButton} from '../references/References';
import {SavedOcrButton} from '../ocr-index/OcrIndex';
import {useEffect,useState} from 'react';
import {ArrowLeft,ExternalLink,ImageOff,Save,Star,X} from 'lucide-react';
import {useCardStore,isDirty} from './store';
import {useTagStore} from '../tags/store';
import {screenshotImage,openCardSource} from '../../shared/api/library';
import {errorMessage} from '../../shared/api/errors';
import {useUiStore} from '../../app/ui-store';
import {openLibraryEditor} from '../../app/library-actions';
import {Modal} from '../../shared/ui/Modal';
import {normalizeTagName} from './model';
import {MarkdownExportButton} from '../export/MarkdownExport';
import {useMarkdownExportStore} from '../export/store';

function CardImage({id,title}:{id:string;title:string}){
  const [attempt,setAttempt]=useState(0),[resource,setResource]=useState<{key:string;url?:string;error?:string}|null>(null);
  const key=`${id}:${attempt}`;
  useEffect(()=>{let active=true,url:string|undefined;void screenshotImage(id).then(blob=>{if(active){url=URL.createObjectURL(blob);setResource({key,url});}}).catch(e=>{if(active)setResource({key,error:errorMessage(e,'图片读取失败，笔记仍可编辑。')});});return()=>{active=false;if(url)URL.revokeObjectURL(url);};},[id,key]);
  const current=resource?.key===key?resource:null;
  return <div className="knowledge-image">{current?.url&&!current.error?<img src={current.url} alt={`${title}图片预览`} onError={()=>setResource({key,error:'图片无法显示，资料已保留。'})}/>:<div className="knowledge-image-empty"><ImageOff size={28}/><p>{current?.error??'正在读取图片…'}</p>{current?.error&&<button className="secondary-button" onClick={()=>setAttempt(n=>n+1)}>重试图片</button>}</div>}</div>;
}
const format=(stamp:number)=>new Date(stamp).toLocaleString('zh-CN');
export function KnowledgeCardView(){
  const exporting=useMarkdownExportStore(s=>s.busy);
  const {activeId,sessions,edit,flush,reload,deletingIds}=useCardStore(),tags=useTagStore(s=>s.items);
  const [tagInput,setTagInput]=useState(''),[actionError,setActionError]=useState<string|null>(null),[confirmReload,setConfirmReload]=useState(false);
  const session=activeId?sessions[activeId]:undefined;
  useEffect(()=>{void useTagStore.getState().refresh();},[]);
  if(!session?.card||!session.draft)return <div className="page-container"><button className="secondary-button" onClick={()=>useUiStore.getState().navigate('screenshots')}>返回素材库</button><h2 className="mt-5 text-lg">科研知识卡片</h2>{session?.error?<><p role="alert" className="error-banner mt-4">{session.error}</p><button className="secondary-button mt-3" onClick={()=>activeId&&void reload(activeId)}>重试读取卡片</button></>:<p role="status" className="mt-4 text-sm text-muted">正在读取卡片…</p>}</div>;
  const {card,draft}=session,id=session.id,disabled=session.status==='loading'||deletingIds.includes(id)||exporting,dirty=isDirty(session);
  const action=async(fn:()=>Promise<unknown>)=>{setActionError(null);try{await fn();}catch(error){setActionError(errorMessage(error,'操作失败，已有内容已保留。'));}};
  const saveThen=async(fn:()=>Promise<unknown>)=>{if(await flush(id))await fn();};
  const addTag=()=>{const name=tagInput.trim();if(!name||Array.from(name).length>50||draft.tagNames.some(t=>normalizeTagName(t)===normalizeTagName(name))){setActionError('标签须为1至50个字符，且不能与已添加标签重名。');return;}if(draft.tagNames.length>=100){setActionError('每张卡片最多100个标签。');return;}edit(id,{tagNames:[...draft.tagNames,name]});setTagInput('');setActionError(null);};
  return <div className="page-container knowledge-page">
    <div className="knowledge-heading"><button className="secondary-button" onClick={()=>{void flush(id);useUiStore.getState().navigate('screenshots');}}><ArrowLeft size={14}/>返回素材库</button><h2>科研知识卡片</h2><div className="flex flex-wrap gap-2"><MathOcrButton image={()=>screenshotImage(id)} name={draft.title} libraryId={id} disabled={disabled}/><SavedOcrButton id={id}/><MarkdownExportButton ids={[id]} disabled={disabled}/><button className="secondary-button" disabled={disabled} onClick={()=>void action(()=>saveThen(()=>openLibraryEditor(id)))}>编辑图片标注</button><button className="secondary-button" disabled={disabled||session.status==='saving'} onClick={()=>void flush(id)}><Save size={14}/>保存卡片</button></div></div>
    <div className="knowledge-layout">
      <section className="knowledge-preview" aria-label="卡片图片"><CardImage key={id} id={id} title={draft.title}/><div className="knowledge-times"><span>原图 {card.screenshot.widthPx} × {card.screenshot.heightPx} px</span><span>创建：{format(card.screenshot.createdAt)}</span><span>修改：{format(card.screenshot.updatedAt)}</span></div></section>
      <section className="knowledge-fields" aria-label="笔记编辑区">
        <div className="flex flex-wrap items-center gap-2"><ReferenceButton screenshotId={id}/><span className="text-xs text-muted">文献关联独立保存，不会替换笔记正文</span></div>
        <div className="flex items-end gap-2"><label className="min-w-0 flex-1">标题<input aria-label="卡片标题" value={draft.title} maxLength={200} disabled={disabled} onChange={e=>edit(id,{title:e.target.value})}/></label><button className="icon-button mb-1" aria-label="收藏卡片" aria-pressed={draft.isFavorite} disabled={disabled} onClick={()=>edit(id,{isFavorite:!draft.isFavorite})}><Star size={18} fill={draft.isFavorite?'currentColor':'none'}/></button></div>
        <label>Markdown 笔记<textarea aria-label="Markdown 笔记正文" value={draft.content} spellCheck={false} disabled={disabled} maxLength={1_048_576} placeholder="记录观察、实验方法、文献要点或下一步思考…" onChange={e=>edit(id,{content:e.target.value})}/></label>
        <div className="knowledge-save-state" role={session.error?'alert':'status'}>{session.error??(session.status==='saving'?'正在保存…':dirty?'等待自动保存…':'已保存到本机')}<span>停止输入约 0.7 秒后自动保存 · Ctrl+S 立即保存</span></div>
        <div className="flex items-end gap-2"><label className="min-w-0 flex-1">来源 URL<input aria-label="来源 URL" value={draft.sourceUrl} disabled={disabled} maxLength={2048} placeholder="https://…" onChange={e=>edit(id,{sourceUrl:e.target.value})}/></label><button className="secondary-button mb-0.5" disabled={disabled||!draft.sourceUrl.trim()} onClick={()=>void action(()=>saveThen(()=>openCardSource(id)))}><ExternalLink size={14}/>打开来源</button></div>
        <div><p className="text-xs text-muted">标签</p><div className="knowledge-tags">{draft.tagNames.map(name=><span key={name}>{name}<button aria-label={`移除标签 ${name}`} disabled={disabled} onClick={()=>edit(id,{tagNames:draft.tagNames.filter(t=>t!==name)})}><X size={12}/></button></span>)}{!draft.tagNames.length&&<p className="text-xs text-faint">添加课题、方法或文献分类标签。</p>}</div>
          <form className="knowledge-tag-input" onSubmit={e=>{e.preventDefault();addTag();}}><input aria-label="添加卡片标签" list="knowledge-tag-options" value={tagInput} maxLength={50} disabled={disabled} placeholder="输入或选择标签" onChange={e=>setTagInput(e.target.value)}/><datalist id="knowledge-tag-options">{tags.map(t=><option key={t.id} value={t.name}/>)}</datalist><button className="secondary-button" disabled={disabled||!tagInput.trim()}>添加标签</button></form>
        </div>
        {card.additionalNoteCount>0&&<p className="text-xs text-muted">另外 {card.additionalNoteCount} 条既有笔记已保留；本页编辑最早创建的笔记。</p>}
        {actionError&&<p className="error-banner" role="alert">{actionError}</p>}
        <button className="knowledge-reload" disabled={disabled||session.status==='saving'} onClick={()=>{if(dirty)setConfirmReload(true);else void reload(id);}}>重新加载已保存内容</button>
      </section>
    </div>
    <LinkDetails kind="screenshot" id={id}/>
    {confirmReload&&<Modal title="重新加载卡片" onClose={()=>setConfirmReload(false)}><div className="p-5"><p className="text-sm">重新加载会用已保存内容替换当前卡片草稿。请先保留尚未保存的文字。</p><div className="mt-4 flex justify-end gap-2"><button className="secondary-button" onClick={()=>setConfirmReload(false)}>保留草稿</button><button className="secondary-button" onClick={()=>{setConfirmReload(false);void reload(id);}}>确认重新加载</button></div></div></Modal>}
  </div>;
}
