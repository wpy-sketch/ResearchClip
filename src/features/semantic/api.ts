import {invoke,isTauri} from '@tauri-apps/api/core';
export const MODEL_ID='clip-b32-multilingual-int8-v1';
export interface SemanticResult {items:{id:string;title:string;score:number}[];stale:number;searched:number}
export interface SemanticJob {phase:''|'running'|'cancelling'|'cancelled'|'completed'|'failed';kind:string;generation:number;total:number;scanned:number;computed:number;skipped:number;error:string|null;results:SemanticResult|null;query:string|null;scope:{projectId:string|null;unclassified:boolean;favorites:boolean}|null}
export interface SemanticStatus {modelId:string;modelBytes:number;modelsPresent:boolean;configured:boolean;stats:{total:number;ready:number};job:SemanticJob}
// Polling and action responses can finish in either order. A job never returns
// to an earlier generation, phase or scan position within the same process.
export function mergeSemanticStatus(current:SemanticStatus|null,next:SemanticStatus):SemanticStatus{
  if(!current)return next;
  const a=current.job,b=next.job;
  if(b.generation<a.generation)return current;
  if(b.generation>a.generation)return next;
  const terminal=(phase:SemanticJob['phase'])=>['completed','cancelled','failed'].includes(phase);
  if((terminal(a.phase)&&!terminal(b.phase))||(a.phase==='cancelling'&&b.phase==='running')||b.scanned<a.scanned||b.computed<a.computed||b.skipped<a.skipped)return current;
  return next;
}
const count=(v:unknown)=>Number.isSafeInteger(v)&&Number(v)>=0;
export function validateSemanticQuery(query:string){if(!query.trim()||[...query].length>500||[...query].some(c=>c.charCodeAt(0)<32||c.charCodeAt(0)===127))throw Error('请输入1至500个可见字符。');}
export function parseSemanticStatus(value:unknown):SemanticStatus{
  const s=value as SemanticStatus,j=s?.job;
  if(!s||s.modelId!==MODEL_ID||s.modelBytes!==228030489||typeof s.modelsPresent!=='boolean'||typeof s.configured!=='boolean'||!s.stats||!count(s.stats.total)||!count(s.stats.ready)||!j||!['','running','cancelling','cancelled','completed','failed'].includes(j.phase)||typeof j.kind!=='string'||![j.generation,j.total,j.scanned,j.computed,j.skipped].every(count)||(j.error!==null&&typeof j.error!=='string'))throw Error('语义任务状态无效。');
  if(j.query!==null&&typeof j.query!=='string')throw Error('语义查询状态无效。');
  if(j.scope!==null&&(!j.scope||typeof j.scope.unclassified!=='boolean'||typeof j.scope.favorites!=='boolean'||(j.scope.projectId!==null&&!/^[a-f0-9]{32}$/.test(j.scope.projectId))))throw Error('语义范围无效。');
  const r=j.results;if(r!==null&&(!r||!Array.isArray(r.items)||r.items.length>60||!count(r.stale)||!count(r.searched)||r.items.some(i=>!i||!/^[a-f0-9]{32}$/.test(i.id)||typeof i.title!=='string'||!Number.isFinite(i.score)||i.score < -1||i.score>1)))throw Error('语义结果无效。');
  return s;
}
export async function semanticCall(name:'semantic_status'|'semantic_start'|'semantic_cancel'|'semantic_clear'|'semantic_import',args?:Record<string,unknown>){if(!isTauri())throw Error('语义检索需要Windows桌面应用与本地模型。');return invoke(name,args);}
export const semanticStatus=async()=>parseSemanticStatus(await semanticCall('semantic_status'));
