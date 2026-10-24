import {commandNames,stableId,type CommandName} from './contracts';
import {CommandFailure,ErrorReporter,failure} from './errors';
export interface TaskRecord {readonly id:string;readonly command:CommandName;readonly phase:'running'|'cancelling'|'completed'|'cancelled'|'failed';readonly progress:number;readonly cancellable:boolean}
export interface TaskContext {signal:AbortSignal;progress:(percent:number)=>void;commit:()=>void}
export class TaskManager {
 private controllers=new Map<string,AbortController>();private records:readonly TaskRecord[]=[];private listeners=new Set<()=>void>();private used=new Set<string>();
 constructor(private errors=new ErrorReporter()){}
 getSnapshot=()=>this.records;
 subscribe=(fn:()=>void)=>{this.listeners.add(fn);return()=>{this.listeners.delete(fn);};};
 private notify(){for(const fn of [...this.listeners])try{fn();}catch{this.listeners.delete(fn);this.errors.report(new CommandFailure('LISTENER_FAILED'));}}
 private update(id:string,patch:Partial<TaskRecord>){this.records=Object.freeze(this.records.map(r=>r.id===id?Object.freeze({...r,...patch}):r));this.notify();}
 cancel(id:string){const task=this.records.find(t=>t.id===id);if(!task||task.phase!=='running')return false;if(!task.cancellable)throw new CommandFailure('TASK_COMMITTING');this.update(id,{phase:'cancelling',cancellable:false});this.controllers.get(id)?.abort();return true;}
 async run<T>(id:string,command:CommandName,execute:(context:TaskContext)=>Promise<T>):Promise<T>{
  if(!stableId(id)||!commandNames.includes(command))throw new CommandFailure('INVALID_INPUT');
  if(this.controllers.size>=4||this.used.has(id))throw new CommandFailure('BUSY');if(this.used.size>=1024)throw new CommandFailure('LEDGER_FULL');this.used.add(id);
  const controller=new AbortController();this.controllers.set(id,controller);this.records=Object.freeze([...this.records.filter(r=>this.controllers.has(r.id)||this.records.indexOf(r)>=this.records.length-24),Object.freeze({id,command,phase:'running' as const,progress:0,cancellable:true})]);
  this.notify();let committed=false,settled=false;
  const context:TaskContext={signal:controller.signal,progress:percent=>{if(settled||controller.signal.aborted)return;if(!Number.isFinite(percent)||percent<0||percent>100)throw new CommandFailure('INVALID_INPUT');this.update(id,{progress:Math.max(this.records.find(r=>r.id===id)!.progress,Math.round(percent))});},commit:()=>{controller.signal.throwIfAborted();committed=true;this.update(id,{cancellable:false});}};
  try{controller.signal.throwIfAborted();const result=await execute(context);if(!committed)controller.signal.throwIfAborted();this.update(id,{phase:'completed',progress:100,cancellable:false});return result;}
  catch(error){const e=controller.signal.aborted&&!committed?new CommandFailure('CANCELLED'):failure(error);this.update(id,{phase:e.code==='CANCELLED'?'cancelled':'failed',cancellable:false});throw e;}
  finally{settled=true;this.controllers.delete(id);}
 }
}
