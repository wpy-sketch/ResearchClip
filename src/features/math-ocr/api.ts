import {invoke,isTauri} from '@tauri-apps/api/core';
import type {OcrRegion} from '../ocr/model';
export interface MathJob {id:number;phase:''|'running'|'completed'|'cancelled'|'error';kind:string;result:{latex?:string;model?:string;verified?:false;downloaded?:boolean}|null;error:string|null;cancelRequested:boolean}
export interface MathStatus {configured:boolean;modelsPresent:boolean;modelBytes:number;job:MathJob;error:string|null}
export function validateLatex(text:string){if(!text.trim()||text.length>8192||Array.from(text).some(c=>c.charCodeAt(0)<32&&!['\n','\t','\r'].includes(c)))throw Error('LaTeX不能为空、超过8192字符或包含控制字符。');}
export function parseMathStatus(v:unknown):MathStatus{const s=v as MathStatus;if(!s||typeof s.configured!=='boolean'||typeof s.modelsPresent!=='boolean'||s.modelBytes!==178952787||!s.job||!Number.isSafeInteger(s.job.id)||s.job.id<0||!['','running','completed','cancelled','error'].includes(s.job.phase)||(s.error!==null&&typeof s.error!=='string')||(s.job.error!==null&&typeof s.job.error!=='string')||typeof s.job.cancelRequested!=='boolean')throw Error('公式任务状态无效。');if(s.job.result?.latex!==undefined){validateLatex(s.job.result.latex);if(s.job.result.verified!==false)throw Error('公式结果缺少未验证标识。');}return s;}
async function call(name:string,args?:Record<string,unknown>|Uint8Array):Promise<unknown>{if(!isTauri())throw Error('公式识别需要Windows桌面应用与本地运行时。');return invoke(name,args);}
export const mathStatus=async()=>parseMathStatus(await call('math_ocr_status'));
export const chooseMathRuntime=async()=>parseMathStatus(await call('choose_math_runtime'));
export const importMathModels=async()=>parseMathStatus(await call('import_math_models'));
export const cancelMath=()=>call('cancel_math_ocr');
export const downloadMathModels=()=>call('download_math_models',{confirmed:true});
export async function recognizeMath(blob:Blob,region:OcrRegion|null){const header=new TextEncoder().encode(JSON.stringify({region})),png=new Uint8Array(await blob.arrayBuffer());if(png.byteLength>68_157_440)throw Error('公式图片过大。');const packet=new Uint8Array(4+header.length+png.length);new DataView(packet.buffer).setUint32(0,header.length,true);packet.set(header,4);packet.set(png,4+header.length);const job=await call('recognize_math_ocr',packet) as MathJob;if(!job||!Number.isSafeInteger(job.id)||job.phase!=='running')throw Error('公式启动响应无效。');return job;}
