import {asSource,createBlankDocument,serializeDocument} from './model';
import type {MarkdownDocument} from './model';
export interface TemplateSeed {name:string;markdown:string;assets:Record<string,Blob>}
export interface TemplateInfo {id:string;name:string}
const sections=(title:string,items:string[])=>`# ${title}\n\n${items.map(item=>`## ${item}\n\n`).join('')}`;
export const builtInTemplates:TemplateSeed[]=[
  {name:'空白文档',markdown:'',assets:{}},
  {name:'论文精读笔记',markdown:sections('论文精读笔记',['论文信息','研究问题','核心创新点','模型与假设','主要方法','关键公式','实验与结果','个人思考']),assets:{}},
  {name:'数学定理推导',markdown:sections('数学定理推导',['问题定义','假设','引理','定理','证明','反例','推论','待解决问题']),assets:{}},
  {name:'科研实验记录',markdown:sections('科研实验记录',['实验信息','研究目标','材料与设备','实验条件','实验步骤','数据与结果','分析与讨论','结论与后续计划']),assets:{}},
];
export function templateDocument(seed:TemplateSeed):MarkdownDocument {
  const document=createBlankDocument();document.title=seed.name;
  document.blocks=[{...document.blocks[0],type:'text',markdown:seed.markdown}];serializeDocument(document);return document;
}
export const templateMarkdown=(document:MarkdownDocument)=>serializeDocument(asSource(document));
