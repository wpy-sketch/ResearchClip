import {commandNames,eventNames,fields,resourceKey,stableId,type CommandName,type EventName,type ResourceKey} from './contracts';
import {CommandFailure,ErrorReporter} from './errors';
import {PermissionService,type ModuleIdentity} from './permissions';
export interface EventMap {
 'resource.opened':{readonly requestId:string;readonly resource:ResourceKey};
 'markdown.imageInserted':{readonly requestId:string;readonly document:ResourceKey<'document'>;readonly image:ResourceKey<'screenshot'>};
 'command.completed':{readonly requestId:string;readonly command:CommandName};
}
export interface EventCause {readonly depth:number}
function payload<N extends EventName>(name:N,value:unknown):EventMap[N]{
 let result:unknown;
 if(name==='resource.opened'){fields(value,['requestId','resource']);result={requestId:value.requestId,resource:resourceKey(value.resource)};}
 else if(name==='markdown.imageInserted'){fields(value,['requestId','document','image']);result={requestId:value.requestId,document:resourceKey(value.document,['document']),image:resourceKey(value.image,['screenshot'])};}
 else{fields(value,['requestId','command']);if(!commandNames.includes(value.command as CommandName))throw new CommandFailure('INVALID_INPUT');result={requestId:value.requestId,command:value.command};}
 if(!stableId(value.requestId))throw new CommandFailure('INVALID_INPUT');return Object.freeze(result) as EventMap[N];
}
/** A local typed notification bus. Events never dispatch application commands automatically. */
export class EventBus {
 private listeners=new Map<EventName,Set<(data:unknown,cause:EventCause)=>void>>();private causes=new WeakMap<EventCause,{events:readonly EventName[];commands:readonly CommandName[]}>();private active:EventCause|undefined;
 constructor(private permissions:PermissionService,private errors:ErrorReporter){}
 commandCause(command:CommandName,parent?:EventCause){
  const cause=parent??this.active;if(cause&&!this.causes.has(cause))throw new CommandFailure('INVALID_INPUT');const trace=cause?this.causes.get(cause)!:{events:[],commands:[]};
  if(!commandNames.includes(command))throw new CommandFailure('UNKNOWN_COMMAND');if(trace.commands.includes(command)||trace.events.length+trace.commands.length>=8)throw new CommandFailure('EVENT_LOOP');
  const next=Object.freeze({depth:trace.events.length+trace.commands.length+1});this.causes.set(next,{events:trace.events,commands:[...trace.commands,command]});return next;
 }
 listenerCount(){return [...this.listeners.values()].reduce((n,s)=>n+s.size,0);}
 subscribe<N extends EventName>(identity:ModuleIdentity,name:N,listener:(data:EventMap[N],cause:EventCause)=>void,options:{signal?:AbortSignal;once?:boolean}={}){
  this.check(identity,name);if(listener.constructor.name==='AsyncFunction')throw new CommandFailure('INVALID_INPUT');if(options.signal?.aborted)return()=>{};
  if(this.listenerCount()>=128)throw new CommandFailure('LISTENER_LIMIT');const group=this.listeners.get(name)??new Set();this.listeners.set(name,group);
  const stop=()=>{group.delete(wrapped);options.signal?.removeEventListener('abort',stop);};
  const wrapped=(data:unknown,cause:EventCause)=>{if(options.once)stop();try{this.check(identity,name);}catch(e){stop();throw e;}const result:unknown=listener(data as EventMap[N],cause);if(result&&typeof result==='object'&&'then'in result){stop();void Promise.resolve(result).catch(()=>this.errors.report(new CommandFailure('LISTENER_FAILED')));throw new CommandFailure('LISTENER_FAILED');}};
  group.add(wrapped);options.signal?.addEventListener('abort',stop,{once:true});return stop;
 }
 private check(identity:ModuleIdentity,name:EventName){if(!eventNames.includes(name))throw new CommandFailure('INVALID_INPUT');this.permissions.check(identity,`event:${name}`);}
 publish<N extends EventName>(identity:ModuleIdentity,name:N,value:EventMap[N],parent?:EventCause){
  this.check(identity,name);const parsed=payload(name,value),cause=parent??this.active;
  if(cause&&!this.causes.has(cause))throw new CommandFailure('INVALID_INPUT');const trace=cause?this.causes.get(cause)!:{events:[],commands:[]},chain=trace.events;
  if(chain.length+trace.commands.length>=8||chain.includes(name))throw new CommandFailure('EVENT_LOOP');const next=Object.freeze({depth:chain.length+trace.commands.length+1});this.causes.set(next,{events:[...chain,name],commands:trace.commands});const previous=this.active;this.active=next;
  try{for(const listener of [...(this.listeners.get(name)??[])]){if(!this.listeners.get(name)?.has(listener))continue;try{listener(parsed,next);}catch(error){this.errors.report(error instanceof CommandFailure?error:new CommandFailure('LISTENER_FAILED'));}}}finally{this.active=previous;}
 }
}
