export const commandNames=['document.open','markdown.insertImage','pdf.navigateTo','screenshot.capturePdfSelection','annotation.create','codeWorkspace.open','agent.requestEdit'] as const;
export type CommandName=typeof commandNames[number];
export const eventNames=['resource.opened','markdown.imageInserted','command.completed'] as const;
export type EventName=typeof eventNames[number];
export const resourceKinds=['project','document','literature','screenshot','pdfAttachment','pdfLocation','annotation','handwritingLayer','stroke','codeWorkspace','evidence','agentSession'] as const;
export type ResourceKind=typeof resourceKinds[number];
export interface ResourceKey<K extends ResourceKind=ResourceKind>{readonly kind:K;readonly id:string}
export interface CommandArgs {
 'document.open':{resource:ResourceKey<'document'|'screenshot'|'pdfAttachment'>};
 'markdown.insertImage':{document:ResourceKey<'document'>;image:ResourceKey<'screenshot'>};
 'pdf.navigateTo':{location:ResourceKey<'pdfLocation'>};
 'screenshot.capturePdfSelection':{location:ResourceKey<'pdfLocation'>};
 'annotation.create':{location:ResourceKey<'pdfLocation'>;annotationType:'highlight'|'comment'|'ink'};
 'codeWorkspace.open':{workspace:ResourceKey<'codeWorkspace'>};
 'agent.requestEdit':{session:ResourceKey<'agentSession'>;document:ResourceKey<'document'>;expectedRevision:number};
}
export type ApplicationCommand={[N in CommandName]:{readonly name:N;readonly args:CommandArgs[N]}}[CommandName];
export interface CommandRequest {readonly version:1;readonly requestId:string;readonly command:ApplicationCommand}
export type EditorType='markdown'|'image'|'pdf'|'code';
export interface ResolvedResource {readonly key:ResourceKey;readonly formatVersion:1;readonly editor:EditorType|null;readonly moduleAvailable:boolean}
export const object=(v:unknown):v is Record<string,unknown>=>!!v&&typeof v==='object'&&!Array.isArray(v);
export const stableId=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{32}$/.test(v);
export function fields(v:unknown,names:string[]):asserts v is Record<string,unknown>{if(!object(v)||Object.keys(v).length!==names.length||names.some(k=>!Object.hasOwn(v,k)))throw Error('INVALID_INPUT');}
export function resourceKey(v:unknown,allowed:readonly ResourceKind[]=resourceKinds):ResourceKey{
 fields(v,['kind','id']);if(!allowed.includes(v.kind as ResourceKind)||!stableId(v.id))throw Error('INVALID_INPUT');return Object.freeze({kind:v.kind as ResourceKind,id:v.id});
}
/** Reconstruct every allowed field; no paths, SQL, grant tokens or caller-supplied privileges. */
export function parseCommand(value:unknown):ApplicationCommand{
 fields(value,['name','args']);const a=value.args;
 switch(value.name){
  case 'document.open':fields(a,['resource']);return Object.freeze({name:value.name,args:Object.freeze({resource:resourceKey(a.resource,['document','screenshot','pdfAttachment']) as CommandArgs['document.open']['resource']})});
  case 'markdown.insertImage':fields(a,['document','image']);return Object.freeze({name:value.name,args:Object.freeze({document:resourceKey(a.document,['document']) as ResourceKey<'document'>,image:resourceKey(a.image,['screenshot']) as ResourceKey<'screenshot'>})});
  case 'pdf.navigateTo':case 'screenshot.capturePdfSelection':fields(a,['location']);return Object.freeze({name:value.name,args:Object.freeze({location:resourceKey(a.location,['pdfLocation']) as ResourceKey<'pdfLocation'>})});
  case 'annotation.create':fields(a,['location','annotationType']);if(typeof a.annotationType!=='string'||!['highlight','comment','ink'].includes(a.annotationType))throw Error('INVALID_INPUT');return Object.freeze({name:value.name,args:Object.freeze({location:resourceKey(a.location,['pdfLocation']) as ResourceKey<'pdfLocation'>,annotationType:a.annotationType as 'highlight'|'comment'|'ink'})});
  case 'codeWorkspace.open':fields(a,['workspace']);return Object.freeze({name:value.name,args:Object.freeze({workspace:resourceKey(a.workspace,['codeWorkspace']) as ResourceKey<'codeWorkspace'>})});
  case 'agent.requestEdit':fields(a,['session','document','expectedRevision']);if(!Number.isSafeInteger(a.expectedRevision)||Number(a.expectedRevision)<1)throw Error('INVALID_INPUT');return Object.freeze({name:value.name,args:Object.freeze({session:resourceKey(a.session,['agentSession']) as ResourceKey<'agentSession'>,document:resourceKey(a.document,['document']) as ResourceKey<'document'>,expectedRevision:a.expectedRevision as number})});
  default:throw Error('UNKNOWN_COMMAND');
 }
}
export function commandResources(command:ApplicationCommand):ResourceKey[]{switch(command.name){case 'document.open':return [command.args.resource];case 'markdown.insertImage':return [command.args.document,command.args.image];case 'pdf.navigateTo':case 'screenshot.capturePdfSelection':case 'annotation.create':return [command.args.location];case 'codeWorkspace.open':return [command.args.workspace];case 'agent.requestEdit':return [command.args.session,command.args.document];}}
export function parseResolved(value:unknown):ResolvedResource{
 fields(value,['key','formatVersion','editor','moduleAvailable']);if(value.formatVersion!==1||typeof value.moduleAvailable!=='boolean'||!(value.editor===null||typeof value.editor==='string'&&['markdown','image','pdf','code'].includes(value.editor)))throw Error('INVALID_RESPONSE');return Object.freeze({key:resourceKey(value.key),formatVersion:1,editor:value.editor as EditorType|null,moduleAvailable:value.moduleAvailable});
}
