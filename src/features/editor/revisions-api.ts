import {revisionMetadataValid,type RevisionMetadata} from '../../shared/revisions/model';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {parseScreenshot} from '../../shared/api/library';
import {parseDocument} from './serialization';
import type {AnnotationDocument} from './types';
export interface AnnotationRevision extends RevisionMetadata {screenshotId:string}
const id=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{32}$/.test(v);
export function parseRevision(v:unknown):AnnotationRevision {
 if(!v||typeof v!=='object')throw Error('版本元数据无效。');const r=v as AnnotationRevision;
 if(!revisionMetadataValid(r)||!id(r.screenshotId))throw Error('版本元数据无效。');return {...r};
}
async function call(command:string,args:Record<string,unknown>):Promise<unknown>{if(!isTauri())throw Error('标注版本历史需要桌面应用。');return invoke(command,args);}
export async function revisionHistory(id:string,before=Number.MAX_SAFE_INTEGER){const v=await call('annotation_history',{id,before,limit:50});if(!Array.isArray(v)||v.length>50)throw Error('版本列表无效。');const rows=v.map(parseRevision);if(rows.some((r,i)=>r.screenshotId!==id||r.versionNumber>=before||(i>0&&r.versionNumber>=rows[i-1].versionNumber)))throw Error('版本列表顺序无效。');return rows;}
export async function revisionDocument(id:string,revisionId:string){const v=await call('annotation_revision',{id,revisionId}) as {revision:unknown;annotationJson:unknown};const revision=parseRevision(v?.revision);if(revision.screenshotId!==id||revision.revisionId!==revisionId||revision.schemaVersion!==1)throw Error('版本格式不支持或响应不匹配。');return {revision,document:parseDocument(v.annotationJson)};}
export async function saveRevision(id:string,expected:number,document:AnnotationDocument,description:string){return parseScreenshot(await call('save_annotation_revision',{id,expected,document:parseDocument(document),description}));}
export async function restoreRevision(id:string,expected:number,revisionId:string,description:string){return parseScreenshot(await call('restore_annotation_revision',{id,expected,revisionId,description}));}
