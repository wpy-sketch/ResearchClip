import {create} from 'zustand';
import {parsePngSize} from '../../shared/api/capture';
import {errorMessage} from '../../shared/api/errors';
import type {OcrRegion} from '../ocr/model';
import * as api from './api';
interface MathSource {blob:Blob;url:string;name:string;widthPx:number;heightPx:number;libraryId?:string}
interface State {source:MathSource|null;phase:'idle'|'preparing'|'recognizing'|'cancelling';region:OcrRegion|null;latex:string;confirmed:boolean;error:string|null;notice:string|null;jobId:number|null;status:api.MathStatus|null;
 open:(image:()=>Promise<Blob>,name:string,libraryId?:string)=>Promise<void>;close:()=>void;edit:(text:string)=>void;confirm:(value:boolean)=>void;setRegion:(region:OcrRegion|null)=>void;run:()=>Promise<void>;cancel:()=>Promise<void>;poll:()=>Promise<void>;}
export function createMathStore(){return create<State>((set,get)=>({source:null,phase:'idle',region:null,latex:'',confirmed:false,error:null,notice:null,jobId:null,status:null,
 open:async(image,name,libraryId)=>{if(get().source||get().phase!=='idle')return;set({phase:'preparing',error:null});let url:string|undefined;try{const blob=await image(),size=parsePngSize(await blob.arrayBuffer());url=URL.createObjectURL(blob);const status=await api.mathStatus();set({source:{blob,url,name,...size,...(libraryId?{libraryId}:{})},status,region:null,latex:'',confirmed:false,error:status.error,notice:null});}catch(e){if(url)URL.revokeObjectURL(url);set({error:errorMessage(e,'公式图片准备失败。')});}finally{set({phase:'idle'});}},
 close:()=>{if(get().phase!=='idle')return;const source=get().source;if(source)URL.revokeObjectURL(source.url);set({source:null,latex:'',region:null,error:null,notice:null,status:null,jobId:null,confirmed:false});},
 edit:latex=>{if(get().phase!=='idle')return;if(latex.length>8192){set({error:'LaTeX最多8192字符。'});return;}set({latex,confirmed:false,error:null,notice:null});},confirm:confirmed=>set({confirmed}),
 setRegion:region=>{if(get().phase==='idle')set({region});},
 run:async()=>{const s=get();if(!s.source||s.phase!=='idle')return;set({phase:'recognizing',error:null,notice:null,confirmed:false});try{const job=await api.recognizeMath(s.source.blob,s.region);set({jobId:job.id});}catch(e){set({phase:'idle',error:errorMessage(e,'公式识别启动失败。')});}},
 cancel:async()=>{if(!['recognizing','cancelling'].includes(get().phase))return;try{await api.cancelMath();set({phase:'cancelling'});}catch(e){set({error:errorMessage(e,'取消失败，请重试。')});}},
 poll:async()=>{const id=get().jobId;if(!id)return;try{const status=await api.mathStatus();if(get().jobId!==id)return;if(status.job.id!==id){set({phase:'idle',jobId:null,error:'公式任务已改变，当前草稿保留。'});return;}set({status});if(status.job.phase==='completed'){const text=status.job.result?.latex;if(typeof text!=='string')throw Error('公式结果缺失。');api.validateLatex(text);set({phase:'idle',latex:text,jobId:null,confirmed:false,notice:'识别结果未经数学正确性验证，请对照原图校对。'});}else if(status.job.phase==='error'||status.job.phase==='cancelled'){set({phase:'idle',jobId:null,error:status.job.error,notice:status.job.phase==='cancelled'?'已取消，原有结果保留。':null});}}catch(e){set({error:errorMessage(e,'公式状态读取失败，请重试或取消。')});}},
}));}
export const useMathStore=createMathStore();
export function formulaMarkdown(source:string,display=true){api.validateLatex(source);return display?`\n\n$$\n${source.trim()}\n$$\n\n`:`$${source.trim()}$`;}
