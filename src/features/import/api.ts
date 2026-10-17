import {invoke,isTauri} from '@tauri-apps/api/core';
import {parseScreenshot} from '../../shared/api/library';
import type {Screenshot} from '../../shared/api/library';
export const MAX_IMPORT_FILES=10,MAX_IMPORT_FILE_BYTES=20*1024*1024,MAX_IMPORT_BATCH_BYTES=64*1024*1024;
export interface ImportItem {name:string;screenshot:Screenshot|null;error:string|null}
export interface ImportReport {items:ImportItem[]}
export function validateFiles(files:readonly Pick<File,'name'|'size'>[]){
  if(!files.length||files.length>MAX_IMPORT_FILES)throw new Error('一次请导入1至10张图片。');
  if(files.some(f=>!Number.isSafeInteger(f.size)||f.size<=0||f.size>MAX_IMPORT_FILE_BYTES||f.name.length>255))throw new Error('单张图片需非空、名称有效且不超过20 MiB。');
  if(files.reduce((n,f)=>n+f.size,0)>MAX_IMPORT_BATCH_BYTES)throw new Error('一次导入的图片合计不能超过64 MiB。');
}
export async function packFiles(files:readonly File[]):Promise<ArrayBuffer>{
  validateFiles(files);const meta=new TextEncoder().encode(JSON.stringify({files:files.map(f=>({name:f.name,size:f.size}))}));if(meta.length>16*1024)throw new Error('文件信息过大。');
  const body=new ArrayBuffer(4+meta.length+files.reduce((n,f)=>n+f.size,0));new DataView(body).setUint32(0,meta.length,true);new Uint8Array(body,4,meta.length).set(meta);let offset=4+meta.length;
  for(const file of files){const bytes=await file.arrayBuffer();if(bytes.byteLength!==file.size)throw new Error('读取文件失败，文件可能已变化。');new Uint8Array(body,offset,file.size).set(new Uint8Array(bytes));offset+=file.size;}return body;
}
export function parseImportReport(v:unknown):ImportReport{
  if(!v||typeof v!=='object'||!('items'in v)||!Array.isArray(v.items)||!v.items.length||v.items.length>MAX_IMPORT_FILES)throw new Error('图片导入响应无效，请刷新素材库检查结果。');
  return {items:v.items.map(item=>{if(!item||typeof item!=='object'||typeof item.name!=='string'||!('screenshot'in item)||!('error'in item)||!((item.screenshot!==null&&item.error===null)||(item.screenshot===null&&typeof item.error==='string')))throw new Error('图片导入结果无效，请刷新素材库检查结果。');return {name:item.name,screenshot:item.screenshot===null?null:parseScreenshot(item.screenshot),error:item.error};})};
}
export async function importFiles(files:readonly File[]):Promise<ImportReport>{if(!isTauri())throw new Error('图片导入需要桌面应用。');return parseImportReport(await invoke('import_image_files',await packFiles(files)));}
export async function importClipboard():Promise<ImportReport>{if(!isTauri())throw new Error('剪贴板图片导入需要 Windows 桌面应用。');const screenshot=parseScreenshot(await invoke('import_clipboard_image'));return {items:[{name:'剪贴板图片',screenshot,error:null}]};}
