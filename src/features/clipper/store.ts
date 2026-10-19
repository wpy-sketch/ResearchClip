import {create} from 'zustand';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {errorMessage} from '../../shared/api/errors';
export interface SavedClip {documentId:string;title:string;duplicate:boolean}
interface Status {enabled:boolean;extensionId:string;last:SavedClip|null}
interface State {status:Status|null;busy:boolean;error:string|null;notice:SavedClip|null;refresh:()=>Promise<void>;connect:(enabled:boolean)=>Promise<void>;receive:(value:unknown)=>void;dismiss:()=>void}
export function parseSavedClip(value:unknown):SavedClip {
 if(!value||typeof value!=='object'||!('documentId'in value)||typeof value.documentId!=='string'||!/^[a-f0-9]{32}$/.test(value.documentId)||!('title'in value)||typeof value.title!=='string'||value.title.length>400||!('duplicate'in value)||typeof value.duplicate!=='boolean')throw Error('剪藏响应无效。');
 return {documentId:value.documentId,title:value.title,duplicate:value.duplicate};
}
export const useClipperStore=create<State>((set,get)=>({status:null,busy:false,error:null,notice:null,
 refresh:async()=>{if(!isTauri())return;try{const status=await invoke<Status>('clipper_status');set({status});}catch(e){set({error:errorMessage(e,'无法读取网页剪藏状态。')});}},
 connect:async(enabled)=>{if(!isTauri()||get().busy)return;set({busy:true,error:null});try{const status=await invoke<Status>('clipper_connection',{enabled});set({status});}catch(e){set({error:errorMessage(e,'网页剪藏连接失败。')});}finally{set({busy:false});}},
 receive:value=>{try{set({notice:parseSavedClip(value),error:null});}catch(e){set({error:errorMessage(e,'剪藏通知无效。')});}},dismiss:()=>set({notice:null}),
}));
