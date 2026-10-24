import type {FormatAction} from './formatting';
const allowed=new Set<FormatAction>(['h1','h2','h3','h4','h5','h6','bold','italic','strike','highlight','bullet','ordered','task','quote','rule','link','table','code','codeBlock','math','mathBlock']);
export const recentFormatKey='researchclip.markdown.recent-formats.v1';
export function parseRecentFormats(value:string|null):FormatAction[]{try{const data:unknown=JSON.parse(value??'null');return Array.isArray(data)?[...new Set(data.filter((v):v is FormatAction=>typeof v==='string'&&allowed.has(v as FormatAction)))].slice(0,6):[];}catch{return [];}}
export const addRecentFormat=(recent:FormatAction[],action:FormatAction)=>[action,...recent.filter(v=>v!==action)].slice(0,6);
