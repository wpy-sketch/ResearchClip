import {useEffect,useState} from 'react';
import * as api from './api';
import {errorMessage} from '../../shared/api/errors';
interface Image {id:string;title:string}
export function LocalAi({reference,peer}:{reference:Image|null;peer:Image|null}){
 const [status,setStatus]=useState<api.AiStatus|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState<string|null>(null);
 useEffect(()=>{let active=true,timer:ReturnType<typeof setTimeout>;const poll=async()=>{try{const value=await api.aiStatus();if(active)setStatus(value);}catch(e){if(active)setError(errorMessage(e,'本地AI状态读取失败。'));}if(active)timer=setTimeout(poll,900);};void poll();return()=>{active=false;clearTimeout(timer);};},[]);
 const running=status?.job.phase==='running'||status?.job.phase==='validating';
 const act=async(work:()=>Promise<api.AiStatus>)=>{if(busy)return;setBusy(true);setError(null);try{setStatus(await work());}catch(e){setError(errorMessage(e,'本地AI操作失败，素材未修改。'));}finally{setBusy(false);}};
 const belongs=reference&&peer&&status?.job.pair?.[0]===reference.id&&status?.job.pair?.[1]===peer.id;
 return <details className="similarity-ai" open={!!peer}><summary>Qwen3.5 4B · 离线辅助比较（实验性）</summary><div><p>{status?.available?'已找到离线模型；运行前校验固定版本与SHA256。':'尚未配置完整离线包；传统哈希检索不受影响。'}</p><p>选择下方结果中的“AI辅助比较”，再开始。只读取所选两张原图的缩略图，不联网、不自动写笔记、不删除素材。不评价数学正确性，输出可能漏检、幻觉或截断。</p><div className="similarity-actions"><button className="secondary-button" disabled={busy||running} onClick={()=>void act(api.chooseAi)}>选择捆绑模型目录</button><button className="capture-button" disabled={busy||running||!status?.available||!reference||!peer} onClick={()=>{if(reference&&peer)void act(()=>api.startAi(reference.id,peer.id));}}>开始本地AI比较</button><button className="secondary-button" disabled={!running} onClick={()=>void api.cancelAi().then(setStatus).catch(e=>setError(errorMessage(e,'取消请求失败，请重试。')))}>取消本地AI任务</button></div>{reference&&peer&&<p>本次比较：{reference.title} ↔ {peer.title}</p>}{running&&<p role="status">{status?.job.phase==='validating'?'正在校验约3.5GB离线资源…':'本机CPU推理中，最长180秒；可取消。'} 不会影响传统索引结果。</p>}{(error||status?.job.error)&&<p role="alert" className="error-banner">{error??status?.job.error}</p>}{status?.job.text&&(belongs?<div className="similarity-ai-output"><strong>模型建议 · 未经人工验证</strong><pre>{status.job.text}</pre></div>:<p>保留着上一次图片对的结果；当前图片对不同，不混用旧结果。</p>)}</div></details>;
}
