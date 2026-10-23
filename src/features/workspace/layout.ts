import type {IJsonModel} from 'flexlayout-react';
export const LAYOUT_VERSION=1,MAX_PANELS=12,MAX_MARKDOWN=3;
export type EditorKind='workbench'|'markdown'|'image'|'pdf'|'code'|'terminal';
export interface PanelRecord {id:string;kind:EditorKind;title:string;resourceId:string|null;hidden:boolean}
export interface WorkspaceLayout {version:1;workspaceId:string;explorer:boolean;panels:PanelRecord[];dock:IJsonModel}
const kinds:EditorKind[]=['workbench','markdown','image','pdf','code','terminal'];
const object=(v:unknown):v is Record<string,unknown>=>!!v&&typeof v==='object'&&!Array.isArray(v);
const identity=(v:unknown):v is string=>typeof v==='string'&&/^[a-zA-Z0-9#:_-]{1,128}$/.test(v)&&!['__proto__','prototype','constructor'].includes(v);
export const validWorkspaceId=(v:string)=>v==='default'||/^project:[a-f0-9]{32}$/.test(v);
export const dockDefaults={tabEnablePopout:false,tabEnableFloat:false,tabSetEnableClose:true,tabSetEnableCloseButton:false,tabSetEnableTabGroups:false,tabSetEnableMaximize:true,tabSetMinWidth:160,tabSetMinHeight:120,splitterSize:7,tabEnableRename:false,tabSetEnableDeleteWhenEmpty:true};
export function defaultDock(panels:PanelRecord[]):IJsonModel{return {global:dockDefaults,borders:[],layout:{type:'row',id:'root',children:[{type:'tabset',id:'main',active:true,children:panels.filter(p=>!p.hidden).map(p=>({type:'tab',id:p.id,name:p.title,component:'workspace',enableClose:p.kind!=='workbench'}))}]}};}
/** Rebuild only known layout fields. Never deserialize routes, paths, grants, component names or executable content. */
export function validateLayout(value:unknown):WorkspaceLayout{
 if(!object(value)||value.version!==LAYOUT_VERSION||typeof value.workspaceId!=='string'||!validWorkspaceId(value.workspaceId)||typeof value.explorer!=='boolean'||!Array.isArray(value.panels)||value.panels.length>MAX_PANELS)throw Error('工作区布局版本或结构不受支持；原记录已保留。');
 const ids=new Set<string>();
 const panels:PanelRecord[]=value.panels.map(p=>{if(!object(p)||!identity(p.id)||ids.has(p.id)||!kinds.includes(p.kind as EditorKind)||typeof p.title!=='string'||p.title.length>160||Array.from(p.title).some(c=>c.charCodeAt(0)<32)||typeof p.hidden!=='boolean'||!(p.resourceId===null||typeof p.resourceId==='string'&&/^[a-f0-9]{32}$/.test(p.resourceId)))throw Error('工作区面板记录无效。');ids.add(p.id);return {id:p.id,kind:p.kind as EditorKind,title:p.title,resourceId:p.resourceId as string|null,hidden:p.hidden};});
 if(panels.filter(p=>p.kind==='markdown').length>MAX_MARKDOWN||panels.filter(p=>p.kind==='workbench').length!==1||!object(value.dock)||!object(value.dock.layout))throw Error('工作区面板数量或布局无效。');
 const used=new Set<string>(),nodeIds=new Set<string>();let count=0;
 const node=(v:unknown,depth:number):Record<string,unknown>=>{
  if(!object(v)||depth>8||++count>64||!identity(v.id)||nodeIds.has(v.id))throw Error('工作区布局过深、重复或超过预算。');nodeIds.add(v.id);
  if(v.type==='tab'){
   const p=panels.find(p=>p.id===v.id);if(!p||p.hidden||used.has(p.id))throw Error('布局含未知或重复面板。');used.add(p.id);
   return {type:'tab',id:p.id,name:p.title,component:'workspace',enableClose:p.kind!=='workbench'};
  }
  if(!['row','tabset'].includes(String(v.type))||!Array.isArray(v.children)||v.children.length>MAX_PANELS)throw Error('工作区布局节点无效。');
  if(v.weight!==undefined&&(typeof v.weight!=='number'||!Number.isFinite(v.weight)||v.weight<=0||v.weight>1e6))throw Error('工作区比例无效。');
  const children=v.children.map(c=>node(c,depth+1));
  if(v.type==='tabset'&&children.some(c=>c.type!=='tab')||v.type==='row'&&children.some(c=>c.type==='tab'))throw Error('工作区节点层级无效。');
  const selected=typeof v.selected==='number'&&Number.isInteger(v.selected)&&v.selected>=0&&v.selected<children.length?v.selected:0;
  return {type:v.type,id:v.id,...(v.weight!==undefined?{weight:v.weight}:{}),children,...(v.type==='tabset'?{selected,...(v.active===true?{active:true}:{}),...(v.maximized===true?{maximized:true}:{})}:{})};
 };
 const layout=node(value.dock.layout,0);if(layout.type!=='row')throw Error('工作区根节点无效。');
 if(panels.some(p=>!p.hidden&&!used.has(p.id)))throw Error('可见面板缺少布局。');
 const rawGlobal=object(value.dock.global)?value.dock.global:{};
 return {version:1,workspaceId:value.workspaceId,explorer:value.explorer,panels,dock:{global:{...dockDefaults,rootOrientationVertical:rawGlobal.rootOrientationVertical===true},borders:[],layout:layout as unknown as IJsonModel['layout']}};
}
/** Restore positions without inventing an opened document or granting files. */
export function projectDock(saved:WorkspaceLayout,visible:PanelRecord[],mapping:Map<string,string>=new Map()):IJsonModel{
 const accepted=new Set(visible.filter(p=>!p.hidden).map(p=>p.id));const found=new Set<string>();
 const prune=(v:Record<string,unknown>):Record<string,unknown>|null=>{
  if(v.type==='tab'){const id=mapping.get(String(v.id))??String(v.id),p=visible.find(p=>p.id===id);if(!p||!accepted.has(id))return null;found.add(id);return {type:'tab',id,name:p.title,component:'workspace',enableClose:p.kind!=='workbench'};}
  const children=(v.children as Record<string,unknown>[]??[]).map(prune).filter(v=>v!==null);return children.length?{...v,selected:0,children}:null;
 };
 const layout=prune(saved.dock.layout as unknown as Record<string,unknown>)??{type:'row',id:'root',children:[]};const children=layout.children as Record<string,unknown>[];
 const missing=visible.filter(p=>!p.hidden&&!found.has(p.id));const nodeIds=new Set<string>();const collect=(n:Record<string,unknown>)=>{nodeIds.add(String(n.id));for(const child of (n.children??[]) as Record<string,unknown>[])collect(child);};collect(layout);let extra='added';while(nodeIds.has(extra))extra+='x';if(missing.length)children.push({type:'tabset',id:extra,children:missing.map(p=>({type:'tab',id:p.id,name:p.title,component:'workspace',enableClose:p.kind!=='workbench'}))});
 return {...saved.dock,layout:layout as unknown as IJsonModel['layout']};
}
