import {documentOutline,type Heading} from '../document/outline';
export interface GuideSection {heading:Heading;chapter:number;body:string;children:GuideSection[]}
export function guideSections(markdown:string):GuideSection[]{
 const headings=documentOutline(markdown).filter(h=>h.depth>=2&&h.depth<=4),roots:GuideSection[]=[],stack:GuideSection[]=[];
 headings.forEach((heading,index)=>{
  while(stack.length&&stack.at(-1)!.heading.depth>=heading.depth)stack.pop();
  const chapter=heading.depth===2?Number(heading.text.match(/^(\d+)\./)?.[1]):stack[0]?.chapter;
  if(!chapter||!Number.isSafeInteger(chapter))return;
  const node:GuideSection={heading,chapter,body:markdown.slice(heading.offset,headings[index+1]?.offset??markdown.length),children:[]};
  if(heading.depth===2)roots.push(node);else if(stack.length)stack.at(-1)!.children.push(node);else return;
  stack.push(node);
 });return roots;
}
const normalize=(s:string)=>s.normalize('NFKC').toLocaleLowerCase().trim();
/** Keep example sections and their ancestors; examples live in the manual itself. */
export function guideExamples(sections:GuideSection[]):GuideSection[]{
 return sections.flatMap(section=>{
  if(section.heading.depth===3&&section.heading.text.includes('功能示例'))return [section];
  const children=guideExamples(section.children);
  return children.length?[{...section,body:'',children}]:[];
 });
}
export function searchGuide(sections:GuideSection[],query:string):GuideSection[]{
 const q=normalize(query);if(!q)return sections;
 return sections.flatMap(section=>{
  if(normalize(section.heading.text).includes(q))return [section];
  const children=searchGuide(section.children,query);
  return children.length||normalize(section.body).includes(q)?[{...section,children}]:[];
 });
}
export function guideOffsets(sections:GuideSection[],depth:number):number[]{return sections.flatMap(s=>s.heading.depth<=depth?[s.heading.offset,...guideOffsets(s.children,depth)]:[]);}
export function visibleGuideLocation(sections:GuideSection[],offset:number|null,depth:number,collapsed:Set<number>):number|null{
 if(offset===null)return null;
 const path=(nodes:GuideSection[]):GuideSection[]|null=>{for(const node of nodes){if(node.heading.offset===offset)return [node];const child=path(node.children);if(child)return [node,...child];}return null;};
 const chain=path(sections);if(!chain)return null;let visible:number|null=null;
 for(const node of chain){if(node.heading.depth>depth)break;visible=node.heading.offset;if(collapsed.has(visible))break;}return visible;
}

export const guideTopicGroups=[{name:'开始与采集',sections:[1,2,3]},{name:'资料与项目',sections:[4,5,6]},{name:'图片工具',sections:[7,8,9]},{name:'Markdown与文献基础',sections:[10,11,12,13,14,15]},{name:'命令、设置与安全',sections:[16,17,18]},{name:'科研工作台',sections:[19,20]},{name:'Markdown进阶与引用',sections:[21,22,23,24,25,26]},{name:'PDF与Zotero',sections:[27,28,29,30,31]}];
