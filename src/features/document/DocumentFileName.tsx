import {useEffect,useRef,useState} from 'react';
import {Modal} from '../../shared/ui/Modal';
import {normalizeMarkdownName} from './file-name';
import {useDocumentStore} from './store';

function RenameDialog({fileName,onClose}:{fileName:string;onClose:()=>void}){
  const [name,setName]=useState(fileName),input=useRef<HTMLInputElement>(null);
  const busy=useDocumentStore(s=>s.phase!=='idle'),error=useDocumentStore(s=>s.error);
  useEffect(()=>{input.current?.setSelectionRange(0,/\.md$/i.test(fileName)?fileName.length-3:fileName.length);},[fileName]);
  let normalized='',invalid='';try{normalized=normalizeMarkdownName(name);}catch(error){invalid=error instanceof Error?error.message:'文件名无效。';}
  return <Modal title="重命名 Markdown 文件" onClose={()=>{if(!busy)onClose();}}><form className="p-5" onKeyDown={event=>{event.stopPropagation();if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='s')event.preventDefault();}} onSubmit={event=>{
    event.preventDefault();if(!busy&&!invalid)void useDocumentStore.getState().rename(name).then(ok=>{if(ok)onClose();});
  }}><label className="block text-sm">文件名称<input ref={input} data-initial-focus aria-label="Markdown 文件名称" className="mt-2 w-full rounded border border-edge bg-surface p-3" value={name} disabled={busy} maxLength={255} onChange={event=>setName(event.target.value)}/></label>
    <p className="my-3 text-xs text-muted">{invalid||`新名称：${normalized}`}。文件名与正文标题独立，图片资源目录保持不变。</p>
    {error&&<p role="alert" className="error-banner">{error}</p>}<div className="mt-4 flex justify-end gap-2"><button type="button" className="secondary-button" disabled={busy} onClick={onClose}>取消</button><button type="submit" className="secondary-button" disabled={busy||Boolean(invalid)}>确认重命名</button></div></form></Modal>;
}
export function DocumentFileName({name,path,disabled}:{name:string;path:string|null;disabled:boolean}){
  const [menu,setMenu]=useState(false),[dialog,setDialog]=useState(false),root=useRef<HTMLDivElement>(null),button=useRef<HTMLButtonElement>(null),item=useRef<HTMLButtonElement>(null);
  useEffect(()=>{if(!menu)return;item.current?.focus();const outside=(event:PointerEvent)=>{if(event.target instanceof Node&&!root.current?.contains(event.target))setMenu(false);};const key=(event:KeyboardEvent)=>{if(event.key==='Escape'){event.preventDefault();setMenu(false);button.current?.focus();}};window.addEventListener('pointerdown',outside);window.addEventListener('keydown',key);return()=>{window.removeEventListener('pointerdown',outside);window.removeEventListener('keydown',key);};},[menu]);
  return <div className="markdown-name-control" ref={root}><button ref={button} type="button" className="markdown-file-name" disabled={disabled} title={`${path??name}（右键重命名）`} aria-label={`文件名：${name}`} aria-haspopup="menu" aria-expanded={menu} onClick={()=>setMenu(!menu)} onContextMenu={event=>{event.preventDefault();if(!disabled)setMenu(true);}}>{name}</button>
    {menu&&<div role="menu" aria-label="Markdown 文件菜单" className="markdown-name-menu"><button ref={item} role="menuitem" className="secondary-button" disabled={disabled} onClick={()=>{setMenu(false);setDialog(true);}}>重命名文件</button></div>}
    {dialog&&<RenameDialog fileName={name} onClose={()=>{setDialog(false);requestAnimationFrame(()=>button.current?.focus());}}/>}
  </div>;
}
