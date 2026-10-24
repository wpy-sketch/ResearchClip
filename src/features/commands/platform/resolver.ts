import {invoke} from '@tauri-apps/api/core';
import {commandResources,parseResolved,resourceKey,type CommandRequest,type ResourceKey,type ResolvedResource} from './contracts';
import {CommandFailure} from './errors';
import {PermissionService,type ModuleIdentity} from './permissions';
export type NativeCall=(name:string,args:Record<string,unknown>)=>Promise<unknown>;
export class ResourceResolver {
 constructor(private permissions:PermissionService,private call:NativeCall=invoke){}
 async resolve(identity:ModuleIdentity,key:ResourceKey,signal:AbortSignal):Promise<ResolvedResource>{this.permissions.check(identity,'resource.resolve');const checked=resourceKey(key);signal.throwIfAborted();const raw=await this.call('resolve_research_resource',{request:{version:1,key:checked}});signal.throwIfAborted();this.permissions.check(identity,'resource.resolve');const parsed=parseResolved(raw);if(parsed.key.id!==checked.id||parsed.key.kind!==checked.kind)throw new CommandFailure('INVALID_RESPONSE');return parsed;}
 async prepare(identity:ModuleIdentity,request:CommandRequest,signal:AbortSignal){
  this.permissions.check(identity,request.command.name);signal.throwIfAborted();const raw=await this.call('prepare_research_command',{request});signal.throwIfAborted();this.permissions.check(identity,request.command.name);
  const expected=commandResources(request.command);if(!Array.isArray(raw)||raw.length!==expected.length)throw new CommandFailure('INVALID_RESPONSE');const resources=raw.map(parseResolved);
  if(resources.some((r,i)=>r.key.kind!==expected[i].kind||r.key.id!==expected[i].id))throw new CommandFailure('INVALID_RESPONSE');return resources;
 }
}
