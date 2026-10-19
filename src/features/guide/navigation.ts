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
