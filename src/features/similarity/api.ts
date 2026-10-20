import {invoke,isTauri} from '@tauri-apps/api/core';
export interface FeatureJob {phase:string;total:number;scanned:number;computed:number;skipped:number;failed:number;error:string|null;generation:number}
export interface FeatureStatus {stats:{total:number;ready:number;failed:number;algorithmVersion:number};job:FeatureJob}
export interface SimilarMatch {id:string;title:string;widthPx:number;heightPx:number;createdAt:number;exact:boolean;distance:number;differenceDistance:number;colorDistance:number}
export interface Matches {items:SimilarMatch[];total:number;page:number;stale:number}
export interface Groups {items:{referenceId:string;contentHash:string;count:number;title:string}[];total:number;page:number}
const object=(v:unknown):v is Record<string,unknown>=>!!v&&typeof v==='object'&&!Array.isArray(v);
const count=(v:unknown):v is number=>Number.isSafeInteger(v)&&Number(v)>=0;
const id=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{32}$/.test(v);
const label=(v:unknown):v is string=>typeof v==='string'&&Array.from(v).length<=200;
function invalid():never{throw Error('相似检索响应无效，请更新应用或重试；素材未修改。');}
export function parseJob(v:unknown):FeatureJob{if(!object(v)||!['','running','cancelling','completed','cancelled','failed'].includes(String(v.phase))||!['total','scanned','computed','skipped','failed','generation'].every(k=>count(v[k]))||v.error!==null&&(typeof v.error!=='string'||v.error.length>2048))return invalid();return v as unknown as FeatureJob;}
export function parseStatus(v:unknown):FeatureStatus{if(!object(v)||!object(v.stats))return invalid();const stats=v.stats;if(!['total','ready','failed'].every(k=>count(stats[k]))||stats.algorithmVersion!==1)return invalid();return {stats:stats as unknown as FeatureStatus['stats'],job:parseJob(v.job)};}
export function parseMatches(v:unknown):Matches{if(!object(v)||!count(v.total)||!count(v.page)||v.page>=200||!count(v.stale)||!Array.isArray(v.items)||v.items.length>25||!v.items.every(x=>object(x)&&id(x.id)&&label(x.title)&&count(x.widthPx)&&Number(x.widthPx)>0&&count(x.heightPx)&&Number(x.heightPx)>0&&count(x.createdAt)&&typeof x.exact==='boolean'&&count(x.distance)&&x.distance<=64&&count(x.differenceDistance)&&x.differenceDistance<=64&&count(x.colorDistance)&&x.colorDistance<=255))return invalid();return v as unknown as Matches;}
export function parseGroups(v:unknown):Groups{if(!object(v)||!count(v.total)||!count(v.page)||v.page>=5000||!Array.isArray(v.items)||v.items.length>20||!v.items.every(x=>object(x)&&id(x.referenceId)&&typeof x.contentHash==='string'&&/^[a-f0-9]{64}$/.test(x.contentHash)&&count(x.count)&&x.count>=2&&label(x.title)))return invalid();return v as unknown as Groups;}
async function call(command:string,args?:Record<string,unknown>){if(!isTauri())throw Error('本地相似检索需要Windows桌面版；浏览器不会读取素材。');return invoke(command,args);}
export async function status(){return parseStatus(await call('similarity_status'));}
export async function start(rebuild:boolean,id:string|null=null){return parseJob(await call('start_similarity_index',{rebuild,id}));}
export async function cancel(){return parseJob(await call('cancel_similarity_index'));}
export async function find(id:string,exactOnly:boolean,threshold:number,colorLimit:number,page:number){return parseMatches(await call('find_similar_images',{id,exactOnly,threshold,colorLimit,page}));}
export async function groups(page:number){return parseGroups(await call('duplicate_image_groups',{page}));}
export interface AiStatus {available:boolean;model:string;job:{phase:string;text:string|null;error:string|null;generation:number;pair:[string,string]|null}}
export function parseAi(v:unknown):AiStatus{if(!object(v)||typeof v.available!=='boolean'||typeof v.model!=='string'||v.model.length>160||!object(v.job))return invalid();const j=v.job;if(!['','running','validating','completed','failed','cancelled'].includes(String(j.phase))||!count(j.generation)||j.text!==null&&(typeof j.text!=='string'||Array.from(j.text).length>8192)||j.error!==null&&(typeof j.error!=='string'||j.error.length>2048)||j.pair!==null&&(!Array.isArray(j.pair)||j.pair.length!==2||!j.pair.every(id)))return invalid();return v as unknown as AiStatus;}
export async function aiStatus(){return parseAi(await call('similarity_ai_status'));}
export async function chooseAi(){return parseAi(await call('choose_similarity_ai_bundle'));}
export async function startAi(a:string,b:string){return parseAi(await call('start_similarity_ai',{a,b}));}
export async function cancelAi(){return parseAi(await call('cancel_similarity_ai'));}
