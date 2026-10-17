import {lazy,Suspense,useEffect,useRef,useState,type ReactNode} from 'react';
import {useDocumentStore} from './store';
import {useSettingsStore} from '../settings/store';
import {splitPercent,splitWidth} from './split';
import {TemplateDialog} from './TemplateDialog';
import {DocumentConfirmation} from './DocumentConfirmation';
const MarkdownPanel=lazy(()=>import('./MarkdownPanel').then(m=>({default:m.MarkdownPanel})));
export function WorkspaceSplit({children}:{children:ReactNode}){
  const documentId=useDocumentStore(s=>s.document?.id),open=Boolean(documentId),error=useDocumentStore(s=>s.error),phase=useDocumentStore(s=>s.phase);
  const templateDialog=useDocumentStore(s=>s.templateDialog);
  const saved=useSettingsStore(s=>s.snapshot.preferences.markdownSplitPercent),[preferred,setPreferred]=useState<number|null>(null),[width,setWidth]=useState(0);
  const host=useRef<HTMLDivElement>(null),drag=useRef<{percent:number;pointer:number}|null>(null),percent=preferred??saved;
  useEffect(()=>{const node=host.current;if(!node)return;const observer=new ResizeObserver(()=>setWidth(node.clientWidth));observer.observe(node);return()=>observer.disconnect();},[]);
  useEffect(()=>{if(!open)drag.current=null;},[open]);
  const persist=(next:number)=>{setPreferred(next);void useSettingsStore.getState().change({markdownSplitPercent:next}).then(()=>setPreferred(useSettingsStore.getState().snapshot.preferences.markdownSplitPercent));};
  return <><div ref={host} className={`workspace-split ${open?'has-document':''}`} data-document-open={open}>
    <div className="original-workspace" style={open&&width?{flexBasis:splitWidth(width,percent)}:undefined}>{children}</div>
    {open&&<><div role="separator" aria-label="调整 Markdown 分栏宽度" aria-orientation="vertical" aria-valuemin={20} aria-valuemax={80} aria-valuenow={percent} tabIndex={0} className="markdown-divider"
      onPointerDown={event=>{if(event.button!==0)return;event.preventDefault();drag.current={percent,pointer:event.pointerId};event.currentTarget.setPointerCapture(event.pointerId);}}
      onPointerMove={event=>{if(!drag.current||!host.current)return;setPreferred(splitPercent(width,event.clientX-host.current.getBoundingClientRect().left));}}
      onPointerUp={event=>{if(!drag.current)return;drag.current=null;event.currentTarget.releasePointerCapture(event.pointerId);persist(percent);}}
      onPointerCancel={()=>{if(drag.current)setPreferred(drag.current.percent);drag.current=null;}}
      onLostPointerCapture={()=>{drag.current=null;}}
      onKeyDown={event=>{if(['ArrowLeft','ArrowRight','Home'].includes(event.key)){event.preventDefault();persist(event.key==='Home'?50:Math.max(20,Math.min(80,percent+(event.key==='ArrowLeft'?-2:2))));}if(event.key==='Escape'&&drag.current){setPreferred(drag.current.percent);drag.current=null;}}}/><Suspense fallback={<p role="status">正在加载 Markdown…</p>}><MarkdownPanel key={documentId}/></Suspense></>}
  </div>{!open&&error&&<p role="alert" className="error-banner">{error}</p>}{!open&&phase==='opening'&&<p role="status">正在等待系统 Markdown 文件对话框…</p>}<DocumentConfirmation/>{templateDialog&&<TemplateDialog key={templateDialog} mode={templateDialog}/>}</>;
}
