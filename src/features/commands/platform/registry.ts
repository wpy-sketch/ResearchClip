import {commandNames,commandResources,parseCommand,stableId,type ApplicationCommand,type CommandName,type CommandArgs,type CommandRequest,type ResolvedResource} from './contracts';
import {CommandFailure,ErrorReporter,failure} from './errors';
import {PermissionService,type ModuleIdentity} from './permissions';
import {TaskManager,type TaskContext} from './tasks';
import {ResourceResolver} from './resolver';
import {EventBus,type EventCause} from './events';
export interface ExecutionContext extends TaskContext {request:CommandRequest;resources:ResolvedResource[];cause:EventCause}
export interface CommandReceipt {readonly requestId:string;readonly status:'completed'}
type Handler=(context:ExecutionContext,command:ApplicationCommand)=>Promise<void>;
/** One registry of application operations; the existing OCR workflow runner remains unchanged. */
export class CommandRegistry {
 private handlers=new Map<CommandName,Handler>();private requests=new Map<string,{fingerprint:string;promise:Promise<CommandReceipt>}>();private locks=new Set<string>();
 constructor(private permissions:PermissionService,private resolver:ResourceResolver,readonly tasks:TaskManager,readonly errors:ErrorReporter,private events:EventBus,private publisher:ModuleIdentity){}
 register<N extends CommandName>(name:N,handler:(context:ExecutionContext,args:CommandArgs[N])=>Promise<void>){if(!commandNames.includes(name)||this.handlers.has(name))throw new CommandFailure('INVALID_INPUT');this.handlers.set(name,(context,command)=>handler(context,command.args as CommandArgs[N]));}
 execute<N extends CommandName>(identity:ModuleIdentity,name:N,args:CommandArgs[N],requestId=crypto.randomUUID().replaceAll('-',''),parentCause?:EventCause):Promise<CommandReceipt>{
  let command:ApplicationCommand;
  try{command=parseCommand({name,args});if(!stableId(requestId))throw new CommandFailure('INVALID_INPUT');this.permissions.check(identity,command.name);}catch(error){const e=this.errors.report(failure(error));return Promise.reject(e);}
  const fingerprint=JSON.stringify(command),old=this.requests.get(requestId);
  if(old){if(old.fingerprint===fingerprint)return old.promise;const e=this.errors.report(new CommandFailure('REQUEST_REUSED'),{command:name,requestId});return Promise.reject(e);}
  if(this.requests.size>=1024){const e=this.errors.report(new CommandFailure('LEDGER_FULL'),{command:name,requestId});return Promise.reject(e);}
  let cause:EventCause;try{cause=this.events.commandCause(name,parentCause);}catch(error){return Promise.reject(this.errors.report(error,{command:name,requestId}));}
  const keys=commandResources(command).map(k=>k.kind+':'+k.id),request:CommandRequest=Object.freeze({version:1,requestId,command});
  // Install receipt before starting an async handler, including reentrant event subscribers.
  const promise=Promise.resolve().then(async()=>{
   if(keys.some(k=>this.locks.has(k)))throw new CommandFailure('BUSY');for(const key of keys)this.locks.add(key);
   try{return await this.tasks.run(requestId,name,async context=>{const resources=await this.resolver.prepare(identity,request,context.signal);if(resources.some(r=>!r.moduleAvailable))throw new CommandFailure('MODULE_UNAVAILABLE');context.progress(50);const handler=this.handlers.get(name);if(!handler)throw new CommandFailure('MODULE_UNAVAILABLE');this.permissions.check(identity,name);context.signal.throwIfAborted();await handler({...context,request,resources,cause},command);this.events.publish(this.publisher,'command.completed',{requestId,command:name},cause);return Object.freeze({requestId,status:'completed' as const});});}
   finally{for(const key of keys)this.locks.delete(key);}
  }).catch(error=>{const e=failure(error);if(e.code!=='CANCELLED')this.errors.report(e,{command:name,requestId});throw e;});
  this.requests.set(requestId,{fingerprint,promise});return promise;
 }
}
