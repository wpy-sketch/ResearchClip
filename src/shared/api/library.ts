import { invoke, isTauri } from '@tauri-apps/api/core';
import { parseDocument } from '../../features/editor/serialization';
import type { AnnotationDocument } from '../../features/editor/types';
import { parsePngSize } from './capture';
export interface ScreenshotSummary { id:string; title:string; originalImagePath:string; thumbnailPath:string; widthPx:number; heightPx:number; createdAt:number; updatedAt:number; isFavorite:boolean; revision:number; imageAvailable:boolean; thumbnailAvailable:boolean;matchExcerpt?:{source:string;text:string;keyword:string} }
export interface Screenshot extends Omit<ScreenshotSummary,'imageAvailable'|'thumbnailAvailable'> { annotationData:AnnotationDocument; annotationVersion?:number }
export const LIBRARY_PAGE_SIZE = 24;
export type LibrarySort = 'createdDesc' | 'createdAsc';
export interface ScreenshotPage {items:ScreenshotSummary[]; total:number; page:number; limit:number; sort:LibrarySort}
export interface LibraryStatus {schemaVersion:20; pendingWrites:number; pendingCleanup:number; warnings:string[]}
export interface Note {id:string;screenshotId:string;content:string;sourceUrl:string|null;createdAt:number;updatedAt:number;revision:number}
export interface Tag {id:string;name:string}
export type SearchSource='all'|'title'|'notes'|'tags'|'ocr';
export interface LibraryFilter {query:string;tagIds:string[];favorite:boolean|null;projectId?:string|null;unclassified?:boolean;source?:SearchSource}
export interface KnowledgeCard {screenshot:ScreenshotSummary;note:Note|null;tags:Tag[];additionalNoteCount:number}
export interface CardDraft {title:string;content:string;sourceUrl:string;tagNames:string[];isFavorite:boolean}
const object=(v:unknown):v is Record<string,unknown>=>Boolean(v)&&typeof v==='object'&&!Array.isArray(v);
const id=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{32}$/.test(v);
const integer=(v:unknown,min=0):v is number=>typeof v==='number'&&Number.isSafeInteger(v)&&v>=min;
function metadata(v:unknown) {
  if(!object(v)||!id(v.id)||typeof v.title!=='string'||!v.title.trim()||Array.from(v.title).length>200||v.originalImagePath!==`originals/${v.id}.png`||v.thumbnailPath!==`thumbnails/${v.id}.png`||!integer(v.widthPx,1)||!integer(v.heightPx,1)||v.widthPx*v.heightPx>16_777_216||!integer(v.createdAt)||!integer(v.updatedAt)||!integer(v.revision,1)||typeof v.isFavorite!=='boolean')throw new Error('素材记录响应无效。');
  return {id:v.id,title:v.title,originalImagePath:v.originalImagePath,thumbnailPath:v.thumbnailPath,widthPx:v.widthPx,heightPx:v.heightPx,createdAt:v.createdAt,updatedAt:v.updatedAt,isFavorite:v.isFavorite,revision:v.revision};
}
export function parseScreenshot(v:unknown):Screenshot {const meta=metadata(v);if(!object(v))throw new Error('素材记录无效。');const annotationData=parseDocument(v.annotationData);if(annotationData.image.widthPx!==meta.widthPx||annotationData.image.heightPx!==meta.heightPx)throw new Error('素材与标注尺寸不一致。');if(v.annotationVersion!==undefined&&!integer(v.annotationVersion,1))throw Error("标注版本编号无效。");return {...meta,annotationData,...(v.annotationVersion!==undefined?{annotationVersion:v.annotationVersion as number}:{})};}
export function parseSummary(v:unknown):ScreenshotSummary {const meta=metadata(v);if(!object(v)||typeof v.imageAvailable!=='boolean'||typeof v.thumbnailAvailable!=='boolean')throw new Error('素材列表响应无效。');if(v.matchExcerpt!==undefined&&(!object(v.matchExcerpt)||!['title','notes','tags','ocr'].includes(String(v.matchExcerpt.source))||typeof v.matchExcerpt.text!=='string'||Array.from(v.matchExcerpt.text).length>140||typeof v.matchExcerpt.keyword!=='string'||Array.from(v.matchExcerpt.keyword).length>200))throw Error('搜索摘要无效。');return {...meta,imageAvailable:v.imageAvailable,thumbnailAvailable:v.thumbnailAvailable,...(v.matchExcerpt?{matchExcerpt:v.matchExcerpt as {source:string;text:string;keyword:string}}:{})};}
export function parseStatus(v:unknown):LibraryStatus {if(!object(v)||v.schemaVersion!==20||!integer(v.pendingWrites)||!integer(v.pendingCleanup)||!Array.isArray(v.warnings)||v.warnings.some(s=>typeof s!=='string'))throw new Error('工作区状态无效。');return {schemaVersion:20,pendingWrites:v.pendingWrites,pendingCleanup:v.pendingCleanup,warnings:v.warnings as string[]};}
function desktop(){if(!isTauri())throw new Error('素材库持久化需要在桌面应用中使用。');}
async function call(name:string,args?:Record<string,unknown>|ArrayBuffer):Promise<unknown>{desktop();return invoke(name,args);}
export async function packScreenshot(blob:Blob,title:string,document:AnnotationDocument):Promise<ArrayBuffer>{
  const json=new TextEncoder().encode(JSON.stringify({title,document:parseDocument(document)})),png=await blob.arrayBuffer();
  if(json.byteLength>4*1024*1024||!png.byteLength||png.byteLength>16_777_216*4+1_048_576)throw new Error('素材数据超过大小上限。');
  const result=new ArrayBuffer(4+json.byteLength+png.byteLength);new DataView(result).setUint32(0,json.byteLength,true);new Uint8Array(result,4,json.byteLength).set(json);new Uint8Array(result,4+json.byteLength).set(new Uint8Array(png));return result;
}
export async function createScreenshot(blob:Blob,title:string,document:AnnotationDocument){return parseScreenshot(await call('create_screenshot',await packScreenshot(blob,title,document)));}
export async function getScreenshot(id:string){return parseScreenshot(await call('get_screenshot',{id}));}
export async function listScreenshots(offset=0,favorites=false){const v=await call('list_screenshots',{offset,limit:50,favorites});if(!Array.isArray(v)||v.length>50)throw new Error('素材列表响应无效。');return v.map(parseSummary);}
export function parseScreenshotPage(v:unknown):ScreenshotPage {
  if(!object(v)||!integer(v.total)||!integer(v.page)||v.limit!==LIBRARY_PAGE_SIZE||(v.sort!=='createdDesc'&&v.sort!=='createdAsc')||!Array.isArray(v.items)||v.page>Math.max(0,Math.ceil(v.total/LIBRARY_PAGE_SIZE)-1)||v.items.length!==Math.min(LIBRARY_PAGE_SIZE,v.total-v.page*LIBRARY_PAGE_SIZE))throw new Error('素材分页响应无效。');
  const items=v.items.map(parseSummary);
  if(new Set(items.map(item=>item.id)).size!==items.length)throw new Error('素材分页包含重复记录。');
  return {items,total:v.total,page:v.page,limit:LIBRARY_PAGE_SIZE,sort:v.sort};
}
export async function readScreenshotPage(page=0,favorites=false,sort:LibrarySort='createdDesc',filter?:LibraryFilter) {
  if(!integer(page)||page>1_000_000||(sort!=='createdDesc'&&sort!=='createdAsc'))throw new Error('素材分页参数无效。');
  if(filter?.source&&!['all','title','notes','tags','ocr'].includes(filter.source))throw Error('搜索来源无效。');
  if(filter&&(Array.from(filter.query).length>200||filter.query.includes('\0')||filter.tagIds.length>100||filter.tagIds.some(value=>!id(value))||new Set(filter.tagIds).size!==filter.tagIds.length||(filter.favorite!==null&&typeof filter.favorite!=='boolean')))throw new Error('搜索筛选参数无效。');
  const result=parseScreenshotPage(await call('screenshot_page',{page,limit:LIBRARY_PAGE_SIZE,favorites,sort,...(filter?{filter}:{})}));
  if(result.sort!==sort||result.page>page)throw new Error('素材分页响应与请求不一致。');
  return result;
}
export async function libraryStatus(reconcile=false){return parseStatus(await call(reconcile?'reconcile_library':'library_status'));}
export async function updateScreenshot(id:string,revision:number,patch:{title?:string;isFavorite?:boolean;annotationData?:AnnotationDocument}){return parseScreenshot(await call('update_screenshot',{id,revision,patch}));}
export async function deleteScreenshot(id:string,revision:number){const v=await call('delete_screenshot',{id,revision});if(!object(v)||v.deleted!==true||!integer(v.pendingCleanup))throw new Error('删除操作响应无效。');return {deleted:true as const,pendingCleanup:v.pendingCleanup};}
export async function screenshotImage(id:string,thumbnail=false){const data=await call('screenshot_image',{id,thumbnail});if(!(data instanceof ArrayBuffer))throw new Error('素材图片响应无效。');const size=parsePngSize(data);if(thumbnail&&Math.max(size.widthPx,size.heightPx)>512)throw new Error('缩略图尺寸无效。');return new Blob([data],{type:'image/png'});}
export function parseNote(v:unknown):Note {if(!object(v)||!id(v.id)||!id(v.screenshotId)||typeof v.content!=='string'||(v.sourceUrl!==null&&typeof v.sourceUrl!=='string')||!integer(v.createdAt)||!integer(v.updatedAt)||!integer(v.revision,1))throw new Error('笔记响应无效。');return {id:v.id,screenshotId:v.screenshotId,content:v.content,sourceUrl:v.sourceUrl as string|null,createdAt:v.createdAt,updatedAt:v.updatedAt,revision:v.revision};}
export function parseTag(v:unknown):Tag {if(!object(v)||!id(v.id)||typeof v.name!=='string'||!v.name.trim()||Array.from(v.name).length>50)throw new Error('标签响应无效。');return {id:v.id,name:v.name};}
export async function listNotes(screenshotId:string){const v=await call('list_notes',{screenshotId});if(!Array.isArray(v)||v.length>100)throw new Error('笔记列表无效。');return v.map(parseNote);}
export async function saveNote(input:{screenshotId:string;id?:string;revision?:number;content:string;sourceUrl:string|null}){return parseNote(await call('save_note',{input}));}
export async function deleteNote(id:string,revision:number){await call('delete_note',{id,revision});}
export async function listTags(){const v=await call('list_tags');if(!Array.isArray(v)||v.length>1000)throw new Error('标签列表无效。');return v.map(parseTag);}
export async function saveTag(name:string,id?:string){return parseTag(await call('save_tag',{name,id:id??null}));}
export async function deleteTag(id:string){await call('delete_tag',{id});}
export async function getScreenshotTags(id:string){const v=await call('get_screenshot_tags',{id});if(!Array.isArray(v)||v.length>100||v.some(t=>!object(t)||t.screenshotId!==id||!idValid(t.tagId)))throw new Error('标签关联响应无效。');return v as {screenshotId:string;tagId:string}[];}
function idValid(value:unknown){return id(value);}
export async function setScreenshotTags(id:string,revision:number,tagIds:string[]){return parseScreenshot(await call('set_screenshot_tags',{id,revision,tagIds}));}
export function parseKnowledgeCard(value:unknown):KnowledgeCard {
  if(!object(value)||!Array.isArray(value.tags)||value.tags.length>100||!integer(value.additionalNoteCount)||value.additionalNoteCount>99)throw new Error('知识卡片响应无效。');
  const screenshot=parseSummary(value.screenshot),note=value.note===null?null:parseNote(value.note),tags=value.tags.map(parseTag);
  if((note&&note.screenshotId!==screenshot.id)||new Set(tags.map(t=>t.id)).size!==tags.length)throw new Error('知识卡片关联响应无效。');
  return {screenshot,note,tags,additionalNoteCount:value.additionalNoteCount};
}
export async function getKnowledgeCard(id:string){return parseKnowledgeCard(await call('get_knowledge_card',{id}));}
export async function saveKnowledgeCard(card:KnowledgeCard,draft:CardDraft){
  return parseKnowledgeCard(await call('save_knowledge_card',{input:{id:card.screenshot.id,revision:card.screenshot.revision,title:draft.title,isFavorite:draft.isFavorite,noteId:card.note?.id??null,noteRevision:card.note?.revision??null,content:draft.content,sourceUrl:draft.sourceUrl.trim()||null,tagNames:draft.tagNames}}));
}
export async function openCardSource(id:string){await call('open_card_source',{id});}
