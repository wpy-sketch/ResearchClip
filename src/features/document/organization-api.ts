import {revisionMetadataValid,type RevisionMetadata} from '../../shared/revisions/model';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {decodeDocument} from './api';
export interface MarkdownRevision extends RevisionMetadata{documentId:string;contentHash:string;restoredFrom:string|null}
export interface DocumentFolder{id:string;parentId:string|null;name:string}
export interface OrganizedDocument{id:string;name:string;folderId:string|null;favorite:boolean;lastOpenedAt:number;available:boolean}
export interface DocumentDirectory{folders:DocumentFolder[];documents:OrganizedDocument[]}
const id=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{32}$/.test(v);
const object=(v:unknown):v is Record<string,unknown>=>!!v&&typeof v==='object'&&!Array.isArray(v);
const integer=(v:unknown)=>Number.isSafeInteger(v)&&Number(v)>=0;
export function parseDirectory(value:unknown):DocumentDirectory{if(!object(value)||!Array.isArray(value.folders)||value.folders.length>1000||!Array.isArray(value.documents)||value.documents.length>5000)throw Error('文档目录响应无效。');const folders=value.folders.map(v=>{if(!object(v)||!id(v.id)||!(v.parentId===null||id(v.parentId))||typeof v.name!=='string'||!v.name.trim()||v.name.length>160)throw Error('目录文件夹无效。');return v as unknown as DocumentFolder;});const keys=new Set(folders.map(f=>f.id));if(keys.size!==folders.length)throw Error('目录包含重复ID。');for(const folder of folders){const seen=new Set<string>();let current:DocumentFolder|undefined=folder;while(current){if(seen.has(current.id)||seen.size>=8)throw Error('目录循环或过深。');seen.add(current.id);if(current.parentId&&!keys.has(current.parentId))throw Error('目录父节点缺失。');current=folders.find(f=>f.id===current!.parentId);}}
 const documents=value.documents.map(v=>{if(!object(v)||!id(v.id)||typeof v.name!=='string'||!(v.folderId===null||id(v.folderId)&&keys.has(v.folderId))||typeof v.favorite!=='boolean'||typeof v.available!=='boolean'||!integer(v.lastOpenedAt))throw Error('文档目录项无效。');return v as unknown as OrganizedDocument;});if(new Set(documents.map(d=>d.id)).size!==documents.length)throw Error('文档ID重复。');return {folders,documents};}
export function parseMarkdownRevision(v:unknown):MarkdownRevision{if(!object(v)||!revisionMetadataValid(v)||!id(v.documentId)||!(v.restoredFrom===null||id(v.restoredFrom))||typeof v.contentHash!=='string'||!/^[a-f0-9]{64}$/.test(v.contentHash))throw Error('文档版本响应无效。');return v as unknown as MarkdownRevision;}
async function call(name:string,args?:Record<string,unknown>):Promise<unknown>{if(!isTauri())throw Error('文档目录与持久历史需要桌面应用。');return invoke(name,args);}
export const documentDirectory=async()=>parseDirectory(await call('document_directory'));
export async function createDocumentFolder(parent:string|null,name:string){const v=await call('create_document_folder',{parent,name});if(!id(v))throw Error('文件夹ID无效。');return v;}
export async function deleteDocumentFolder(id:string){await call('delete_document_folder',{id});}
export async function organizeDocument(id:string,folder:string|null,favorite:boolean){await call('organize_document',{id,folder,favorite});}
export async function relinkMarkdownDocument(id:string){const v=await call('relink_markdown_document',{id});if(typeof v!=='boolean')throw Error('定位结果无效。');return v;}
export async function markdownHistory(id:string,before=Number.MAX_SAFE_INTEGER){const v=await call('markdown_history',{id,before});if(!Array.isArray(v)||v.length>50)throw Error('历史列表响应无效。');const rows=v.map(parseMarkdownRevision);if(rows.some((r,i)=>r.documentId!==id||r.versionNumber>=before||i>0&&r.versionNumber>=rows[i-1].versionNumber))throw Error('历史归属或顺序无效。');return rows;}
export async function loadMarkdownRevision(token:string,id:string,revisionId:string){const disk=decodeDocument(await call('load_markdown_revision',{token,id,revisionId}));if(!disk||disk.token!==token)throw Error('历史文件授权响应不匹配。');return disk;}

export async function markdownRevisionText(id:string,revisionId:string){const v=await call('markdown_revision_text',{id,revisionId});if(!object(v)||typeof v.markdown!=='string'||new TextEncoder().encode(v.markdown).length>2*1024*1024)throw Error('历史正文响应无效。');const revision=parseMarkdownRevision(v.revision);if(revision.documentId!==id||revision.revisionId!==revisionId||revision.schemaVersion!==1)throw Error('历史版本不支持或归属不匹配。');return {revision,markdown:v.markdown};}
