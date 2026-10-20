import {create} from 'zustand';
import * as api from './api';
import {errorMessage} from '../../shared/api/errors';
interface Source {id:string;title:string}
interface State {open:boolean;scope:api.SearchScope;source:Source|null;exactOnly:boolean;threshold:number;colorLimit:number;page:number;status:api.FeatureStatus|null;matches:api.Matches|null;groups:api.Groups|null;busy:boolean;loading:boolean;error:string|null;generation:number;show:(source?:Source,exact?:boolean)=>void;close:()=>void;configure:(patch:Partial<Pick<State,'threshold'|'colorLimit'|'exactOnly'|'page'|'scope'>>)=>void;refresh:()=>Promise<void>;query:()=>Promise<void>;start:(rebuild:boolean)=>Promise<void>;cancel:()=>Promise<void>}
export function createSimilarityStore(){return create<State>((set,get)=>{
 let reading=false;
 return {open:false,scope:{projectId:null,unclassified:false,favorites:false},source:null,exactOnly:false,threshold:10,colorLimit:35,page:0,status:null,matches:null,groups:null,busy:false,loading:false,error:null,generation:0,
 show:(source,exact=false)=>set({open:true,source:source??null,exactOnly:exact,page:0,matches:null,groups:null,error:null,generation:get().generation+1}),
 close:()=>set({open:false,loading:false,generation:get().generation+1}),
 configure:patch=>{const p={...patch};if(p.threshold!==undefined)p.threshold=Math.max(0,Math.min(64,Math.round(p.threshold)));if(p.colorLimit!==undefined)p.colorLimit=Math.max(0,Math.min(255,Math.round(p.colorLimit)));set({...p,matches:null,groups:null,page:p.page??0,generation:get().generation+1});},
 refresh:async()=>{if(reading)return;reading=true;try{set({status:await api.status()});}catch(e){set({error:errorMessage(e,'索引状态读取失败。')});}finally{reading=false;}},
 query:async()=>{const s=get(),generation=s.generation;set({loading:true,error:null});try{if(s.source){const matches=await api.find(s.source.id,s.exactOnly,s.threshold,s.colorLimit,s.page,s.scope);if(get().generation===generation)set({matches});}else{const groups=await api.groups(s.page,s.scope);if(get().generation===generation)set({groups});}}catch(e){if(get().generation===generation)set({matches:null,groups:null,error:errorMessage(e,'相似图片检索失败。')});}finally{if(get().generation===generation)set({loading:false});}},
 start:async rebuild=>{if(get().busy)return;set({busy:true,error:null});try{await api.start(rebuild);await get().refresh();}catch(e){set({error:errorMessage(e,'索引启动失败。')});}finally{set({busy:false});}},
 cancel:async()=>{try{await api.cancel();await get().refresh();}catch(e){set({error:errorMessage(e,'取消失败，请重试。')});}},
 };
});}
export const useSimilarityStore=createSimilarityStore();
