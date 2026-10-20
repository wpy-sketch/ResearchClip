import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {MousePointer2,ScanLine,X} from 'lucide-react';
import {errorMessage} from '../../shared/api/errors';
import {pickerRect,type WindowPickerInfo} from './window-picker';

export function WindowPicker({sessionId}:{sessionId:string}) {
  const [info,setInfo]=useState<WindowPickerInfo|null>(null),[error,setError]=useState<string|null>(null),[viewport,setViewport]=useState({width:innerWidth,height:innerHeight}),[submitting,setSubmitting]=useState(false);
  useEffect(()=>{
    let disposed=false,timer:ReturnType<typeof setTimeout>|undefined,ready=false;
    const refresh=async()=>{try{
      const next=await invoke<WindowPickerInfo>('window_picker_status',{sessionId});
      if(disposed)return;
      setInfo(next);
      if(!ready){await invoke('window_picker_action',{sessionId,action:'ready'});ready=true;}
    }catch(e){if(!disposed)setError(errorMessage(e,'无法读取窗口边界。'));}
    finally{if(!disposed)timer=setTimeout(()=>void refresh(),120);}};
    void refresh();
    const resize=()=>setViewport({width:innerWidth,height:innerHeight});window.addEventListener('resize',resize);
    return()=>{disposed=true;clearTimeout(timer);window.removeEventListener('resize',resize);};
  },[sessionId]);
  async function action(action:string){if(submitting)return;setError(null);if(action==='capture')setSubmitting(true);try{await invoke('window_picker_action',{sessionId,action});}catch(e){setError(errorMessage(e,'窗口选择失败。'));setSubmitting(false);}}
  useEffect(()=>{
    const key=(event:KeyboardEvent)=>{if(event.key==='Escape'){event.preventDefault();void invoke('window_picker_action',{sessionId,action:'cancel'}).catch(e=>setError(errorMessage(e,'无法取消窗口选择。')));}
      if(event.key==='Tab'){event.preventDefault();void invoke('window_picker_action',{sessionId,action:event.shiftKey?'previous':'next'}).catch(e=>setError(errorMessage(e,'无法切换窗口。')));}};
    window.addEventListener('keydown',key);return()=>window.removeEventListener('keydown',key);
  },[sessionId]);
  const selected=info?.selected,rect=info&&selected?pickerRect(selected.bounds,info.desktop,viewport.width,viewport.height):null;
  return <div className="window-picker" onPointerDown={event=>{if(event.button===0 && !(event.target as Element).closest('button'))void action('capture');}}>
    {rect&&<div className={`window-picker-outline ${selected?.blocked?'is-blocked':''}`} style={rect} aria-label="目标窗口边界"><span className="window-picker-dimensions">{selected?.bounds.width} × {selected?.bounds.height} px · {selected?.dpi} DPI</span></div>}
    <section className="window-picker-hint" role="status"><div className="window-picker-icon"><ScanLine size={22}/></div><div><strong>{submitting?'正在捕获窗口…':selected?.title??'移动鼠标，选择一个窗口'}</strong><p>{selected?.blocked?'此窗口受系统保护，无法截图。':`单击捕获 · Tab 切换 ${info?.candidateCount??0} 个候选窗口 · Esc 取消`}</p>{error&&<p className="window-picker-error" role="alert">{error}</p>}</div><button aria-label="取消窗口截图" title="取消（Esc）" onClick={()=>void action('cancel')}><X size={18}/></button><MousePointer2 size={16}/></section>
  </div>;
}
