export type ValueType='none'|'image'|'draft'|'confirmed'|'result';
export interface WorkflowConfig {id:string;title:string;actions:string[]}
export const actionSpecs=[
 {actionId:'capture',title:'区域截图',inputType:'none',outputType:'image',errorPolicy:'retry'},
 {actionId:'select',title:'选择素材图片',inputType:'none',outputType:'image',errorPolicy:'retry'},
 {actionId:'card',title:'创建素材卡片',inputType:'image',outputType:'result',errorPolicy:'inspect'},
 {actionId:'ocr',title:'识别图片文字',inputType:'image',outputType:'draft',errorPolicy:'retry'},
 {actionId:'formula',title:'识别数学公式',inputType:'image',outputType:'draft',errorPolicy:'retry'},
 {actionId:'confirm',title:'校对并确认结果',inputType:'draft',outputType:'confirmed',errorPolicy:'retry'},
 {actionId:'insert',title:'插入当前 Markdown',inputType:'confirmed',outputType:'result',errorPolicy:'retry'},
] as const;
export type ActionId=typeof actionSpecs[number]['actionId'];
export const presets:WorkflowConfig[]=[
 {id:'preset-card',title:'截图 → 创建素材卡片',actions:['capture','card']},
 {id:'preset-ocr',title:'截图 → OCR → 校对 → 插入 Markdown',actions:['capture','ocr','confirm','insert']},
 {id:'preset-formula',title:'选择图片 → 识别公式 → 用户确认 → 插入 Markdown',actions:['select','formula','confirm','insert']},
];
export function validateWorkflow(value:unknown):WorkflowConfig{
 const v=value as WorkflowConfig;
 if(!v||typeof v.id!=='string'||!/^(?:preset-[a-z]+|custom-[a-f0-9]{32})$/.test(v.id)||typeof v.title!=='string'||!v.title.trim()||v.title.length>80||Array.from(v.title).some(c=>c.charCodeAt(0)<32)||!Array.isArray(v.actions)||v.actions.length<2||v.actions.length>8||new Set(v.actions).size!==v.actions.length)throw Error('工作流名称或步骤无效（2–8步，不得重复动作）。');
 let type:ValueType='none';
 for(const id of v.actions){const action=actionSpecs.find(a=>a.actionId===id);if(!action||action.inputType!==type)throw Error('工作流包含未知动作或输入输出不兼容。');type=action.outputType;}
 if((type as ValueType)!=='result')throw Error('工作流必须以创建卡片或插入文档结束。');
 return {id:v.id,title:v.title,actions:[...v.actions]};
}
export function parseWorkflows(value:unknown):WorkflowConfig[]{if(!Array.isArray(value)||value.length>16)throw Error('自定义工作流最多16项。');const result=value.map(validateWorkflow);if(result.some(v=>!v.id.startsWith('custom-'))||new Set(result.map(v=>v.id)).size!==result.length)throw Error('自定义工作流ID重复或占用预设ID。');return result;}
export interface Searchable {id:string;title:string;keywords:string}
export function searchCommands<T extends Searchable>(items:T[],query:string):T[]{const terms=query.normalize('NFKC').toLocaleLowerCase().trim().split(/\s+/).filter(Boolean);return items.filter(item=>terms.every(term=>`${item.title} ${item.keywords}`.normalize('NFKC').toLocaleLowerCase().includes(term)));}
export function paletteShortcut(e:Pick<KeyboardEvent,'key'|'ctrlKey'|'shiftKey'|'altKey'|'metaKey'|'isComposing'|'repeat'>){return e.ctrlKey&&e.shiftKey&&!e.altKey&&!e.metaKey&&!e.isComposing&&!e.repeat&&e.key.toLowerCase()==='p';}
