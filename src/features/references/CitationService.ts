import {invoke,isTauri} from '@tauri-apps/api/core';
import {readReferences,parseReference,parseInput,type Reference,type ReferenceInput} from './api';
export interface ExternalCitationKey{provider:'betterBibtex';instanceId:string;libraryId:number;externalKey:string}
async function call(name:string,args:Record<string,unknown>={}){if(!isTauri())throw Error('本地文献服务需要桌面应用。');return invoke<unknown>(name,args);}
export class CitationService{
 search(query:string,page=0){return readReferences(query,page);}
 async resolve(ids:string[]):Promise<Reference[]>{const value=await call('citation_references',{ids});if(!Array.isArray(value)||value.length>200)throw Error('文献解析响应无效。');const rows=value.map(parseReference);if(new Set(rows.map(r=>r.id)).size!==rows.length||rows.some(r=>!ids.includes(r.id)))throw Error('文献解析身份不匹配。');return rows;}
 async previewBibtex(text:string):Promise<ReferenceInput[]>{const value=await call('preview_bibtex',{text});if(!Array.isArray(value)||value.length>200)throw Error('导入预览响应无效。');return value.map(parseInput);}
 async exportBibtex(ids:string[]){const v=await call('export_bibtex',{ids});if(typeof v!=='boolean')throw Error('导出响应无效。');return v;}
 async bindExternalKey(key:ExternalCitationKey,id:string){await call('bind_citation_key',{key,id});}
 async resolveExternalKey(key:ExternalCitationKey){const v=await call('resolve_citation_key',{key});if(v!==null&&(typeof v!=='string'||!/^[a-f0-9]{32}$/.test(v)))throw Error('外部引用键映射无效。');return v as string|null;}
}
export const citations=new CitationService();
