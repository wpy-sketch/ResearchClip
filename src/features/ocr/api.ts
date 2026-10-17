import {invoke,isTauri} from '@tauri-apps/api/core';
import {parsePngSize} from '../../shared/api/capture';
import {parseResult,parseStatus,validateText,type OcrRegion} from './model';
function desktop(){if(!isTauri())throw new Error('离线文字识别需要在 Windows 桌面应用中使用。');}
export async function ocrStatus(){desktop();return parseStatus(await invoke('ocr_status'));}
export async function recognizeOcr(blob:Blob,language:string,region:OcrRegion|null){
  desktop();const png=await blob.arrayBuffer();parsePngSize(png);
  const header=new TextEncoder().encode(JSON.stringify({language,region})),bytes=new Uint8Array(4+header.length+png.byteLength);
  new DataView(bytes.buffer).setUint32(0,header.length,true);bytes.set(header,4);bytes.set(new Uint8Array(png),4+header.length);
  return parseResult(await invoke('recognize_ocr',bytes));
}
export async function copyOcrText(text:string){desktop();validateText(text);await invoke('copy_ocr_text',{text});}
