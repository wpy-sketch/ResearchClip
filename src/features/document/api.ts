import {invoke,isTauri} from '@tauri-apps/api/core';
import {assetNameValid,MAX_ASSET_BYTES,MAX_IMAGES,MAX_TEXT_BYTES} from './model';
import {parsePngSize} from '../../shared/api/capture';
import {normalizeMarkdownName} from './file-name';
export interface DiskDocument {token:string;path:string;markdown:string;assets:Record<string,Blob>}
const desktop=()=>{if(!isTauri())throw new Error('Markdown 文件操作需要桌面应用。');};
export async function releaseDocument(token:string|null){if(token&&isTauri())await invoke('release_markdown_document',{token});}
export async function saveDocumentFile(markdown:string,assets:Record<string,Blob>,token:string|null,saveAs:boolean,fileName='untitled.md'):Promise<{token:string;path:string}|null>{
  desktop();const body=await packDocument(markdown,assets,token,saveAs,fileName);
  const result:unknown=await invoke('save_markdown_document',body.buffer);
  if(result===null)return null;
  if(!result||typeof result!=='object'||!('token'in result)||!('path'in result)||typeof result.token!=='string'||typeof result.path!=='string')throw new Error('Markdown 保存响应无效。');
  return {token:result.token,path:result.path};
}
export async function packDocument(markdown:string,assets:Record<string,Blob>,token:string|null=null,saveAs=false,fileName='untitled.md'){
  const entries=Object.entries(assets),sizes=entries.map(([name,blob])=>({name,size:blob.size}));
  if(entries.length>MAX_IMAGES||entries.some(([name])=>!assetNameValid(name))||entries.reduce((sum,[,b])=>sum+b.size,0)>MAX_ASSET_BYTES)throw new Error('文档图片超出数量或128 MiB上限。');
  const json=new TextEncoder().encode(JSON.stringify({markdown,assets:sizes,token,saveAs,fileName:normalizeMarkdownName(fileName)}));
  if(json.length>MAX_TEXT_BYTES*2)throw new Error('文档数据过大。');
  const body=new Uint8Array(4+json.length+entries.reduce((sum,[,b])=>sum+b.size,0));new DataView(body.buffer).setUint32(0,json.length,true);body.set(json,4);let offset=4+json.length;
  for(const [,blob]of entries){body.set(new Uint8Array(await blob.arrayBuffer()),offset);offset+=blob.size;}
  return body;
}
export async function renameDocumentFile(token:string,fileName:string):Promise<{token:string;path:string}>{
  desktop();return invoke('rename_markdown_document',{token,fileName:normalizeMarkdownName(fileName)});
}
export async function openDocumentFile():Promise<DiskDocument|null>{
  desktop();return decodeDocument(await invoke('open_markdown_document'));
}
export function decodeDocument(buffer:unknown):DiskDocument|null{
  if(!(buffer instanceof ArrayBuffer))throw new Error('Markdown 文件响应无效。');if(!buffer.byteLength)return null;
  if(buffer.byteLength<4||buffer.byteLength>MAX_ASSET_BYTES+MAX_TEXT_BYTES*2+4)throw new Error('Markdown 文件响应大小无效。');
  const size=new DataView(buffer).getUint32(0,true);if(size>MAX_TEXT_BYTES*2||size+4>buffer.byteLength)throw new Error('Markdown 文件头无效。');
  const meta:unknown=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(new Uint8Array(buffer,4,size)));
  if(!meta||typeof meta!=='object'||!('token'in meta)||!('path'in meta)||!('markdown'in meta)||!('assets'in meta)||typeof meta.token!=='string'||typeof meta.path!=='string'||typeof meta.markdown!=='string'||!Array.isArray(meta.assets)||meta.assets.length>MAX_IMAGES)throw new Error('Markdown 文件信息无效。');
  const assets:Record<string,Blob>={};let offset=4+size;
  for(const a of meta.assets){if(!a||typeof a!=='object'||!assetNameValid(a.name)||!Number.isSafeInteger(a.size)||a.size<=0||offset+a.size>buffer.byteLength||assets[a.name])throw new Error('Markdown 图片封包无效。');const data=buffer.slice(offset,offset+a.size);parsePngSize(data);assets[a.name]=new Blob([data],{type:'image/png'});offset+=a.size;}
  if(offset!==buffer.byteLength)throw new Error('Markdown 封包有多余内容。');
  return {token:meta.token,path:meta.path,markdown:meta.markdown,assets};
}
