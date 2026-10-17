import {invoke,isTauri} from '@tauri-apps/api/core';
import {decodeDocument,packDocument} from './api';
import type {TemplateInfo,TemplateSeed} from './templates';
export async function listTemplates():Promise<TemplateInfo[]>{
  if(!isTauri())return [];
  const result:unknown=await invoke('list_markdown_templates');
  if(!Array.isArray(result)||result.length>100||result.some(t=>!t||typeof t.id!=='string'||!/^[a-f0-9]{32}$/.test(t.id)||typeof t.name!=='string'))throw new Error('模板列表响应无效。');
  return result;
}
export async function loadTemplate(info:TemplateInfo):Promise<TemplateSeed>{
  const disk=decodeDocument(await invoke('load_markdown_template',{id:info.id}));if(!disk)throw new Error('模板内容为空。');
  return {name:info.name,markdown:disk.markdown,assets:disk.assets};
}
export async function saveTemplate(seed:TemplateSeed){
  if(!isTauri())throw new Error('自定义模板持久化需要桌面应用。');
  const body=await packDocument(seed.markdown,seed.assets,null,false,seed.name);await invoke('create_markdown_template',body.buffer);
}
export const renameTemplate=(id:string,name:string)=>invoke('rename_markdown_template',{id,name});
export const deleteTemplate=(id:string)=>invoke('delete_markdown_template',{id});
