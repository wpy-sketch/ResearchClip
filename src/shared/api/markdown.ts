import {invoke,isTauri} from '@tauri-apps/api/core';
export const MAX_EXPORT_CARDS=100;
export interface ExportIssue {id:string;message:string}
export interface MarkdownExportReport {directory:string;exportedIds:string[];failures:ExportIssue[];warnings:ExportIssue[]}
const validId=(value:unknown):value is string=>typeof value==='string'&&/^[a-f0-9]{32}$/.test(value);
export function validateExportIds(ids:string[]){if(!ids.length||ids.length>MAX_EXPORT_CARDS||ids.some(id=>!validId(id))||new Set(ids).size!==ids.length)throw new Error('请选择1至100张不同的已存库卡片。');}
function parseIssues(value:unknown):ExportIssue[]{
  if(!Array.isArray(value)||value.length>MAX_EXPORT_CARDS*2)throw new Error('导出问题列表无效。');
  return value.map(issue=>{if(!issue||typeof issue!=='object'||!validId(issue.id)||typeof issue.message!=='string'||!issue.message||issue.message.length>400)throw new Error('导出问题响应无效。');return {id:issue.id,message:issue.message};});
}
export function parseExportReport(value:unknown,ids:string[]):MarkdownExportReport|null {
  if(value===null)return null;
  if(!value||typeof value!=='object'||!('directory' in value)||typeof value.directory!=='string'||!value.directory||value.directory.includes('\0')||!('exportedIds' in value)||!Array.isArray(value.exportedIds)||!value.exportedIds.length||value.exportedIds.some(id=>!validId(id))||!('failures' in value)||!('warnings' in value))throw new Error('Markdown导出响应无效。');
  const exportedIds=value.exportedIds as string[],failures=parseIssues(value.failures),warnings=parseIssues(value.warnings),completed=[...exportedIds,...failures.map(f=>f.id)];
  if(completed.length!==ids.length||new Set(completed).size!==completed.length||completed.some(id=>!ids.includes(id))||warnings.some(w=>!exportedIds.includes(w.id)))throw new Error('Markdown导出结果与所选卡片不一致。');
  return {directory:value.directory,exportedIds,failures,warnings};
}
export async function exportKnowledgeMarkdown(ids:string[]){validateExportIds(ids);if(!isTauri())throw new Error('Markdown导出需要桌面应用和系统文件夹对话框。');return parseExportReport(await invoke('export_knowledge_markdown',{ids}),ids);}
