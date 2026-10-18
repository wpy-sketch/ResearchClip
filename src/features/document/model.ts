export const MAX_BLOCKS = 100;
export const MAX_IMAGES = 32;
export const MAX_TEXT_BYTES = 2 * 1024 * 1024;
export const MAX_ASSET_BYTES = 128 * 1024 * 1024;
export interface ImageBlock { id:string; type:'image'; assetName:string; sourceScreenshotId:string|null; alt:string; caption:string }
export interface TextBlock { id:string; type:'text'; markdown:string }
export type ContentBlock = ImageBlock | TextBlock;
export interface MarkdownDocument { id:string; title:string; blocks:ContentBlock[]; mode?:'source' }
export interface DocumentAsset { blob:Blob; url:string }
export const newId = () => crypto.randomUUID().replaceAll('-', '');
const id = (v:unknown):v is string => typeof v === 'string' && /^[a-f0-9]{32}$/.test(v);
export const assetNameValid = (v:unknown):v is string => typeof v==='string' && /^researchclip-[a-f0-9]{32}\.png$/.test(v);
const object = (v:unknown):v is Record<string,unknown> => Boolean(v)&&typeof v==='object'&&!Array.isArray(v);
export const escapeInline = (v:string) => v.replace(/[\\`*_[\]{}()#+.!<>~|&]/g, '\\$&').replace(/[\r\n]/g, ' ');
const header = (kind:string, v:unknown) => `<!-- ResearchClip:${kind} ${JSON.stringify(v).replaceAll('<','\\u003c').replaceAll('>','\\u003e')} -->\n`;
export const createDocument = ():MarkdownDocument => ({id:newId(),title:'未命名文档',blocks:[]});
export const createBlankDocument=():MarkdownDocument=>({id:newId(),title:'untitled.md',mode:'source',blocks:[{id:newId(),type:'text',markdown:''}]});
export const sourceText=(doc:MarkdownDocument)=>doc.blocks.map(b=>b.type==='text'?b.markdown:`![${escapeInline(b.alt)}](assets/${b.assetName})\n\n${b.caption}`).join('\n\n');
export const asSource=(doc:MarkdownDocument):MarkdownDocument=>doc.mode==='source'?doc:{...doc,mode:'source',blocks:[{id:newId(),type:'text',markdown:`# ${escapeInline(doc.title)}\n\n${sourceText(doc)}`}]};
export function validateDocument(doc:MarkdownDocument) {
  if (!id(doc.id)||typeof doc.title!=='string'||!doc.title.trim()||doc.title.length>200||/[\r\n\0]/.test(doc.title)||!Array.isArray(doc.blocks)||doc.blocks.length>MAX_BLOCKS) throw new Error('文档标题、ID或内容块数量无效（最多100块）。');
  const ids=new Set<string>();let images=0;
  for(const b of doc.blocks){
    if(!id(b.id)||ids.has(b.id))throw new Error('内容块ID无效或重复。');ids.add(b.id);
    if(b.type==='image'){
      images++;
      if(b.assetName!==`researchclip-${b.id}.png`||!(b.sourceScreenshotId===null||id(b.sourceScreenshotId))||typeof b.alt!=='string'||b.alt.length>200||typeof b.caption!=='string')throw new Error('图片内容块关联无效。');
    } else if(b.type!=='text'||typeof b.markdown!=='string')throw new Error('不支持的内容块类型。');
  }
  if(images>MAX_IMAGES)throw new Error('每份文档最多32张图片。');
}
export function serializeDocument(doc:MarkdownDocument):string {
  validateDocument(doc);
  if(doc.mode==='source'){
    if(doc.blocks.length!==1||doc.blocks[0].type!=='text')throw new Error('Markdown 源文结构无效。');
    const text=doc.blocks[0].markdown;
    if(new TextEncoder().encode(text).length>MAX_TEXT_BYTES)throw new Error('Markdown 文档超过2 MiB上限。');
    return text;
  }
  let result=header('document',{version:1,id:doc.id,title:doc.title})+`# ${escapeInline(doc.title)}\n\n`;
  for(const block of doc.blocks){
    const body=block.type==='image'?`![${escapeInline(block.alt)}](assets/${block.assetName})\n\n${block.caption}`:block.markdown;
    const meta=block.type==='image'?{id:block.id,type:block.type,assetName:block.assetName,sourceScreenshotId:block.sourceScreenshotId,alt:block.alt,length:body.length}:{id:block.id,type:block.type,length:body.length};
    result+=header('block',meta)+body+'\n<!-- ResearchClip:end -->\n\n';
  }
  if(new TextEncoder().encode(result).length>MAX_TEXT_BYTES)throw new Error('Markdown 文档超过2 MiB上限。');
  return result;
}
export function parseMarkdown(input:string, fallbackTitle='已打开文档'):MarkdownDocument {
  const text=input.replace(/^\uFEFF/,'').replaceAll('\r\n','\n');
  if(new TextEncoder().encode(text).length>MAX_TEXT_BYTES)throw new Error('Markdown 文档超过2 MiB上限。');
  if(!text.startsWith('<!-- ResearchClip:document ')){
    if(text.startsWith('<!-- ResearchClip:'))throw new Error('不支持的 ResearchClip 文档版本。');
    return {id:newId(),title:fallbackTitle.slice(0,200)||'已打开文档',mode:'source',blocks:[{id:newId(),type:'text',markdown:input}]};
  }
  const takeHeader=(start:number,kind:string):{meta:Record<string,unknown>;next:number}=>{
    const prefix=`<!-- ResearchClip:${kind} `,end=text.indexOf(' -->\n',start);
    if(!text.startsWith(prefix,start)||end<0)throw new Error('Markdown 内容块结构已损坏，原文件未改动。');
    const value:unknown=JSON.parse(text.slice(start+prefix.length,end));if(!object(value))throw new Error('Markdown 元数据无效。');
    return {meta:value,next:end+5};
  };
  const {meta,next}=takeHeader(0,'document');
  if(meta.version!==1||!id(meta.id)||typeof meta.title!=='string')throw new Error('不支持的 Markdown 文档版本或ID。');
  const doc:MarkdownDocument={id:meta.id,title:meta.title,blocks:[]},heading=`# ${escapeInline(doc.title)}\n\n`;
  if(!text.startsWith(heading,next))throw new Error('标题被外部修改，结构不一致；请保留原文件并在文本编辑器中检查。');
  let cursor=next+heading.length;
  while(cursor<text.length){
    const {meta:m,next:bodyStart}=takeHeader(cursor,'block');
    if(!id(m.id)||!Number.isSafeInteger(m.length)||(m.length as number)<0)throw new Error('内容块长度无效。');
    const end=bodyStart+(m.length as number),body=text.slice(bodyStart,end),suffix='\n<!-- ResearchClip:end -->\n\n';
    if(!text.startsWith(suffix,end))throw new Error('内容块被外部修改或不完整；未替换当前文档。');
    if(m.type==='image'&&typeof m.alt==='string'&&assetNameValid(m.assetName)&&(m.sourceScreenshotId===null||id(m.sourceScreenshotId))){
      const image=`![${escapeInline(m.alt)}](assets/${m.assetName})\n\n`;
      if(!body.startsWith(image))throw new Error('图片与注释的关联已损坏。');
      doc.blocks.push({id:m.id,type:'image',alt:m.alt,assetName:m.assetName,sourceScreenshotId:m.sourceScreenshotId,caption:body.slice(image.length)});
    }else if(m.type==='text')doc.blocks.push({id:m.id,type:'text',markdown:body});
    else throw new Error('不支持的内容块。');
    cursor=end+suffix.length;
  }
  validateDocument(doc);return doc;
}
export function moveBlock(doc:MarkdownDocument,id:string,delta:-1|1):MarkdownDocument {
  const blocks=[...doc.blocks],i=blocks.findIndex(b=>b.id===id),j=i+delta;
  if(i<0||j<0||j>=blocks.length)return doc;
  [blocks[i],blocks[j]]=[blocks[j],blocks[i]];return {...doc,blocks};
}
