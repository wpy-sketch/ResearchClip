import {useEffect,useState} from 'react';
import {ScanLine,RefreshCw,Play,Square,X,ChevronDown} from 'lucide-react';
import {Modal} from '../../shared/ui/Modal';
import {useScrollStore,terminal} from './scroll-store';
import {scrollStatus,scrollAction,type ScrollProgress} from './scroll-api';
import {errorMessage} from '../../shared/api/errors';
export function ScrollCaptureButton({disabled=false}:{disabled?:boolean}){return <button className="secondary-button" disabled={disabled} onClick={()=>void useScrollStore.getState().open()}><ScanLine size={15}/>滚动长截图</button>;}
export function ScrollCaptureDialog(){
 const s=useScrollStore(),[target,setTarget]=useState(''),[auto,setAuto]=useState(false);
 const visible=s.visible,sessionId=s.progress?.sessionId,state=s.progress?.state;
 useEffect(()=>{if(!visible||!sessionId||!state||terminal(state))return;const timer=setInterval(()=>void useScrollStore.getState().poll(),500);return()=>clearInterval(timer);},[visible,sessionId,state]);
 if(!s.visible)return null;
 const selected=s.targets.find(t=>t.id===target),p=s.progress,active=p&&!terminal(p.state);
 return <Modal title="滚动长截图 · 浏览器 MVP" onClose={()=>void s.close()}><div className="scroll-capture-dialog">
  <div className="scroll-capture-steps"><span>1 选窗口</span><span>2 框选正文</span><span>3 滚动拼接</span></div>
  <p className="text-sm text-secondary">优先支持 Chrome / Edge 普通长网页。请先打开目标页面并回到希望截取的起点，再明确选择窗口；不会自动滚动其他后台窗口。</p>
  <label>目标窗口<select aria-label="长截图目标窗口" value={target} disabled={s.busy||Boolean(active)} onChange={e=>{setTarget(e.target.value);setAuto(false);}}><option value="">请选择窗口</option>{s.targets.map(t=><option key={t.id} value={t.id}>{t.title}{t.automatic?' · 浏览器自动模式可用':' · 仅手动'}</option>)}</select></label>
  <label className="scroll-mode"><input type="checkbox" checked={auto} disabled={!selected?.automatic||s.busy||Boolean(active)} onChange={e=>setAuto(e.target.checked)}/>自动向下滚动（只支持已选择的 Chrome / Edge 窗口）</label>
  <p className="text-xs text-muted">框选网页正文，排除地址栏、滚动条、悬浮侧栏及固定底栏。固定顶栏最多约220像素；复杂悬浮内容请缩小选区。手动模式每次滚动约半屏，等待稳定后采集下一段。</p>
  <div className="flex flex-wrap gap-2"><button className="secondary-button" disabled={s.busy||Boolean(active)} onClick={()=>void s.open()}><RefreshCw size={14}/>刷新窗口</button><button className="capture-button" disabled={!selected||s.busy||Boolean(active)} onClick={()=>void s.prepare(target,auto)}><ScanLine size={14}/>{s.busy?'正在准备…':'框选滚动区域'}</button>{p?.state==='ready'&&<button className="capture-button" disabled={s.busy} onClick={()=>void s.start()}><Play size={14}/>开始</button>}</div>
  {p&&<div className="scroll-progress" role="status"><strong>{p.width} × {p.height} 像素 · {p.frames} 段</strong><p>{p.message}</p>{active&&p.state!=='ready'&&<button className="secondary-button" onClick={()=>void s.action('stop')}><Square size={14}/>停止并生成长图</button>}</div>}
  {s.error&&<p className="error-banner" role="alert">{s.error}</p>}
  <p className="text-xs text-muted">最多64帧、180秒、30,000像素高且总计不超过16,777,216像素。到上限后分段保存；连续不变仅视为疑似底部，请核对完整性。</p>
  <button className="secondary-button" disabled={s.busy} onClick={()=>void s.close()}><X size={14}/>{p?.state==='done'?'查看长图预览':'取消 / 关闭'}</button>
 </div></Modal>;
}
export function ScrollController({sessionId}:{sessionId:string}){
 const [p,setP]=useState<ScrollProgress|null>(null),[error,setError]=useState<string|null>(null);
 useEffect(()=>{let alive=true;const poll=()=>void scrollStatus(sessionId).then(v=>{if(alive)setP(v);}).catch(e=>{if(alive)setError(errorMessage(e,'读取进度失败。'));});poll();const timer=setInterval(poll,400);const key=(e:KeyboardEvent)=>{if(e.key==='Escape')void scrollAction(sessionId,'cancel').catch(e=>setError(errorMessage(e,'取消失败。')));};window.addEventListener('keydown',key);return()=>{alive=false;clearInterval(timer);window.removeEventListener('keydown',key);};},[sessionId]);
 const action=(a:'next'|'stop'|'cancel')=>void scrollAction(sessionId,a).catch(e=>setError(errorMessage(e,'操作失败。')));
 return <main className="scroll-controller"><strong>滚动长截图 · {p?.frames??0}段 / {p?.height??0}px</strong><p role="status">{error??p?.message??'正在准备首帧…'}</p><div>{(!p?.automatic||p?.state==='paused')&&<button className="capture-button" onClick={()=>action('next')}><ChevronDown size={14}/>采集下一段</button>}<button className="secondary-button" onClick={()=>action('stop')}><Square size={14}/>停止</button><button className="secondary-button" onClick={()=>action('cancel')}><X size={14}/>取消</button></div></main>;
}
