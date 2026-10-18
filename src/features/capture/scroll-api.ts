import {invoke,isTauri} from '@tauri-apps/api/core';
import {parseCaptureInfo} from '../../shared/api/capture';
import type {CaptureInfo} from '../../shared/types/capture';
export interface ScrollTarget {id:string;title:string;automatic:boolean}
export interface ScrollProgress {sessionId:string;state:'ready'|'capturing'|'paused'|'encoding'|'done'|'cancelled'|'failed';frames:number;width:number;height:number;automatic:boolean;message:string;result:CaptureInfo|null}
const record=(v:unknown):v is Record<string,unknown>=>Boolean(v)&&typeof v==='object';
export function parseTargets(value:unknown):ScrollTarget[]{
  if(!Array.isArray(value)||value.length>256||value.some(t=>!record(t)||typeof t.id!=='string'||!/^\d+-\d+$/.test(t.id)||typeof t.title!=='string'||t.title.length>180||typeof t.automatic!=='boolean')||new Set(value.map(t=>t.id)).size!==value.length)throw new Error('目标窗口列表无效，请刷新重试。');
  return value as ScrollTarget[];
}
export function parseProgress(value:unknown):ScrollProgress{
  if(!record(value)||typeof value.sessionId!=='string'||!/^\d+-\d+$/.test(value.sessionId)||!['ready','capturing','paused','encoding','done','cancelled','failed'].includes(String(value.state))||typeof value.message!=='string'||typeof value.automatic!=='boolean'||!Number.isInteger(value.frames)||Number(value.frames)<0||Number(value.frames)>64||!Number.isInteger(value.width)||!Number.isInteger(value.height)||Number(value.width)<128||Number(value.width)>4096||Number(value.height)<160||Number(value.height)>30000||Number(value.width)*Number(value.height)>16777216)throw new Error('长截图进度无效，请取消后重试。');
  return {...value,result:value.result===null?null:parseCaptureInfo(value.result)} as ScrollProgress;
}
async function call(name:string,args?:Record<string,unknown>){if(!isTauri())throw new Error('滚动长截图需要启动Windows桌面应用。');return invoke<unknown>(name,args);}
export const scrollTargets=async()=>parseTargets(await call('scroll_targets'));
export const prepareScroll=async(targetId:string,automatic:boolean)=>parseProgress(await call('scroll_prepare',{targetId,automatic}));
export const startScroll=async(sessionId:string)=>{await call('scroll_start',{sessionId});};
export const scrollStatus=async(sessionId:string)=>parseProgress(await call('scroll_status',{sessionId}));
export const scrollAction=async(sessionId:string,action:'next'|'stop'|'cancel')=>{await call('scroll_action',{sessionId,action});};

export const currentScroll=async()=>{const value=await call('scroll_status',{sessionId:null});return value===null?null:parseProgress(value);};
