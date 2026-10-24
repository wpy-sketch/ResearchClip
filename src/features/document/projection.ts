import {fromMarkdown} from 'mdast-util-from-markdown';
import {math} from 'micromark-extension-math';
import {mathFromMarkdown} from 'mdast-util-math';
export interface SourceBlock {from:number;to:number;kind:string;image:boolean;fixed:boolean}
export interface SourceProjection {blocks:SourceBlock[];reason:string|null;references:string}
export const normalizedText=(raw:string)=>raw.replace(/\r\n?/g,'\n');
export function rawOffset(raw:string,offset:number){let at=0,display=0;while(at<raw.length&&display<offset){at+=raw[at]==='\r'&&raw[at+1]==='\n'?2:1;display++;}return at;}
export const displayOffset=(raw:string,offset:number)=>normalizedText(raw.slice(0,Math.max(0,offset))).length;
export function applyDomText(raw:string,next:string,start:number,end:number){
 const before=normalizedText(raw);let left=0,right=0;
 while(left<before.length&&left<next.length&&before[left]===next[left])left++;
 while(right<before.length-left&&right<next.length-left&&before[before.length-1-right]===next[next.length-1-right])right++;
 const eol=raw.match(/\r\n|\n|\r/)?.[0]??'\n',inserted=next.slice(left,next.length-right).replaceAll('\n',eol),text=raw.slice(0,rawOffset(raw,left))+inserted+raw.slice(rawOffset(raw,before.length-right));
 return {text,start:rawOffset(text,start),end:rawOffset(text,end)};
}
export function projectMarkdown(raw:string):SourceProjection{
 const fallback=(reason:string)=>({blocks:[{from:0,to:raw.length,kind:'protected',image:false,fixed:true}],reason,references:''});
 if(raw.length>512*1024)return fallback('大稿使用整篇原文编辑，保留预览与保存；块拖动上限为512 Ki字符。');
 if(/^[ \t]*(?::{3,}|!!!\s)/m.test(raw))return fallback('检测到未注册扩展容器，保留完整原文；为避免拆散扩展，暂停块拖动。');
 let body=0;const blocks:SourceBlock[]=[];
 const front=/^\uFEFF?(---|\+\+\+)[ \t]*\r?\n/.exec(raw);
 if(front){const close=new RegExp('^'+front[1].replaceAll('+','\\+')+'[ \\t]*\\r?$','gm');close.lastIndex=front[0].length;const end=close.exec(raw);if(!end)return fallback('未闭合的前置元数据保持原文，暂停块拖动。');body=end.index+end[0].length;blocks.push({from:0,to:body,kind:'metadata',image:false,fixed:true});}
 const nodes=fromMarkdown(raw.slice(body),{extensions:[math()],mdastExtensions:[mathFromMarkdown()]}).children;
 if(nodes.length>300)return fallback('超过300个可视块，保留整篇原文编辑，暂停块拖动。');
 const refs:string[]=[];
 const tableNode=(node:typeof nodes[number])=>{if(node.type!=='paragraph')return false;const rows=raw.slice(body+(node.position?.start.offset??0),body+(node.position?.end.offset??0)).split(/\r\n|\n|\r/);if(rows.length<2||!rows[0].includes('|'))return false;const cells=rows[1].trim().replace(/^\|/,'').replace(/\|$/,'').split('|').map(c=>c.trim());return cells.length>0&&cells.every(c=>/^:?-{3,}:?$/.test(c));};
 for(let index=0;index<nodes.length;index++){
  const node=nodes[index],from=body+(node.position?.start.offset??0);let to=body+(node.position?.end.offset??0);
  const image=node.type==='paragraph'&&node.children.some(c=>c.type==='image'||c.type==='imageReference')&&node.children.every(c=>c.type==='image'||c.type==='imageReference'||c.type==='text'&&!c.value.trim());
  // Ordinary Markdown has no caption metadata: consecutive prose immediately below an
  // image belongs to its visible group. Lists/headings/code/math/another image end it.
  if(image){while(index+1<nodes.length){const next=nodes[index+1];if(next.type!=='paragraph'||tableNode(next)||next.children.some(c=>c.type==='image'||c.type==='imageReference'))break;to=body+(next.position?.end.offset??0);index++;}}
  if(node.type==='definition')refs.push(raw.slice(from,to));
  blocks.push({from,to,kind:image?'image':tableNode(node)?'table':node.type,image,fixed:false});
 }
 if(!blocks.length)blocks.push({from:0,to:raw.length,kind:'paragraph',image:false,fixed:false});
 return {blocks,reason:null,references:refs.join('\n\n')};
}
export function projectionRoundTrip(raw:string,projection=projectMarkdown(raw)){
 let cursor=0,result='';for(const block of projection.blocks){result+=raw.slice(cursor,block.from)+raw.slice(block.from,block.to);cursor=block.to;}return result+raw.slice(cursor);
}
export function moveSourceBlock(raw:string,expected:string,from:number,before:number,caret:{start:number;end:number}){
 if(raw!==expected)throw new Error('文档已变化，请重新拖动；当前原文未改动。');
 const p=projectMarkdown(raw),blocks=p.blocks;
 if(p.reason||!Number.isInteger(from)||!Number.isInteger(before)||from<0||from>=blocks.length||before<0||before>blocks.length||blocks[from].fixed||before===0&&blocks[0].fixed)throw new Error('此内容块不能移动，原文保留。');
 if(from===before||from+1===before)return {text:raw,...caret};
 const order=blocks.map((_,i)=>i);order.splice(from,1);order.splice(before>from?before-1:before,0,from);
 const gaps=blocks.slice(0,-1).map((b,i)=>raw.slice(b.to,blocks[i+1].from)),eol=raw.match(/\r\n|\n|\r/)?.[0]??'\n';let text=raw.slice(0,blocks[0].from);const offsets=new Map<number,number>();
 order.forEach((id,index)=>{offsets.set(id,text.length);text+=raw.slice(blocks[id].from,blocks[id].to);if(index<order.length-1){const gap=gaps[index];text+=(gap.match(/\r\n|\n|\r/g)?.length??0)>=2?gap:eol+eol;}});text+=raw.slice(blocks.at(-1)!.to);
 const map=(at:number)=>{const id=blocks.findIndex(b=>at>=b.from&&at<=b.to);return id<0?Math.min(at,text.length):offsets.get(id)!+at-blocks[id].from;};
 const start=map(caret.start),end=map(caret.end);return {text,start:Math.min(start,end),end:Math.max(start,end)};
}
export function scrollRatio(top:number,height:number,viewport:number){return Math.max(0,Math.min(1,top/Math.max(1,height-viewport)));}
export function slashTrigger(raw:string,start:number,end:number){
 if(start!==end)return null;const prefix=raw.slice(0,start),match=/(?:^|\n)[ \t]{0,3}\/([a-z]{0,16})$/.exec(prefix);if(!match)return null;
 const from=start-match[1].length-1,tree=fromMarkdown(raw,{extensions:[math()],mdastExtensions:[mathFromMarkdown()]});let blocked=false;
 const walk=(node:typeof tree|typeof tree.children[number])=>{if(['code','inlineCode','math','inlineMath','html'].includes(node.type)&&from>=(node.position?.start.offset??0)&&from<(node.position?.end.offset??0))blocked=true;if('children'in node)node.children.forEach(n=>walk(n as typeof tree.children[number]));};walk(tree);
 return blocked?null:{from,to:start,query:match[1]};
}
