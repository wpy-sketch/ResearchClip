import {invoke,isTauri} from '@tauri-apps/api/core';
import type {LibraryFilter,LibrarySort} from '../../shared/api/library';
export interface Project {id:string;name:string;imageCount:number;documentCount:number}
export interface ProjectDocument {id:string;name:string;projectId:string|null;updatedAt:number;available:boolean}
export interface SelectionItem {id:string;revision:number}
export type BatchAction={kind:'move';projectId:string|null}|{kind:'tags';names:string[]}|{kind:'favorite';value:boolean}|{kind:'delete';confirmedCount:number};
const validId=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{32}$/.test(v);
const object=(v:unknown):v is Record<string,unknown>=>Boolean(v)&&typeof v==='object';
const count=(v:unknown):v is number=>Number.isSafeInteger(v)&&Number(v)>=0;
async function call(name:string,args?:Record<string,unknown>):Promise<unknown>{if(!isTauri())throw new Error('项目管理需要桌面应用。');return invoke(name,args);}
export function parseProjects(v:unknown):Project[]{if(!Array.isArray(v)||v.length>500)throw new Error('项目列表无效。');return v.map(p=>{if(!object(p)||!validId(p.id)||typeof p.name!=='string'||!p.name.trim()||!count(p.imageCount)||!count(p.documentCount))throw new Error('项目响应无效。');return {id:p.id,name:p.name,imageCount:p.imageCount,documentCount:p.documentCount};});}
export function parseProjectDocument(v:unknown):ProjectDocument{if(!object(v)||!validId(v.id)||typeof v.name!=='string'||!(v.projectId===null||validId(v.projectId))||!count(v.updatedAt)||typeof v.available!=='boolean')throw new Error('项目文档响应无效。');return {id:v.id,name:v.name,projectId:v.projectId,updatedAt:v.updatedAt,available:v.available};}
export function parseSelection(v:unknown):SelectionItem[]{if(!Array.isArray(v)||v.length>5000||v.some(i=>!object(i)||!validId(i.id)||!count(i.revision)||i.revision<1)||new Set(v.map(i=>i.id)).size!==v.length)throw new Error('素材选择无效或超过5000张。');return v.map(i=>({id:i.id,revision:i.revision}));}
export async function listProjects(){return parseProjects(await call('list_projects'));}
export async function saveProject(name:string,id:string|null=null){await call('save_project',{name,id});}
export async function deleteEmptyProject(id:string){await call('delete_empty_project',{id});}
export async function listProjectDocuments(projectId:string|null,unclassified:boolean){const v=await call('list_project_documents',{projectId,unclassified});if(!Array.isArray(v)||v.length>5000)throw new Error('项目文档列表无效。');return v.map(parseProjectDocument);}
export async function syncProjectDocument(token:string,opened=false,restoredFrom:string|null=null){return parseProjectDocument(await call('sync_project_document',{token,opened,restoredFrom}));}
export async function assignProjectDocument(id:string,projectId:string|null){await call('assign_project_document',{id,projectId});}
export async function selectLibraryScreenshots(favorites:boolean,sort:LibrarySort,filter:LibraryFilter){return parseSelection(await call('select_library_screenshots',{favorites,sort,filter}));}
export async function batchLibraryScreenshots(items:SelectionItem[],action:BatchAction){const selection=parseSelection(items);if(!items.length)throw new Error('请先选择图片。');const v=await call('batch_library_screenshots',{items:selection,action});if(!object(v)||v.affected!==items.length||!count(v.pendingCleanup))throw new Error('批量结果无效，请刷新资料库确认。');return {affected:items.length,pendingCleanup:v.pendingCleanup};}
