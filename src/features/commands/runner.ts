import {errorMessage} from '../../shared/api/errors';
import {actionSpecs,validateWorkflow,type ActionId,type WorkflowConfig,type ValueType} from './model';
export type Payload={type:'none'}|{type:'image';blob:Blob;name:string;widthPx:number;heightPx:number;screenshotId?:string}|{type:'draft'|'confirmed';text:string;format:'text'|'latex';image:Extract<Payload,{type:'image'}>}|{type:'result';message:string;screenshotId?:string;documentId?:string};
export interface Context {signal:AbortSignal;select:()=>Promise<Extract<Payload,{type:'image'}>>;confirm:(draft:Extract<Payload,{type:'draft'|'confirmed'}>)=>Promise<string>}
export interface Action {actionId:ActionId;title:string;inputType:ValueType;outputType:ValueType;errorPolicy:'retry'|'inspect';execute:(input:Payload,context:Context)=>Promise<Payload>}
export type ActionRegistry=ReadonlyMap<ActionId,Action>;
export interface RunState {phase:'idle'|'running'|'waiting'|'cancelling'|'cancelled'|'failed'|'completed';workflow:WorkflowConfig|null;step:number;completed:number;values:Payload[];error:string|null;retryable:boolean}
const empty=():RunState=>({phase:'idle',workflow:null,step:0,completed:0,values:[],error:null,retryable:false});
const active=(s:RunState)=>['running','waiting','cancelling'].includes(s.phase);
/** A finite linear runner. Checkpoints live only for this process; restart never resumes writes. */
export function createRunner(registry:ActionRegistry,interaction:{select:(signal:AbortSignal)=>ReturnType<Context['select']>;confirm:(draft:Parameters<Context['confirm']>[0],signal:AbortSignal)=>Promise<string>}){
 let state=empty(),controller=new AbortController();const listeners=new Set<()=>void>();
 const set=(patch:Partial<RunState>)=>{state={...state,...patch};listeners.forEach(f=>f());};
 const run=async()=>{
  const workflow=state.workflow!;
  for(let i=state.completed;i<workflow.actions.length;i++){
   if(controller.signal.aborted){set({phase:'cancelled'});return;}
   const action=registry.get(workflow.actions[i] as ActionId)!;set({phase:'running',step:i,error:null,retryable:false});
   try{
    const input=state.values.at(-1)??{type:'none' as const};if(input.type!==action.inputType)throw Error('运行输入类型不匹配。');
    const wait=async<T>(fn:()=>Promise<T>)=>{set({phase:'waiting'});try{return await fn();}finally{if(!controller.signal.aborted)set({phase:'running'});}};
    const output=await action.execute(input,{signal:controller.signal,select:()=>wait(()=>interaction.select(controller.signal)),confirm:d=>wait(()=>interaction.confirm(d,controller.signal))});
    if(output.type!==action.outputType)throw Error('动作返回类型不匹配，请检查结果后重新开始。');
    // Preserve successful irreversible outcomes even when cancellation arrived while awaiting IPC.
    set({completed:i+1,values:[...state.values,output]});
   }catch(error){const aborted=error instanceof DOMException&&error.name==='AbortError';if(controller.signal.aborted||aborted){set({phase:'cancelled',error:aborted?null:errorMessage(error,'取消期间步骤失败，请核对当前任务状态。'),retryable:false});return;}set({phase:'failed',error:errorMessage(error,'步骤失败，已完成步骤保留。'),retryable:action.errorPolicy==='retry'});return;}
  }
  set({phase:controller.signal.aborted?'cancelled':'completed'});
 };
 return {
  getSnapshot:()=>state,subscribe:(f:()=>void)=>{listeners.add(f);return()=>{listeners.delete(f);};},
  start:async(value:WorkflowConfig)=>{if(active(state))return false;const workflow=validateWorkflow(value);for(const id of workflow.actions){const a=registry.get(id as ActionId),spec=actionSpecs.find(a=>a.actionId===id);if(!a||a.inputType!==spec?.inputType||a.outputType!==spec.outputType||a.errorPolicy!==spec.errorPolicy)throw Error('动作注册表与契约不匹配。');}controller=new AbortController();state={...empty(),workflow};set({phase:'running'});await run();return true;},
  retry:async()=>{if(state.phase!=='failed'||!state.retryable)return false;controller=new AbortController();set({phase:'running'});await run();return true;},
  cancel:()=>{if(active(state)){controller.abort();set({phase:'cancelling',retryable:false});}},
 };
}
