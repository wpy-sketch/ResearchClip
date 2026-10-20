import {parseSyncConfig,type SyncConfig,type SyncStatus} from './api';
export const syncDefaults:SyncConfig={server:'',folder:'researchclip-sync-v2',projects:[],unclassified:false};
export function normalizeConfig(config:SyncConfig):SyncConfig{return {...config,server:config.server.trim(),folder:config.folder.trim(),projects:[...config.projects].sort()};}
export function sameConfig(a:SyncConfig,b:SyncConfig){return JSON.stringify(normalizeConfig(a))===JSON.stringify(normalizeConfig(b));}
export function sameConnection(a:SyncConfig,b:SyncConfig|null|undefined){return Boolean(b&&a.server.trim()===b.server&&a.folder.trim()===b.folder);}
export function connectionIssue(config:SyncConfig,username:string,password:string,status:SyncStatus|null):{field:string;message:string}|null {
 const c=normalizeConfig(config);let url:URL;
 try{url=new URL(c.server);}catch{return {field:'WebDAV服务器',message:'请输入有效的 HTTPS 服务器目录，例如 https://dav.jianguoyun.com/dav/ResearchClip/。'};}
 if(url.protocol!=='https:'||url.username||url.password||url.search||url.hash||!c.server.endsWith('/')||c.server.length>2048)return {field:'WebDAV服务器',message:'服务器必须使用 HTTPS、以 / 结尾，且不包含用户名、密码、查询参数或令牌。'};
 if(c.folder.length<14||c.folder.length>80||!/^researchclip-[a-zA-Z0-9-]+$/.test(c.folder))return {field:'WebDAV专属目录',message:'专属子目录应为 14–80 个字符，以 researchclip- 开头，后接字母、数字或连字符。'};
 if(Boolean(username.trim())!==Boolean(password))return {field:username.trim()?'WebDAV应用密码':'WebDAV用户名',message:'更新凭据时请同时填写用户名和应用授权密码；同一连接已有凭据时可同时留空。'};
 if(!username.trim()&&!password&&!(sameConnection(c,status?.config)&&status?.credentialAvailable))return {field:'WebDAV用户名',message:'此连接尚无可用凭据，请填写用户名和应用授权密码，再保存。'};
 if(username.trim().length>512||password.length>1536)return {field:'WebDAV应用密码',message:'用户名或应用授权密码过长，请检查后重试。'};
 try{parseSyncConfig(c);}catch{return {field:'WebDAV专属目录',message:'同步范围或配置无效，请刷新项目后重新选择。'};}
 return null;
}
