import {fromMarkdown} from 'mdast-util-from-markdown';
import {toString} from 'mdast-util-to-string';
import {math} from 'micromark-extension-math';
import {mathFromMarkdown} from 'mdast-util-math';
export type LinkKind='document'|'screenshot';
export interface Target {id:string;kind:LinkKind;anchorId?:string}
export interface IndexedLink {targetId:string;linkType:LinkKind;start:number;end:number;label:string;anchorId?:string}
export function internalTarget(url:string):Target|null {const m=/^researchclip:\/\/(document|screenshot)\/([a-f0-9]{32})(?:\/anchor\/([a-f0-9]{32}))?$/.exec(url);return m&&(!m[3]||m[1]==='screenshot')?{kind:m[1] as LinkKind,id:m[2],...(m[3]?{anchorId:m[3]}:{})}:null;}
export const targetUrl=(t:Target)=>`researchclip://${t.kind}/${t.id}${t.anchorId!==undefined&&t.anchorId!==null?`/anchor/${t.anchorId}`:''}`;
export const targetKey=(t:Target)=>`${t.kind}:${t.id}`;
export function linkMarkdown(target:Target,title:string){if(!internalTarget(targetUrl(target)))throw Error('内部引用目标无效。');const label=Array.from(title.replace(/[\r\n\t]/g,' ')).slice(0,200).join('').replace(/[\\[\]]/g,'\\$&');return `[${label||'未命名'}](${targetUrl(target)})`;}
const parse=(text:string)=>fromMarkdown(text,{extensions:[math()],mdastExtensions:[mathFromMarkdown()]});
export function parseLinks(markdown:string):IndexedLink[]{
 const links:IndexedLink[]=[],stack:ReturnType<typeof parse>['children']= [...parse(markdown).children];
 while(stack.length){const node=stack.pop()!;
  if(node.type==='link'&&node.position){const t=internalTarget(node.url),start=node.position.start.offset,end=node.position.end.offset;
   if(t&&start!==undefined&&end!==undefined&&markdown.slice(start,end).endsWith(`](${targetUrl(t)})`))links.push({targetId:t.id,linkType:t.kind,...(t.anchorId?{anchorId:t.anchorId}:{}),start,end,label:Array.from(toString(node).replace(/\p{Cc}/gu,' ')).slice(0,200).join('')});
   if(links.length>5000)throw Error('每篇文档最多索引5000条内部引用，已有索引保留。');
  }
  if('children'in node)stack.push(...node.children as ReturnType<typeof parse>['children']);
 }
 return links.sort((a,b)=>a.start-b.start);
}
export interface WikiTrigger {start:number;end:number;query:string}
export function wikiTrigger(markdown:string,caret:number):WikiTrigger|null{
 if(caret<0||caret>markdown.length)return null;
 const prefix=markdown.slice(Math.max(0,caret-204),caret),at=prefix.lastIndexOf('[[');if(at<0)return null;
 const tail=prefix.slice(at+2),closed=tail.endsWith(']]'),query=closed?tail.slice(0,-2):tail;if(/[\r\n[\]]/.test(query)||query.length>200)return null;
 const start=caret-prefix.length+at;
 if(start>0&&markdown[start-1]==='\\')return null;
 const stack:ReturnType<typeof parse>['children']=[...parse(markdown).children];let textNode=false;
 while(stack.length){const n=stack.pop()!;if(n.type==='text'&&(n.position?.start.offset??Infinity)<=start&&(n.position?.end.offset??-1)>=caret)textNode=true;if('children'in n&&n.type!=='link')stack.push(...n.children as ReturnType<typeof parse>['children']);}
 if(!textNode)return null;
 return {start,end:!closed&&markdown.slice(caret,caret+2)===']]'?caret+2:caret,query};
}
export interface GraphNode extends Target {title:string;projectId:string|null;available:boolean}
export interface GraphEdge {source:string;target:string}
export interface GraphData {nodes:GraphNode[];edges:GraphEdge[];truncated:boolean}
export function graphLayout(nodes:GraphNode[],focus:string|null){
 const ordered=[...nodes].sort((a,b)=>targetKey(a).localeCompare(targetKey(b))),others=ordered.filter(n=>targetKey(n)!==focus);
 const points=new Map<string,{x:number;y:number}>();
 if(focus&&ordered.some(n=>targetKey(n)===focus))points.set(focus,{x:500,y:320});
 others.forEach((n,i)=>{const ring=Math.floor(i/36),angle=2*Math.PI*(i%36)/Math.min(36,others.length-ring*36),radius=others.length===1?(focus&&points.has(focus)?170:0):170+ring*80;points.set(targetKey(n),{x:500+Math.cos(angle)*radius,y:320+Math.sin(angle)*radius});});
 return points;
}
