import {create} from 'zustand';
import {currentScroll,scrollTargets,prepareScroll,startScroll,scrollStatus,scrollAction,type ScrollProgress,type ScrollTarget} from './scroll-api';
import {errorMessage} from '../../shared/api/errors';
import {useCaptureStore} from './store';
interface State {visible:boolean;busy:boolean;targets:ScrollTarget[];progress:ScrollProgress|null;error:string|null;open:()=>Promise<void>;prepare:(id:string,auto:boolean)=>Promise<void>;start:()=>Promise<void>;poll:()=>Promise<void>;close:()=>Promise<void>;action:(a:'next'|'stop'|'cancel')=>Promise<void>}
export const terminal=(state:string)=>['done','cancelled','failed'].includes(state);
export const useScrollStore=create<State>((set,get)=>({visible:false,busy:false,targets:[],progress:null,error:null,
 open:async()=>{if(get().busy)return;set({visible:true,busy:true,error:null});try{const p=await currentScroll();if(p&&!terminal(p.state)){set({progress:p});useCaptureStore.setState({phase:'capturing'});}else set({targets:await scrollTargets(),progress:null});}catch(e){set({error:errorMessage(e,'无法读取目标窗口。')});}finally{set({busy:false});}},
 prepare:async(id,auto)=>{if(get().busy||get().progress&&!terminal(get().progress!.state))return;set({busy:true,error:null});useCaptureStore.setState({phase:'capturing'});try{set({progress:await prepareScroll(id,auto)});}catch(e){set({error:errorMessage(e,'未能选择长截图区域。')});useCaptureStore.setState({phase:'idle'});}finally{set({busy:false});}},
 start:async()=>{const p=get().progress;if(get().busy||p?.state!=='ready')return;set({busy:true,error:null});try{await startScroll(p.sessionId);}catch(e){set({error:errorMessage(e,'无法开始长截图。')});useCaptureStore.setState({phase:'idle'});}finally{set({busy:false});}},
 poll:async()=>{const p=get().progress;if(!p||terminal(p.state))return;try{const next=await scrollStatus(p.sessionId);if(get().progress?.sessionId!==p.sessionId)return;set({progress:next});if(next.result)await useCaptureStore.getState().acceptNative(next.result);else if(terminal(next.state))useCaptureStore.setState({phase:'idle'});}catch(e){set({error:errorMessage(e,'无法读取长截图状态，请取消后重试。')});}},
 action:async(action)=>{const p=get().progress;if(!p)return;try{await scrollAction(p.sessionId,action);set({error:null});}catch(e){set({error:errorMessage(e,'长截图操作失败。')});}},
 close:async()=>{if(get().busy)return;const p=get().progress;if(p&&!terminal(p.state)){try{await scrollAction(p.sessionId,'cancel');}catch(e){set({error:errorMessage(e,'取消失败，已保留控制面板。')});return;}useCaptureStore.setState({phase:'idle'});}set({visible:false});},
}));
