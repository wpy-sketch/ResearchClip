import {sourceText,type MarkdownDocument} from './model';
import {normalizedText,rawOffset,displayOffset} from './projection';
export function textTarget(document:MarkdownDocument,node:HTMLTextAreaElement){
 const key=node.dataset.markdownTarget;
 if(key==='source'&&document.mode==='source'){
  const whole=sourceText(document),from=node.hasAttribute('data-source-from')?Number(node.dataset.sourceFrom):0,to=node.hasAttribute('data-source-to')?Number(node.dataset.sourceTo):whole.length;
  if(!Number.isSafeInteger(from)||!Number.isSafeInteger(to)||from<0||to<from||to>whole.length)return null;
  const raw=whole.slice(from,to);if(normalizedText(raw)!==node.value)return null;
  return {key,raw,whole,from,to,start:rawOffset(raw,node.selectionStart),end:rawOffset(raw,node.selectionEnd)};
 }
 const block=document.blocks.find(b=>b.id===key);if(!block)return null;const raw=block.type==='image'?block.caption:block.markdown;if(normalizedText(raw)!==node.value)return null;
 return {key:key!,raw,whole:raw,from:0,to:raw.length,start:rawOffset(raw,node.selectionStart),end:rawOffset(raw,node.selectionEnd)};
}
export function restoreTextTarget(node:HTMLTextAreaElement,document:MarkdownDocument,range:{start:number;end:number}){
 const target=textTarget(document,node);if(!target)return null;return {start:displayOffset(target.raw,Math.max(0,range.start-target.from)),end:displayOffset(target.raw,Math.max(0,range.end-target.from))};
}
