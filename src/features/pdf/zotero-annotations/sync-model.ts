import {revisionMetadataValid,type RevisionMetadata} from '../../../shared/revisions/model';
import {annotationSource,sourceKey,parseAnnotationCache,type AnnotationSource,type AnnotationReadCache,type ZoteroAnnotation} from './model';
import type {WriteInput,WriteOutcome} from './Writer';
export const syncStates=['SYNCED','LOCAL_DIRTY','REMOTE_CHANGED','CONFLICT','FAILED','READ_ONLY'] as const;
export type AnnotationSyncState=typeof syncStates[number];
export interface AnnotationSyncEntry {formatVersion:1;id:string;documentId:string;source:AnnotationSource;pdfContentHash:string;annotationKey:string;revision:number;status:AnnotationSyncState;base:ZoteroAnnotation|null;remote:ZoteroAnnotation|null;local:WriteInput|null;needsVerification:boolean;message:string;errorCode:string|null;localUpdatedAt:number|null;lastSyncedAt:number|null;updatedAt:number}
export interface AnnotationSyncScope {refreshInterval:0|30|60|300;lastRefreshedAt:number|null;lastAttemptAt:number|null;lastSyncedAt:number|null;message:string}
export interface AnnotationSyncSnapshot {entries:AnnotationSyncEntry[];scope:AnnotationSyncScope}
export interface AnnotationSyncResponse {activeId:string|null;snapshot:AnnotationSyncSnapshot;outcome:WriteOutcome|null;cache:AnnotationReadCache|null;diagnostic:{code:string;message:string}|null;history:RevisionMetadata[]|null;historicalEntry:AnnotationSyncEntry|null}
export type SyncRequest={operation:'list'|'refresh'|'acceptRead'}|{operation:'stage';changeId:string|null;revision:number|null;input:WriteInput}|{operation:'submit'|'verify';changeId:string;revision:number}|{operation:'resolve';changeId:string;revision:number;remoteVersion:number|null;choice:'local'|'remote'}|{operation:'setRefresh';interval:number}|{operation:'history';changeId:string}|{operation:'revision';changeId:string;revisionId:string};
const id=(v:unknown)=>typeof v==='string'&&/^[a-f0-9]{32}$/.test(v),key=(v:unknown)=>typeof v==='string'&&/^[A-Z0-9]{8}$/.test(v),hash=(v:unknown)=>typeof v==='string'&&/^[a-f0-9]{64}$/.test(v),time=(v:unknown)=>v===null||Number.isSafeInteger(v)&&Number(v)>0&&Number(v)<=8.64e15;
const text=(v:unknown,max=65536)=>typeof v==='string'&&new TextEncoder().encode(v).length<=max&&![...v].some(c=>{const n=c.codePointAt(0)!;return (n<32||n>=127&&n<160)&&!['\r','\n','\t'].includes(c);});
export function validLocalInput(input:unknown):input is WriteInput {
 const v=input as WriteInput|null;if(!v||!['create','update','delete'].includes(v.action))return false;
 if(v.action!=='create'&&(!key(v.annotationKey)||!Number.isSafeInteger(v.expectedVersion)||v.expectedVersion<=0))return false;
 if(v.action==='delete')return Object.keys(v).every(k=>['action','annotationKey','expectedVersion'].includes(k));
 if(!['highlight','underline','note'].includes(v.annotationType)||!/^#[a-f0-9]{6}$/i.test(v.color)||!text(v.text)||!text(v.comment)||v.annotationType==='note'&&!!v.text)return false;
 if(v.action==='update')return Object.keys(v).every(k=>['action','annotationKey','expectedVersion','annotationType','color','text','comment'].includes(k));
 const g=v.geometry;if(!g||!Number.isInteger(g.pageIndex)||g.pageIndex<0||g.pageIndex>=2000||!Array.isArray(g.view)||g.view.length!==4||!g.view.every(Number.isFinite)||g.view[2]<=g.view[0]||g.view[3]<=g.view[1]||!Number.isFinite(g.userUnit)||g.userUnit<=0||g.userUnit>1000||!Array.isArray(g.rectangles)||!g.rectangles.length||g.rectangles.length>100||!g.rectangles.every(r=>r.length===4&&r.every(Number.isFinite)&&r[0]>=0&&r[1]>=0&&r[2]>0&&r[3]>0))return false;
 return Object.keys(v).every(k=>['action','annotationType','color','text','comment','geometry'].includes(k));
}
export function parseSyncEntry(value:unknown,source:AnnotationSource,expectedHash?:string,documentId?:string):AnnotationSyncEntry {
 const e=value as AnnotationSyncEntry|null;
 if(!e||e.formatVersion!==1||!id(e.id)||!id(e.documentId)||documentId&&e.documentId!==documentId||!hash(e.pdfContentHash)||expectedHash&&e.pdfContentHash!==expectedHash||sourceKey(annotationSource(e.source))!==sourceKey(source)||!key(e.annotationKey)||!Number.isSafeInteger(e.revision)||e.revision<=0||!syncStates.includes(e.status)||typeof e.needsVerification!=='boolean'||!text(e.message,4000)||!(e.errorCode===null||typeof e.errorCode==='string'&&/^[A-Z_]{1,96}$/.test(e.errorCode))||!time(e.localUpdatedAt)||!time(e.lastSyncedAt)||!time(e.updatedAt)||e.updatedAt===null||!(e.local===null||validLocalInput(e.local)))throw Error('批注同步身份、修订或待处理输入无效。');
 for(const a of [e.base,e.remote])if(a!==null){parseAnnotationCache({formatVersion:1,source,pdfContentHash:e.pdfContentHash,fetchedAt:1,productVersion:null,libraryVersion:null,truncated:false,annotations:[a]},source,e.pdfContentHash);if(a.identity.annotationKey!==e.annotationKey)throw Error('同步快照包含其他批注。');}
 if(e.local&&e.local.action!=='create'&&e.local.annotationKey!==e.annotationKey||e.status==='SYNCED'&&(e.local||e.needsVerification)||e.needsVerification&&!e.local)throw Error('同步状态与本地意图矛盾。');return e;
}
export function parseSyncResponse(value:unknown,source:AnnotationSource,expectedHash?:string,documentId?:string):AnnotationSyncResponse {
 const r=value as AnnotationSyncResponse|null,s=r?.snapshot,scope=s?.scope;
 if(!r||!(r.activeId===null||id(r.activeId))||!s||!Array.isArray(s.entries)||s.entries.length>1000||!scope||![0,30,60,300].includes(scope.refreshInterval)||!time(scope.lastRefreshedAt)||!time(scope.lastAttemptAt)||!time(scope.lastSyncedAt)||!text(scope.message,4000))throw Error('批注同步快照或刷新设置无效。');
 const entries=s.entries.map(e=>parseSyncEntry(e,source,expectedHash,documentId));if(new Set(entries.map(e=>e.id)).size!==entries.length||new Set(entries.map(e=>e.annotationKey)).size!==entries.length)throw Error('同步记录身份重复。');
 if(!(r.diagnostic===null||r.diagnostic&&/^[A-Z_]{1,96}$/.test(r.diagnostic.code)&&text(r.diagnostic.message,4000)))throw Error('同步诊断格式无效。');
 if(!(r.outcome===null||r.outcome&&typeof r.outcome.verified==='boolean'&&key(r.outcome.annotationKey)&&text(r.outcome.message,4000)))throw Error('同步写入回执无效。');
 if(r.cache!==null){const h=expectedHash??r.cache.pdfContentHash;parseAnnotationCache(r.cache,source,h);}
 if(r.history!==null&&(!Array.isArray(r.history)||r.history.length>1000||!r.history.every(revisionMetadataValid)))throw Error('批注处理修订列表无效。');
 if(r.historicalEntry!==null)parseSyncEntry(r.historicalEntry,source,expectedHash,documentId);return {...r,snapshot:{entries,scope}};
}
export const syncStateNames:Record<AnnotationSyncState,string>={SYNCED:'已同步',LOCAL_DIRTY:'本地待提交',REMOTE_CHANGED:'远端已变化',CONFLICT:'存在冲突',FAILED:'处理失败',READ_ONLY:'只读 / 未授权'};
