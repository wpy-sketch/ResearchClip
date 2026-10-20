import {invoke,isTauri} from '@tauri-apps/api/core';
export interface SyncConfig {server:string;folder:string;projects:string[];unclassified:boolean}
export interface SyncConflict {id:string;resourceKey:string;reason:string;localManifest:string|null;remoteManifest:string|null}
export interface SyncStatus {config:SyncConfig|null;credentialAvailable:boolean;credentialError?:string|null;pending:number;conflicts:SyncConflict[];job:{phase:string;message:string;writable:boolean;progress:{completed:number;total:number;uploaded:number;downloaded:number;conflicts:number;skipped:number};remoteProjects:{id:string;name:string}[]}}
const object=(v:unknown):v is Record<string,unknown>=>Boolean(v)&&typeof v==='object'&&!Array.isArray(v);
const id=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{32}$/.test(v);
const count=(v:unknown):v is number=>Number.isSafeInteger(v)&&Number(v)>=0;
export const syncRunning=(phase:string)=>['testing','syncing','stopping'].includes(phase);
export function parseSyncConfig(v:unknown):SyncConfig {if(!object(v)||typeof v.server!=='string'||!v.server.startsWith('https://')||v.server.length>2048||typeof v.folder!=='string'||(v.folder.length<14||v.folder.length>80||!/^researchclip-[a-zA-Z0-9-]+$/.test(v.folder))||!Array.isArray(v.projects)||v.projects.length>500||!v.projects.every(id)||typeof v.unclassified!=='boolean')throw Error('WebDAV 配置响应无效。');const url=new URL(v.server);if(url.protocol!=='https:'||url.username||url.password||url.search||url.hash||!v.server.endsWith('/')||new Set(v.projects).size!==v.projects.length)throw Error('WebDAV配置不安全。');return {server:v.server,folder:v.folder,projects:v.projects,unclassified:v.unclassified};}
export function parseSyncStatus(v:unknown):SyncStatus {
 if(!object(v)||typeof v.credentialAvailable!=='boolean'||!count(v.pending)||!Array.isArray(v.conflicts)||v.conflicts.length>5000||!object(v.job))throw Error('同步状态响应无效。');const j=v.job,p=j.progress;
 if(typeof j.phase!=='string'||!['idle','testing','syncing','stopping','succeeded','conflicts','stopped','failed'].includes(j.phase)||typeof j.message!=='string'||j.message.length>2048||typeof j.writable!=='boolean'||!object(p)||!['completed','total','uploaded','downloaded','conflicts','skipped'].every(k=>count(p[k]))||!Array.isArray(j.remoteProjects)||j.remoteProjects.length>500||!j.remoteProjects.every(x=>object(x)&&id(x.id)&&typeof x.name==='string'&&x.name.length<=160))throw Error('同步进度响应无效。');
 const hash=(s:unknown)=>s===null||typeof s==='string'&&/^[a-f0-9]{64}$/.test(s);
 if(!v.conflicts.every(c=>object(c)&&id(c.id)&&typeof c.resourceKey==='string'&&/^(screenshot|project|document)\/[a-f0-9]{32}$/.test(c.resourceKey)&&typeof c.reason==='string'&&c.reason.length<=2048&&hash(c.localManifest)&&hash(c.remoteManifest)))throw Error('同步冲突响应无效。');
 if(v.credentialError!==undefined&&v.credentialError!==null&&(typeof v.credentialError!=='string'||v.credentialError.length>2048))throw Error('凭据状态响应无效。');
 return {config:v.config===null?null:parseSyncConfig(v.config),credentialAvailable:v.credentialAvailable,credentialError:v.credentialError??null,pending:v.pending,conflicts:v.conflicts.map(c=>({id:c.id,resourceKey:c.resourceKey,reason:c.reason,localManifest:c.localManifest,remoteManifest:c.remoteManifest})),job:{phase:j.phase,message:j.message,writable:j.writable,progress:{completed:p.completed,total:p.total,uploaded:p.uploaded,downloaded:p.downloaded,conflicts:p.conflicts,skipped:p.skipped},remoteProjects:j.remoteProjects.map(p=>({id:p.id,name:p.name}))}} as SyncStatus;
}
async function call(name:string,args?:Record<string,unknown>):Promise<unknown>{if(!isTauri())throw Error('WebDAV 需要 Windows 桌面应用；浏览器预览不会读写服务器或凭据。');return invoke(name,args);}
export async function syncStatus(){return parseSyncStatus(await call('sync_status'));}
export async function configureSync(config:SyncConfig,username:string,password:string){return parseSyncStatus(await call('sync_configure',{config,username:username||null,password:password||null}));}
export async function startSync(test:boolean){await call('sync_start',{test});}
export async function stopSync(){await call('sync_stop');}
export async function resolveSync(id:string,choice:'local'|'remote'|'both'){await call('sync_resolve',{id,choice,confirmed:true});}
export async function authorizeSyncDocuments(){const v=await call('sync_authorize_documents');if(!count(v))throw Error('文档授权结果无效。');return v;}
