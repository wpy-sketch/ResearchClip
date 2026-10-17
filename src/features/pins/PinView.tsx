import { useEffect, useRef, useState } from 'react';
import { X } from 'lucide-react';
import { loadPin, pinAction } from './api';
import type { PinAction, PinInfo } from './api';
import { errorMessage } from '../../shared/api/errors';

/** A dedicated native WebView, deliberately never mounts App or its listeners/stores. */
export function PinView() {
  const [url,setUrl]=useState<string|null>(null);
  const [info,setInfo]=useState<PinInfo|null>(null);
  const [error,setError]=useState<string|null>(null);
  const [notice,setNotice]=useState<string|null>(null);
  const [menu,setMenu]=useState(false);
  const [busy,setBusy]=useState(false);
  const locked=useRef(false),disposed=useRef(false),wheelPending=useRef(false);
  const surface=useRef<HTMLDivElement>(null);
  const action=async (name:PinAction,value?:number)=>{
    if(locked.current)return;
    locked.current=true;setBusy(true);setError(null);setNotice(null);
    try { const next=await pinAction(name,value); if(!disposed.current){setInfo(next);if(name==='copy')setNotice('图片已复制');} }
    catch(e){if(!disposed.current)setError(errorMessage(e,'贴图操作失败，请重试。'));}
    finally{locked.current=false;if(!disposed.current)setBusy(false);}
  };
  useEffect(()=>{
    disposed.current=false;let owned:string|null=null;
    void loadPin().then(value=>{owned=value;if(disposed.current)URL.revokeObjectURL(value);else setUrl(value);}).catch(e=>setError(errorMessage(e,'贴图加载失败，请关闭后重新创建。')));
    return()=>{disposed.current=true;if(owned)URL.revokeObjectURL(owned);};
  },[]);
  useEffect(()=>{
    const element=surface.current;
    const wheel=(event:WheelEvent)=>{
      event.preventDefault();
      if(event.deltaY===0||wheelPending.current||locked.current||event.target instanceof HTMLElement&&event.target.closest('[role="menu"]'))return;
      wheelPending.current=true;
      void action('zoom',event.deltaY<0?1.12:1/1.12).finally(()=>{wheelPending.current=false;});
    };
    element?.addEventListener('wheel',wheel,{passive:false});
    return()=>element?.removeEventListener('wheel',wheel);
  },[]);
  return <div ref={surface} className="desktop-pin" onContextMenu={e=>{e.preventDefault();setMenu(true);}} onKeyDown={e=>{if(e.key==='Escape'){e.preventDefault();if(menu)setMenu(false);else void action('close');}}} tabIndex={-1}>
    {url&&<img src={url} alt="桌面贴图" draggable={false} onLoad={()=>void action('ready').then(()=>surface.current?.focus())} onError={()=>setError('贴图图像无法显示，请关闭后重新创建。')} onMouseDown={e=>{if(e.button===0&&!menu){e.preventDefault();surface.current?.focus();void action('drag');}}}/>}
    <button className="pin-close" aria-label="关闭贴图" title="关闭贴图" disabled={busy} onClick={()=>void action('close')}><X size={15}/></button>
    <div className="pin-hint">拖动移动 · 滚轮缩放 · 右键菜单</div>
    {menu&&<div className="pin-menu" role="menu" aria-label="贴图菜单" onKeyDown={e=>{if(e.key==='Escape')setMenu(false);}}>
      <button role="menuitem" disabled={busy} onClick={()=>void action('copy')}>复制图片</button>
      <button role="menuitem" disabled={busy} onClick={()=>void action('save')}>另存为 PNG</button>
      <label>不透明度 {info?.opacity??100}%<input type="range" aria-label="贴图不透明度" min={20} max={100} value={info?.opacity??100} disabled={busy} onChange={e=>void action('opacity',Number(e.target.value))}/></label>
      <button role="menuitem" disabled={busy} onClick={()=>void action('recover')}>恢复可见位置 / 原始大小</button>
      <button role="menuitem" disabled={busy} onClick={()=>void action('close')}>关闭贴图</button>
      <button role="menuitem" onClick={()=>setMenu(false)}>收起菜单</button>
    </div>}
    {error&&<div className="pin-message" role="alert">{error}<button onClick={()=>void action('close')}>关闭贴图</button></div>}
    {notice&&<div className="pin-message" role="status" onClick={()=>setNotice(null)}>{notice}</div>}
  </div>;
}
