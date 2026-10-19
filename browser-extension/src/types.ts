export type Mode='link'|'selection'|'article'|'images';
export interface ClipImage {id:string;url:string;alt:string;pngBase64:string|null;warning:string|null;width:number;height:number;thumbnail:string|null}
export interface Extracted {title:string;url:string;siteName:string;byline:string;mode:Mode;markdown:string;images:ClipImage[]}
export interface ClipRequest {version:1;requestId:string;operation:'projects'|'save'|'ping';clip:({title:string;url:string;siteName:string;byline:string;mode:Mode;markdown:string;projectId:string|null;images:{id:string;url:string;alt:string;pngBase64:string|null;warning:string|null}[]})|null}
export interface Reply {ok:boolean;code:string;message:string;data:{projects?:{id:string;name:string}[];documentId?:string;duplicate?:boolean}}
export const newId=()=>crypto.randomUUID().replaceAll('-','');
export function safeUrl(value:string,base?:string):string|null {try{const u=new URL(value,base);return value.length<=4096&&['https:','http:'].includes(u.protocol)&&!u.username&&!u.password&&!Array.from(value).some(c=>c.charCodeAt(0)<32)?u.href:null;}catch{return null;}}
export function publicImageUrl(value:string):boolean {
 const url=safeUrl(value);if(!url)return false;const host=new URL(url).hostname.toLowerCase().replace(/^\[|\]$/g,'');
 if(host==='localhost'||host.endsWith('.localhost')||host.endsWith('.local')||!host.includes('.')||host.includes(':'))return false;
 if(/^\d+\.\d+\.\d+\.\d+$/.test(host)){const [a,b]=host.split('.').map(Number);if(a===0||a===10||a===127||a>=224||(a===169&&b===254)||(a===172&&b>=16&&b<=31)||(a===192&&b===168)||(a===100&&b>=64&&b<=127))return false;}
 return true;
}
export function requestFits(request:ClipRequest){return new TextEncoder().encode(JSON.stringify(request)).length<=700*1024;}
export interface Pending {at:number;fingerprint:string;copyImages:boolean;request:ClipRequest}
export function clipFingerprint(c:Extracted,title:string,project:string,copy:boolean){return JSON.stringify({url:c.url,mode:c.mode,markdown:c.markdown,images:c.images.map(i=>({id:i.id,url:i.url,alt:i.alt})),title:title.trim(),project,copy});}
export function restorePending(value:unknown):Pending|null {
 if(!value||typeof value!=='object')return null;const p=value as Partial<Pending>,r=p.request,c=r?.clip;
 if(!Number.isFinite(p.at)||typeof p.at!=='number'||Date.now()-p.at>600000||p.at>Date.now()+1000||typeof p.fingerprint!=='string'||p.fingerprint.length>500*1024||typeof p.copyImages!=='boolean'||!r||r.version!==1||r.operation!=='save'||!(/^[a-f0-9]{32}$/.test(r.requestId))||!c)return null;
 if(typeof c.title!=='string'||!c.title.trim()||c.title.length>400||typeof c.url!=='string'||!safeUrl(c.url)||typeof c.siteName!=='string'||c.siteName.length>512||typeof c.byline!=='string'||c.byline.length>512||!['link','selection','article','images'].includes(c.mode)||typeof c.markdown!=='string'||new TextEncoder().encode(c.markdown).length>192*1024||!(c.projectId===null||(typeof c.projectId==='string'&&/^[a-f0-9]{32}$/.test(c.projectId)))||!Array.isArray(c.images)||c.images.length>32)return null;
 for(const i of c.images)if(!i||typeof i.id!=='string'||!/^[a-f0-9]{32}$/.test(i.id)||typeof i.url!=='string'||!safeUrl(i.url)||typeof i.alt!=='string'||i.alt.length>800||!(i.pngBase64===null||(typeof i.pngBase64==='string'&&i.pngBase64.length<=350000))||!(i.warning===null||(typeof i.warning==='string'&&i.warning.length<=512)))return null;
 return requestFits(r)?p as Pending:null;
}
