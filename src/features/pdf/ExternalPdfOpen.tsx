import {useEffect,useRef,useState} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {workspaceManager} from '../workspace/manager';
import {errorMessage} from '../../shared/api/errors';
interface Offer{id:string;title:string}
function offers(value:unknown):Offer[]{if(!Array.isArray(value)||value.length>16||value.some(v=>!v||typeof v!=='object'||!/^[a-f0-9]{32}$/.test(v.id)||typeof v.title!=='string'||v.title.length>160))throw Error('外部PDF请求格式不受支持。');return value;}
/** No path input: only native argv offers can enter the external-open route. */
export function ExternalPdfOpen(){const ready=workspaceManager.state(s=>s.ready),[pending,setPending]=useState<Offer[]>([]),[error,setError]=useState<string|null>(null),retry=useRef<()=>void>(()=>{});
 useEffect(()=>{if(!ready||!isTauri())return;let stopped=false,running=false,again=false;let unlisten:(()=>void)|null=null;const count=()=>[...workspaceManager.sessions.values()].filter(s=>s.panel.kind==='pdf').length;let previous=count();
  const drain=async()=>{if(stopped)return;if(running){again=true;return;}running=true;
   try{let queue=offers(await invoke('pending_pdf_requests'));if(stopped)return;setPending(queue);setError(null);while(queue.length&&count()<3&&!stopped){const {openPdf}=await import('./PdfViewerSession');if(stopped)return;await openPdf(undefined,queue[0].id);queue=offers(await invoke('pending_pdf_requests'));if(!stopped)setPending(queue);}}
   catch(e){if(!stopped)setError(errorMessage(e,'外部PDF接收失败；可重试或使用打开PDF。'));}finally{running=false;if(again&&!stopped){again=false;void drain();}}
  };
  retry.current=()=>void drain();const unsubscribe=workspaceManager.state.subscribe(()=>{const now=count();if(now<previous)void drain();previous=now;});
  void listen('pdf-open-requested',()=>void drain()).then(stop=>{if(stopped)stop();else{unlisten=stop;void drain();}}).catch(e=>{if(!stopped)setError(errorMessage(e,'外部PDF通知监听失败。'));});
  return()=>{stopped=true;unlisten?.();unsubscribe();retry.current=()=>{};};
 },[ready]);
 return pending.length||error?<aside className="workspace-layout-message external-pdf-requests" aria-label="外部PDF请求">{error?<span role="alert">{error}</span>:<span>外部PDF待打开 {pending.length} 份。已打开3份时，请先关闭一个PDF标签。</span>}<button onClick={()=>retry.current()}>重试接收</button>{pending.map(item=><button key={item.id} title={item.title} onClick={()=>{void invoke('dismiss_pdf_request',{id:item.id}).then(()=>retry.current()).catch(e=>setError(errorMessage(e,'取消外部请求失败。')));}}>取消 {item.title}</button>)}</aside>:null;
}
