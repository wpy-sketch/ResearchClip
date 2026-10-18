import {fromMarkdown} from 'mdast-util-from-markdown';
import {toString} from 'mdast-util-to-string';
import {math} from 'micromark-extension-math';
import {mathFromMarkdown} from 'mdast-util-math';
export interface Heading {depth:number;text:string;offset:number;line:number}
export function documentOutline(markdown:string):Heading[]{
  const headings:Heading[]=[];
  const walk=(node:ReturnType<typeof fromMarkdown>|ReturnType<typeof fromMarkdown>['children'][number])=>{
    if(node.type==='heading'&&node.position)headings.push({depth:node.depth,text:toString(node,{includeHtml:false})||'（空标题）',offset:node.position.start.offset??0,line:node.position.start.line});
    if('children'in node)for(const child of node.children)walk(child as ReturnType<typeof fromMarkdown>['children'][number]);
  };
  walk(fromMarkdown(markdown,{extensions:[math()],mdastExtensions:[mathFromMarkdown()]}));return headings;
}
// A same-width mirror accounts for soft wrapping, tabs and CJK text in the textarea.
export function revealSourceHeading(node:HTMLTextAreaElement,offset:number){
  const mirror=document.createElement('div'),style=getComputedStyle(node);
  for(const key of ['font','letterSpacing','lineHeight','padding','boxSizing','tabSize','wordBreak','overflowWrap'] as const)mirror.style[key]=style[key];
  Object.assign(mirror.style,{position:'fixed',left:'-100000px',top:'0',width:`${node.clientWidth}px`,whiteSpace:'pre-wrap',visibility:'hidden'});
  mirror.textContent=node.value.slice(0,offset);const marker=document.createElement('span');marker.textContent='\u200b';mirror.append(marker);document.body.append(mirror);
  try{const top=marker.getBoundingClientRect().top-mirror.getBoundingClientRect().top;node.focus();node.setSelectionRange(offset,offset);node.scrollTop=Math.max(0,top-node.clientHeight/3);}finally{mirror.remove();}
}
