import {invoke,isTauri} from '@tauri-apps/api/core';
export interface ReferenceInput {title:string;authors:string[];year:number|null;journalOrConference:string;doi:string;url:string;bibtexKey:string;bibtexEntry:string;verifiedSource:'manual'|'crossref'|'bibtex'|'edited';sourceMetadata:unknown}
export interface Reference extends ReferenceInput{id:string;updatedAt:number;revision:number}
export interface Target{kind:'screenshot'|'document';id:string}
export interface ReferenceWrite{id:string|null;expectedRevision:number|null;data:ReferenceInput;confirmKeyChange:boolean}
export interface Page{items:Reference[];total:number}
const obj=(v:unknown):v is Record<string,unknown>=>!!v&&typeof v==='object'&&!Array.isArray(v);
const id=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{32}$/.test(v);
export const validKey=(s:string)=>/^[A-Za-z0-9_:-]{1,100}$/.test(s);
function invalid():never{throw Error('文献响应无效，已有笔记和文件未修改。');}
export function parseInput(v:unknown):ReferenceInput{if(!obj(v)||!['title','journalOrConference','doi','url','bibtexKey','bibtexEntry'].every(k=>typeof v[k]==='string')||String(v.title).length>8000||String(v.bibtexEntry).length>65536||!Array.isArray(v.authors)||v.authors.length>200||v.authors.some(a=>typeof a!=='string'||a.length>2000)||v.year!==null&&(!Number.isInteger(v.year)||Number(v.year)<1||Number(v.year)>9999)||!['manual','crossref','bibtex','edited'].includes(String(v.verifiedSource))||JSON.stringify(v.sourceMetadata??null).length>262144)return invalid();return {title:v.title as string,authors:v.authors as string[],year:v.year as number|null,journalOrConference:v.journalOrConference as string,doi:v.doi as string,url:v.url as string,bibtexKey:v.bibtexKey as string,bibtexEntry:v.bibtexEntry as string,verifiedSource:v.verifiedSource as ReferenceInput['verifiedSource'],sourceMetadata:v.sourceMetadata??null};}
export function parseReference(v:unknown):Reference {const input=parseInput(v);if(!obj(v)||!id(v.id)||!Number.isSafeInteger(v.updatedAt)||Number(v.updatedAt)<0||!Number.isSafeInteger(v.revision)||Number(v.revision)<1||!validKey(input.bibtexKey))return invalid();return v as unknown as Reference;}
export function parsePage(v:unknown):Page{if(!obj(v)||!Array.isArray(v.items)||v.items.length>25||!Number.isSafeInteger(v.total)||Number(v.total)<0)return invalid();return {items:v.items.map(parseReference),total:Number(v.total)};}
async function call(name:string,args?:Record<string,unknown>){if(!isTauri())throw Error('文献库需要 ResearchClip 桌面应用；离线笔记不受影响。');return invoke(name,args);}
export const readReferences=async(query='',page=0,target:Target|null=null)=>parsePage(await call('reference_page',{query,page,target}));
export const queryDoi=async(doi:string)=>parseInput(await call('lookup_reference_doi',{doi}));
export async function saveReferences(items:ReferenceWrite[]){const v=await call('save_references',{items:items.map(item=>({...item,data:parseInput(item.data)}))});if(!Array.isArray(v)||v.length>200)return invalid();return v.map(parseReference);}
export const associate=async(target:Target,id:string,linked:boolean)=>{await call('associate_reference',{target,id,linked});};
export async function importBib(){const v=await call('import_bibtex');if(v===null)return null;if(!Array.isArray(v)||v.length>200)return invalid();return v.map(parseInput);}
export const openDoi=async(id:string)=>{await call('open_reference_doi',{id});};
export const copyBib=async(text:string)=>{await call('copy_ocr_text',{text});};
export const blankReference=():ReferenceInput=>({title:'',authors:[],year:null,journalOrConference:'',doi:'',url:'',bibtexKey:`ref-${crypto.randomUUID().slice(0,8)}`,bibtexEntry:'',verifiedSource:'manual',sourceMetadata:{}});
export const citation=(key:string)=>{if(!validKey(key))throw Error('引用键无效。');return `[@${key}]`;};
