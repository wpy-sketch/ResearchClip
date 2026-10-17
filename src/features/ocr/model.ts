export const MAX_OCR_TEXT_BYTES=1024*1024;
export interface OcrRegion {x:number;y:number;width:number;height:number}
export interface OcrStatus {languages:{tag:string;name:string}[];maxDimension:number}
export interface OcrResult {text:string;language:string;widthPx:number;heightPx:number}
export function validateText(text:string){if(text.includes('\0')||new TextEncoder().encode(text).length>MAX_OCR_TEXT_BYTES)throw new Error('识别文字超过 1 MiB 或包含无效字符。');}
export function parseStatus(value:unknown):OcrStatus{
  const v=value as OcrStatus;
  if(!v||!Number.isInteger(v.maxDimension)||v.maxDimension<=0||!Array.isArray(v.languages)||v.languages.length>200||v.languages.some(l=>!l||typeof l.tag!=='string'||!/^[a-z0-9-]{1,64}$/i.test(l.tag)||typeof l.name!=='string'||l.name.length>200))throw new Error('OCR 环境响应无效。');
  return v;
}
export function parseResult(value:unknown):OcrResult{
  const v=value as OcrResult;
  if(!v||typeof v.text!=='string'||typeof v.language!=='string'||![v.widthPx,v.heightPx].every(n=>Number.isInteger(n)&&n>0&&n<=100000))throw new Error('OCR 结果响应无效。');
  validateText(v.text);return v;
}
export function regionFromDrag(start:{x:number;y:number},end:{x:number;y:number},display:{width:number;height:number},image:{widthPx:number;heightPx:number}):OcrRegion|null{
  if(![start.x,start.y,end.x,end.y,display.width,display.height,image.widthPx,image.heightPx].every(Number.isFinite)||display.width<=0||display.height<=0||image.widthPx<=0||image.heightPx<=0)return null;
  const x=(v:number)=>Math.max(0,Math.min(image.widthPx,v/display.width*image.widthPx)),y=(v:number)=>Math.max(0,Math.min(image.heightPx,v/display.height*image.heightPx));
  if(start.x===end.x||start.y===end.y)return null;
  const left=Math.floor(Math.min(x(start.x),x(end.x))),top=Math.floor(Math.min(y(start.y),y(end.y))),right=Math.ceil(Math.max(x(start.x),x(end.x))),bottom=Math.ceil(Math.max(y(start.y),y(end.y)));
  return right>left&&bottom>top?{x:left,y:top,width:right-left,height:bottom-top}:null;
}
