import {invoke} from '@tauri-apps/api/core';
export interface OCRContent {screenshotId:string;recognizedText:string;language:string;recognitionStatus:'running'|'completed'|'failed';ocrEngineVersion:string;updatedAt:number;sourceImageRevision:string;revision:number;error:string|null;corrected:boolean}
export interface IndexProgress {phase:string;total:number;completed:number;failed:number;current:string|null;error:string|null;generation:number}
export function parseOCRContent(v:unknown):OCRContent|null {
  if(v===null)return null;
  if(!v||typeof v!=='object')throw Error('OCR记录无效。');const d=v as Record<string,unknown>;
  if(typeof d.screenshotId!=='string'||!/^[a-f0-9]{32}$/.test(d.screenshotId)||typeof d.recognizedText!=='string'||new TextEncoder().encode(d.recognizedText).length>1_048_576||d.recognizedText.includes('\0')||!['running','completed','failed'].includes(String(d.recognitionStatus))||typeof d.language!=='string'||typeof d.ocrEngineVersion!=='string'||d.sourceImageRevision!==`original-v1:${d.screenshotId}`||!Number.isSafeInteger(d.revision)||Number(d.revision)<1||!Number.isSafeInteger(d.updatedAt)||(d.error!==null&&typeof d.error!=='string')||typeof d.corrected!=='boolean')throw Error('OCR记录格式或版本无效。');
  return d as unknown as OCRContent;
}
export const getOCRContent=async(id:string)=>parseOCRContent(await invoke('get_ocr_content',{id}));
export const correctOCRContent=async(id:string,revision:number,text:string)=>{const d=parseOCRContent(await invoke('correct_ocr_content',{id,revision,text}));if(!d)throw Error('OCR保存响应无效。');return d;};
export async function progressCall(command:string,args?:Record<string,unknown>):Promise<IndexProgress>{
  const p=await invoke<IndexProgress>(command,args);
  if(!p||!['','running','paused','cancelling','cancelled','completed'].includes(p.phase)||![p.total,p.completed,p.failed,p.generation].every(n=>Number.isSafeInteger(n)&&n>=0)||p.total>5000||p.completed+p.failed>p.total||(p.current!==null&&(typeof p.current!=='string'||!/^[a-f0-9]{32}$/.test(p.current)))||(p.error!==null&&typeof p.error!=='string'))throw Error('OCR任务响应无效。');return p;
}
export const startIndex=(language:string,id:string|null,failedOnly=false)=>progressCall('start_ocr_index',{language,id,failedOnly});
export const indexStatus=()=>progressCall('ocr_index_status');
export const indexAction=(action:'pause'|'resume'|'cancel')=>progressCall('ocr_index_action',{action});
export const rebuildIndex=()=>invoke('rebuild_search_index');
