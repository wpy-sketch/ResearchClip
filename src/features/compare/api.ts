import {invoke,isTauri} from '@tauri-apps/api/core';
import {getScreenshot,screenshotImage} from '../../shared/api/library';
import {parsePngSize} from '../../shared/api/capture';
export interface LoadedComparison {name:string;format:string;blob:Blob;widthPx:number;heightPx:number}
export async function comparisonFromLibrary(id:string):Promise<LoadedComparison>{const s=await getScreenshot(id),blob=await screenshotImage(id);return {name:s.title,format:'PNG',widthPx:s.widthPx,heightPx:s.heightPx,blob};}
export async function chooseComparisonFile():Promise<LoadedComparison|null>{
  if(!isTauri())throw new Error('本地图片选择需要桌面应用。');const data:unknown=await invoke('choose_comparison_image');
  if(!(data instanceof ArrayBuffer)||data.byteLength>68*1024*1024)throw new Error('对比图片响应无效。');if(!data.byteLength)return null;
  if(data.byteLength<4)throw new Error('对比图片封包不完整。');const size=new DataView(data).getUint32(0,true);if(size>4096||size+4>=data.byteLength)throw new Error('对比图片头无效。');
  const meta:unknown=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(new Uint8Array(data,4,size)));if(!meta||typeof meta!=='object'||!('name'in meta)||typeof meta.name!=='string'||!('format'in meta)||typeof meta.format!=='string'||!['PNG','JPEG','WebP'].includes(meta.format))throw new Error('对比图片信息无效。');
  const png=data.slice(4+size),dimensions=parsePngSize(png);return {name:meta.name,format:meta.format,blob:new Blob([png],{type:'image/png'}),...dimensions};
}
