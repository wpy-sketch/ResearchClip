import {escapeInline,sourceText,type MarkdownDocument} from './model';
import {analyzePaper} from './paper-model';
export function paperDocumentContext(document:MarkdownDocument){
 const offsets=new Map<string,number>();
 if(document.mode==='source'){offsets.set('source',0);return {paper:analyzePaper(sourceText(document)),offsets};}
 let raw=`# ${escapeInline(document.title)}\n\n`;
 for(const [index,block]of document.blocks.entries()){
  if(index)raw+='\n\n';
  if(block.type==='image')raw+=`![${escapeInline(block.alt)}](assets/${block.assetName})\n\n`;
  offsets.set(block.id,raw.length);raw+=block.type==='image'?block.caption:block.markdown;
 }
 return {paper:analyzePaper(raw),offsets};
}
