import {commandNames,eventNames,type CommandName,type EventName} from './contracts';
import {CommandFailure} from './errors';
export type Capability=CommandName|'resource.resolve'|`event:${EventName}`;
export interface ModuleIdentity {readonly module:string}
/** A handle is valid only in the service that issued it. Module strings never grant authority. */
export class PermissionService {
 private grants=new WeakMap<ModuleIdentity,ReadonlySet<Capability>>();
 issue(module:string,capabilities:readonly Capability[]):ModuleIdentity{
  const allowed:string[]=[...commandNames,'resource.resolve',...eventNames.map(n=>'event:'+n)];
  if(!/^[a-z][a-z0-9-]{0,39}$/.test(module)||capabilities.some(c=>!allowed.includes(c)))throw new CommandFailure('INVALID_INPUT');
  const identity=Object.freeze({module});this.grants.set(identity,new Set(capabilities));return identity;
 }
 revoke(identity:ModuleIdentity){this.grants.delete(identity);}
 check(identity:ModuleIdentity,capability:Capability){if(!this.grants.get(identity)?.has(capability))throw new CommandFailure('PERMISSION_DENIED');}
}
