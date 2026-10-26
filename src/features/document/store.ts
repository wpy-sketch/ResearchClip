import {loadMarkdownRevision} from './organization-api';
import {withLegacyImageTitle} from './content-model';
import {create} from 'zustand';
import {createContext,useContext} from 'react';
import {getScreenshot,getKnowledgeCard,screenshotImage} from '../../shared/api/library';
import {useEditorStore} from '../editor/store';
import {useCardStore} from '../notes/store';
import {errorMessage} from '../../shared/api/errors';
import {createDocument,newId,moveBlock,parseMarkdown,serializeDocument,MAX_BLOCKS,MAX_IMAGES,MAX_ASSET_BYTES,createBlankDocument,asSource,sourceText,escapeInline} from './model';
import type {MarkdownDocument,DocumentAsset,ImageBlock} from './model';
import {saveDocumentFile,openDocumentFile,releaseDocument,renameDocumentFile} from './api';
import {templateDocument,type TemplateSeed} from './templates';
import {normalizeMarkdownName} from './file-name';
export type DocumentAction='new'|'blank'|'open'|'close'|'template'|'project';
export interface DocumentCaret {start:number;end:number;blockId?:string}
interface TextEdit {document:MarkdownDocument;caret:DocumentCaret}
const bounded=(items:TextEdit[])=>{let bytes=0;return items.slice(-100).reverse().filter(item=>{bytes+=JSON.stringify(item.document).length*2;return bytes<=8*1024*1024;}).reverse();};
export interface DocumentState {
  projectOpenId:string|null;projectOpenOffset:number;projectLinkNavigation:boolean;openProject:(id:string,offset?:number)=>Promise<void>;
  insertText:(text:string)=>boolean;
  templateDialog:'choose'|'save'|null; setTemplateDialog:(dialog:'choose'|'save'|null)=>void;
  templateSeed:TemplateSeed|null; createFromTemplate:(seed:TemplateSeed)=>Promise<void>;
  jumpToSource:(offset:number)=>void;
  caret:DocumentCaret;past:TextEdit[];future:TextEdit[];focusRequest:number;
  setCaret:(start:number,end:number,blockId?:string)=>void;editSource:(text:string,start?:number,end?:number)=>void;toSource:()=>void;undoText:()=>void;redoText:()=>void;insertEditor:()=>Promise<boolean>;
  document:MarkdownDocument|null;assets:Record<string,DocumentAsset>;token:string|null;path:string|null;fileName:string|null;savedFingerprint:string|null;
  phase:'idle'|'adding'|'saving'|'opening'|'renaming';error:string|null;notice:string|null;pending:DocumentAction|null;
  request:(action:DocumentAction)=>Promise<void>;resolvePending:(choice:'save'|'discard'|'cancel')=>Promise<void>;
  editTitle:(title:string)=>void;editBlock:(id:string,text:string,start?:number,end?:number)=>void;addText:(afterId?:string)=>void;move:(id:string,delta:-1|1)=>void;remove:(id:string)=>void;
  editImage:(id:string,alt:string,title:string)=>boolean;duplicateImage:(id:string)=>boolean;
  addImages:(ids:string[],options?:{separateParagraphs?:boolean;pdfLocationId?:string})=>Promise<boolean>;insertImagesAt:(ids:string[],target:{documentId:string;blockId:string;from:number;to:number;expected:string})=>Promise<boolean>;reorderBlocks:(id:string,before:string|null,expected:string[])=>void;restoreHistory:(id:string,revisionId:string)=>Promise<boolean>;replaceDocument:(document:MarkdownDocument)=>boolean;save:(saveAs?:boolean,restoredFrom?:string)=>Promise<boolean>;rename:(name:string)=>Promise<boolean>;
}
export const documentDirty=(s:Pick<DocumentState,'document'|'savedFingerprint'>)=>Boolean(s.document&&(s.savedFingerprint===null||JSON.stringify(s.document)!==s.savedFingerprint));
const dispose=(assets:Record<string,DocumentAsset>)=>Object.values(assets).forEach(a=>URL.revokeObjectURL(a.url));
export function createDocumentStore(){
  return create<DocumentState>((set,get)=>{
    const release=async(token:string|null)=>{try{await releaseDocument(token);}catch{set({error:'旧文档文件授权未能释放；已有文件未改动，请保存当前文档后重启。'});}};
    const run=async(action:DocumentAction)=>{
      if(action==='open'||action==='project'){
        set({phase:'opening',error:null,notice:null,pending:null});let openedToken:string|null=null;
        try{
          const disk=await openDocumentFile(action==='project'?get().projectOpenId??undefined:undefined);if(!disk)return;
          openedToken=disk.token;const parsed=parseMarkdown(disk.markdown,disk.path.split(/[\\/]/).at(-1)),offset=action==='project'?Math.min(get().projectOpenOffset,disk.markdown.length):0,document=action==='project'&&get().projectLinkNavigation?{...parsed,mode:'source' as const,blocks:[{id:newId(),type:'text' as const,markdown:disk.markdown}]}:parsed;
          for(const b of document.blocks)if(b.type==='image'&&!disk.assets[b.assetName])throw new Error('文档引用的图片缺失，当前草稿已保留。');
          const assets:Record<string,DocumentAsset>={};
          try{for(const [name,blob] of Object.entries(disk.assets))assets[name]={blob,url:URL.createObjectURL(blob)};}catch(error){dispose(assets);throw error;}
          const old=get();dispose(old.assets);set({document,caret:{start:offset,end:offset},past:[],future:[],focusRequest:get().focusRequest+1,assets,token:disk.token,path:disk.path,fileName:disk.path.split(/[\\/]/).at(-1)??'untitled.md',savedFingerprint:JSON.stringify(document),pending:null,notice:'Markdown 已打开；图片与注释顺序已恢复。'});openedToken=null;await release(old.token);
        }catch(error){set({error:errorMessage(error,'打开失败，当前文档已保留。')});}
        finally{if(openedToken)await release(openedToken);set({phase:'idle'});}
      }else if(action==='template'){
        const seed=get().templateSeed;if(!seed)return;
        const assets:Record<string,DocumentAsset>={};
        try{const document=templateDocument(seed);for(const [name,blob]of Object.entries(seed.assets))assets[name]={blob,url:URL.createObjectURL(blob)};
          const old=get();dispose(old.assets);set({document,assets,token:null,path:null,fileName:normalizeMarkdownName(seed.name),savedFingerprint:null,pending:null,templateSeed:null,caret:{start:0,end:0},past:[],future:[],focusRequest:get().focusRequest+1,error:null,notice:'已从模板创建独立草稿；原文档和模板未改动。'});await release(old.token);
        }catch(error){dispose(assets);set({error:errorMessage(error,'模板加载失败，当前文档未改动。'),pending:null,templateSeed:null});}
      }else{
        const old=get();dispose(old.assets);set({document:action==='new'?createDocument():action==='blank'?createBlankDocument():null,caret:{start:0,end:0},past:[],future:[],focusRequest:get().focusRequest+1,assets:{},token:null,path:null,fileName:action==='new'?'未命名文档.md':action==='blank'?'untitled.md':null,savedFingerprint:null,pending:null,templateSeed:null,error:null,notice:null});await release(old.token);
      }
    };
    const change=(fn:(doc:MarkdownDocument)=>MarkdownDocument)=>{const s=get();if(s.document&&s.phase==='idle'){const document=fn(s.document);if(document!==s.document)set({document,notice:null,error:null,past:[],future:[]});}};
    const updateSource=(text:string,start:number,end:number)=>{
      const s=get();if(!s.document||s.phase!=='idle'||s.document.mode!=='source')return;
      const document={...s.document,blocks:[{id:s.document.blocks[0].id,type:'text' as const,markdown:text}]};
      try{serializeDocument(document);}catch(error){set({error:errorMessage(error,'文档过大。')});return;}
      if(text===sourceText(s.document)){set({caret:{start,end}});return;}
      set({document,caret:{start,end},past:bounded([...s.past,{document:s.document,caret:s.caret}]),future:[],notice:null,error:null});
    };
    const append=(document:MarkdownDocument,blocks:ImageBlock[],caret:DocumentCaret,separateParagraphs=false,pdfLocationId?:string)=>{
      if(document.mode!=='source'){
        const at=caret.blockId?document.blocks.findIndex(b=>b.id===caret.blockId):-1,existing=document.blocks[at];
        if(at<0)return {...document,blocks:[...document.blocks,...blocks]};
        if(existing.type==='image')return {...document,blocks:[...document.blocks.slice(0,at+1),...blocks,...document.blocks.slice(at+1)]};
        const start=Math.min(caret.start,existing.markdown.length),end=Math.min(caret.end,existing.markdown.length),tail=existing.markdown.slice(end);
        return {...document,blocks:[...document.blocks.slice(0,at),{...existing,markdown:existing.markdown.slice(0,start)},...blocks,...(tail?[{id:newId(),type:'text' as const,markdown:tail}]:[]),...document.blocks.slice(at+1)]};
      }
      const text=sourceText(document),start=Math.min(caret.start,text.length),end=Math.min(caret.end,text.length);
      const eol=text.match(/\r\n|\n|\r/)?.[0]??'\n',prefix=text.slice(0,start),gap=separateParagraphs&&prefix&&!/\n[ \t]*\n$/.test(prefix.replace(/\r\n|\r/g,'\n'))?(prefix.endsWith('\n')||prefix.endsWith('\r')?eol:eol+eol):'';
      if(pdfLocationId&&(!/^[a-f0-9]{32}$/.test(pdfLocationId)||blocks.length!==1))throw Error('摘录分组必须使用一个有效的PDF位置ID。');
      const inserted=gap+(blocks.map(b=>`${pdfLocationId?`<!-- rc-pdf-excerpt:v1 ${pdfLocationId} -->\n\n`:''}![${escapeInline(b.alt)}](assets/${b.assetName})\n\n${b.caption}${pdfLocationId?'\n\n<!-- /rc-pdf-excerpt -->':''}`).join('\n\n')+'\n\n').replaceAll('\n',separateParagraphs?eol:'\n');
      const next={...document,blocks:[{id:document.blocks[0].id,type:'text' as const,markdown:text.slice(0,start)+inserted+text.slice(end)}]};
      serializeDocument(next);
      return next;
    };
    const appendedHistory=(before:DocumentState,after:MarkdownDocument)=>{if(before.document?.mode!=='source'){const last=after.blocks.filter(b=>!before.document?.blocks.some(old=>old.id===b.id)&&b.type==='image').at(-1);return {past:bounded([...before.past,{document:before.document!,caret:before.caret}]),future:[],...(last?.type==='image'?{caret:{start:last.caption.length,end:last.caption.length,blockId:last.id},focusRequest:before.focusRequest+1}:{})};}const at=Math.min(before.caret.end,sourceText(before.document).length)+sourceText(after).length-sourceText(before.document).length;return {caret:{start:at,end:at},past:bounded([...before.past,{document:before.document,caret:before.caret}]),future:[],focusRequest:before.focusRequest+1};};
    return {
      projectOpenId:null,projectOpenOffset:0,projectLinkNavigation:false,openProject:async(id,offset)=>{if(get().phase!=='idle'||get().pending)return;if(offset!==undefined&&(!Number.isSafeInteger(offset)||offset<0))return;set({projectOpenId:id,projectOpenOffset:offset??0,projectLinkNavigation:offset!==undefined});await get().request('project');},
      templateDialog:null,setTemplateDialog:(templateDialog)=>set({templateDialog}),
      templateSeed:null,
      createFromTemplate:async(seed)=>{if(get().phase!=='idle'||get().pending)return;try{templateDocument(seed);normalizeMarkdownName(seed.name);set({templateSeed:seed});await get().request('template');}catch(error){set({error:errorMessage(error,'模板无效。')});}},
      jumpToSource:(offset)=>{const s=get();if(!s.document||s.phase!=='idle')return;const document=asSource(s.document),at=Math.max(0,Math.min(offset,sourceText(document).length));set({document,caret:{start:at,end:at},focusRequest:s.focusRequest+1,...(s.document.mode==='source'?{}:{past:[],future:[]})});},
      insertText:(text)=>{
        const s=get();if(!s.document||s.phase!=='idle'||!text.trim())return false;
        try{
          if(s.document.mode==='source'){
            const current=sourceText(s.document),start=Math.min(s.caret.start,current.length),end=Math.min(s.caret.end,current.length),next=current.slice(0,start)+text+current.slice(end);
            serializeDocument({...s.document,blocks:[{...s.document.blocks[0],type:'text',markdown:next}]});
            updateSource(next,start+text.length,start+text.length);set({focusRequest:s.focusRequest+1});
          }else{
            if(s.document.blocks.length>=MAX_BLOCKS)throw new Error('每份文档最多100个内容块。');
            const document={...s.document,blocks:[...s.document.blocks,{id:newId(),type:'text' as const,markdown:text}]};serializeDocument(document);set({document,notice:null,error:null,past:[],future:[]});
          }
          return true;
        }catch(error){set({error:errorMessage(error,'插入文字失败，原文档未改动。')});return false;}
      },
      caret:{start:0,end:0},past:[],future:[],focusRequest:0,
      setCaret:(start,end,blockId)=>{if(Number.isSafeInteger(start)&&Number.isSafeInteger(end)&&start>=0&&end>=start)set({caret:{start,end,...(blockId?{blockId}:{})}});},
      editSource:(text,start=text.length,end=start)=>updateSource(text,start,end),
      toSource:()=>{const s=get();if(s.document&&s.phase==='idle')set({document:asSource(s.document),caret:{start:0,end:0},past:[],future:[],focusRequest:s.focusRequest+1});},
      undoText:()=>{const s=get(),last=s.past.at(-1);if(s.phase==='idle'&&last&&s.document)set({...last,past:s.past.slice(0,-1),future:bounded([...s.future,{document:s.document,caret:s.caret}]),focusRequest:s.focusRequest+1});},
      redoText:()=>{const s=get(),last=s.future.at(-1);if(s.phase==='idle'&&last&&s.document)set({...last,future:s.future.slice(0,-1),past:bounded([...s.past,{document:s.document,caret:s.caret}]),focusRequest:s.focusRequest+1});},
      insertEditor:async()=>{
        const s=get(),editor=useEditorStore.getState(),session=editor.activeId?editor.sessions[editor.activeId]:null;
        if(!s.document||s.phase!=='idle'||!session||session.loadError||editor.gesture)return false;
        if(Object.keys(s.assets).length>=MAX_IMAGES){set({error:'文档最多32张图片（含撤销保留资源）。'});return false;}
        set({phase:'adding',error:null});let url:string|undefined;
        try{const {renderComposite}=await import('../editor/export');const blob=await renderComposite(session.image.blob,session.document);
          if(Object.values(s.assets).reduce((n,a)=>n+a.blob.size,blob.size)>MAX_ASSET_BYTES)throw new Error('文档图片合计超过128 MiB。');
          const id=newId(),assetName=`researchclip-${id}.png`,block:ImageBlock={id,type:'image',assetName,sourceScreenshotId:session.image.libraryId??null,alt:session.image.name.slice(0,200),caption:'注释：'};
          url=URL.createObjectURL(blob);const document=append(s.document,[block],s.caret);serializeDocument(document);
          set({document,...appendedHistory(s,document),assets:{...s.assets,[assetName]:{blob,url}},notice:'当前标注合成图已插入 Markdown，原图未改动。'});return true;
        }catch(error){if(url)URL.revokeObjectURL(url);set({error:errorMessage(error,'插入图片失败。')});return false;}finally{set({phase:'idle'});}
      },
      document:null,assets:{},token:null,path:null,fileName:null,savedFingerprint:null,phase:'idle',error:null,notice:null,pending:null,
      request:async(action)=>{if(get().phase!=='idle'){set({error:'文档正在处理，请等待完成。'});return;}if(documentDirty(get())){set({pending:action});return;}await run(action);},
      resolvePending:async(choice)=>{const action=get().pending;if(!action||get().phase!=='idle')return;if(choice==='cancel'){set({pending:null,templateSeed:null});return;}if(choice==='save'&&!await get().save())return;await run(action);},
      editTitle:(title)=>change(doc=>({...doc,title})),
      editBlock:(id,text,start=text.length,end=start)=>{const s=get(),block=s.document?.blocks.find(b=>b.id===id);if(!s.document||s.phase!=='idle'||!block)return;const value=text,document={...s.document,blocks:s.document.blocks.map(b=>b.id===id?b.type==='image'?{...b,caption:value}:{...b,markdown:value}:b)};try{serializeDocument(document);}catch(error){set({error:errorMessage(error,'文档过大，原内容保留。')});return;}if((block.type==='image'?block.caption:block.markdown)===value){set({caret:{start,end,blockId:id}});return;}set({document,caret:{start,end,blockId:id},past:bounded([...s.past,{document:s.document,caret:s.caret}]),future:[],notice:null,error:null});},
      addText:(afterId)=>change(doc=>{if(doc.blocks.length>=MAX_BLOCKS){set({error:'每份文档最多100个内容块。'});return doc;}const blocks=[...doc.blocks],index=afterId?blocks.findIndex(b=>b.id===afterId)+1:blocks.length;blocks.splice(index,0,{id:newId(),type:'text',markdown:''});return {...doc,blocks};}),
      move:(id,delta)=>change(doc=>moveBlock(doc,id,delta)),
      remove:(id)=>change(doc=>{const block=doc.blocks.find(b=>b.id===id);if(block?.type==='image'){const assets={...get().assets};if(assets[block.assetName])URL.revokeObjectURL(assets[block.assetName].url);delete assets[block.assetName];set({assets});}return {...doc,blocks:doc.blocks.filter(b=>b.id!==id)};}),
      editImage:(id,alt,title)=>{const s=get(),doc=s.document,block=doc?.blocks.find(b=>b.id===id);if(!doc||doc.mode==='source'||s.phase!=='idle'||block?.type!=='image')return false;try{if(alt.length>200||/[\r\n\0]/.test(alt))throw new Error('图片替代文本限200字且不能换行。');const caption=withLegacyImageTitle(block.caption,title),document={...doc,blocks:doc.blocks.map(b=>b.id===id?{...block,alt,caption}:b)};serializeDocument(document);if(alt===block.alt&&caption===block.caption)return true;set({document,past:bounded([...s.past,{document:doc,caret:s.caret}]),future:[],error:null,notice:'图片属性已修改，可撤销；图题保存在可读注释中。'});return true;}catch(e){set({error:errorMessage(e,'图片属性修改失败，原内容保留。')});return false;}},
      duplicateImage:(id)=>{const s=get(),doc=s.document,block=doc?.blocks.find(b=>b.id===id);if(!doc||doc.mode==='source'||s.phase!=='idle'||block?.type!=='image')return false;let url:string|undefined;try{const asset=s.assets[block.assetName];if(!asset)throw new Error('图片资源缺失，不能复制。');if(doc.blocks.length>=MAX_BLOCKS||Object.keys(s.assets).length>=MAX_IMAGES||Object.values(s.assets).reduce((n,a)=>n+a.blob.size,asset.blob.size)>MAX_ASSET_BYTES)throw new Error('复制超过文档资源预算，原内容保留。');const nextId=newId(),name=`researchclip-${nextId}.png`,copy={...block,id:nextId,assetName:name},blocks=[...doc.blocks];blocks.splice(blocks.findIndex(b=>b.id===id)+1,0,copy);const document={...doc,blocks};serializeDocument(document);url=URL.createObjectURL(asset.blob);set({document,assets:{...s.assets,[name]:{blob:asset.blob,url}},past:bounded([...s.past,{document:doc,caret:s.caret}]),future:[],error:null,notice:'已复制图片和整段注释，原图未改动，可撤销。'});return true;}catch(e){if(url)URL.revokeObjectURL(url);set({error:errorMessage(e,'复制图片失败，原内容保留。')});return false;}},
      addImages:async(ids,options)=>{
        const s=get();if(!s.document||s.phase!=='idle')return false;
        if(!ids.length||new Set(ids).size!==ids.length||ids.some(id=>!/^[a-f0-9]{32}$/.test(id))||s.document.blocks.length+ids.length>MAX_BLOCKS||Object.keys(s.assets).length+ids.length>MAX_IMAGES){set({error:'图片选择无效或超过32张图片/100块上限。'});return false;}
        set({phase:'adding',error:null});const assets:Record<string,DocumentAsset>={},blocks:ImageBlock[]=[];let size=Object.values(s.assets).reduce((n,a)=>n+a.blob.size,0);
        try{
          for(const [index,id]of ids.entries()){
            set({notice:`正在合成已保存标注 ${index+1}/${ids.length}…`});
            if(Object.values(useEditorStore.getState().sessions).some(session=>session.image.libraryId===id&&(JSON.stringify(session.document)!==session.savedDocument||useEditorStore.getState().gesture?.imageId===session.image.id)))throw new Error('选中图片仍有未保存标注，请先在编辑器保存到素材库。');
            const image=await getScreenshot(id),card=await getKnowledgeCard(id);
            if(image.revision!==card.screenshot.revision)throw new Error('素材刚被修改，请刷新后重新添加。');
            const {renderComposite}=await import('../editor/export');
            const source=await screenshotImage(id),blob=await renderComposite(source,image.annotationData);
            size+=blob.size;if(size>MAX_ASSET_BYTES)throw new Error('文档图片合计超过128 MiB。');
            const blockId=newId(),name=`researchclip-${blockId}.png`,caption=useCardStore.getState().sessions[id]?.draft?.content??card.note?.content??'';
            assets[name]={blob,url:URL.createObjectURL(blob)};blocks.push({id:blockId,type:'image',assetName:name,sourceScreenshotId:id,alt:image.title,caption:caption.trim()?caption.replaceAll('\r\n','\n'):'注释：'});
          }
          const document=append(s.document,blocks,s.caret,options?.separateParagraphs,options?.pdfLocationId);serializeDocument(document);
          set({document,...appendedHistory(s,document),assets:{...s.assets,...assets},notice:`已添加 ${blocks.length} 张含标注的图片；注释为添加时的笔记快照。`});return true;
        }catch(error){dispose(assets);set({error:errorMessage(error,'添加失败，已有内容未改动。'),notice:null});return false;}
        finally{set({phase:'idle'});}
      },
      reorderBlocks:(id,before,expected)=>{const s=get(),doc=s.document;if(!doc||doc.mode==='source'||s.phase!=='idle')return;const order=doc.blocks.map(b=>b.id);if(JSON.stringify(order)!==JSON.stringify(expected)||!order.includes(id)||(before!==null&&!order.includes(before))){set({error:'内容块已变化，请重新拖动；原文未改动。'});return;}if(id===before)return;const blocks=doc.blocks.filter(b=>b.id!==id),at=before===null?blocks.length:blocks.findIndex(b=>b.id===before);blocks.splice(at,0,doc.blocks.find(b=>b.id===id)!);if(blocks.every((b,i)=>b===doc.blocks[i]))return;set({document:{...doc,blocks},past:bounded([...s.past,{document:doc,caret:s.caret}]),future:[],focusRequest:s.focusRequest+1,error:null,notice:'已整体移动图文块，可撤销。'});},
      insertImagesAt:async(ids,target)=>{const before=get(),doc=before.document,block=doc?.blocks.find(b=>b.id===target.blockId),field=doc?.mode==='source'&&target.blockId==='source'?sourceText(doc):block?(block.type==='image'?block.caption:block.markdown):null;
        if(!doc||before.phase!=='idle'||doc.id!==target.documentId||field!==target.expected||!Number.isSafeInteger(target.from)||!Number.isSafeInteger(target.to)||target.from<0||target.to<target.from||target.to>field.length){set({error:'插图目标已变化，当前文档未改动。'});return false;}
        if(doc.mode==='source')get().setCaret(target.from,target.to);
        if(!await get().addImages(ids)){set({caret:before.caret});return false;}
        if(doc.mode==='source')return true;
        try{const after=get(),added=after.document!.blocks.filter(b=>!doc.blocks.some(old=>old.id===b.id)),value=field.slice(0,target.from)+field.slice(target.to),blocks=doc.blocks.map(b=>b.id===target.blockId?b.type==='image'?{...b,caption:value}:{...b,markdown:value}:b),at=blocks.findIndex(b=>b.id===target.blockId);blocks.splice(at+1,0,...added);const document={...doc,blocks};serializeDocument(document);set({document,past:bounded([...before.past,{document:doc,caret:before.caret}]),future:[],caret:{start:target.from,end:target.from,blockId:target.blockId},focusRequest:after.focusRequest+1,error:null});return true;}
        catch(error){const after=get();for(const [name,asset]of Object.entries(after.assets))if(!before.assets[name])URL.revokeObjectURL(asset.url);set({document:doc,assets:before.assets,past:before.past,future:before.future,caret:before.caret,error:errorMessage(error,'插图失败，原文已保留。')});return false;}
      },
      rename:async(name)=>{
        const s=get();if(!s.document||s.phase!=='idle')return false;
        set({phase:'renaming',error:null,notice:null});
        try{const fileName=normalizeMarkdownName(name);
          if(!s.token){set({fileName,notice:'文件名已修改，首次保存时使用此名称。'});return true;}
          const result=await renameDocumentFile(s.token,fileName);
          if(!result||typeof result.path!=='string'||result.token!==s.token)throw new Error('文件重命名响应异常，请检查磁盘文件后重新打开。');
          set({path:result.path,fileName:result.path.split(/[\\/]/).at(-1)??fileName,notice:'文件已重命名；未保存的正文仍需保存。'});return true;
        }catch(error){set({error:errorMessage(error,'重命名失败，原名称和草稿已保留。')});return false;}
        finally{set({phase:'idle'});}
      },
      replaceDocument:(document)=>{const s=get();if(!s.document||s.phase!=='idle'||s.document.id!==document.id)return false;try{serializeDocument(document);}catch(e){set({error:errorMessage(e,'文档修改失败。')});return false;}if(JSON.stringify(s.document)===JSON.stringify(document))return true;set({document,past:bounded([...s.past,{document:s.document,caret:s.caret}]),future:[],error:null,notice:null});return true;},
      restoreHistory:async(id,revisionId)=>{
        if(!get().document||get().phase!=='idle'||!get().token)return false;
        if(!await get().save())return false;const before=get();if(!before.document||!before.token)return false;
        const created:Record<string,DocumentAsset>={};let adopted=false;set({phase:'opening',error:null,notice:null});
        try{const disk=await loadMarkdownRevision(before.token,id,revisionId);const parsed=parseMarkdown(disk.markdown,before.fileName??undefined),document={...parsed,id:before.document.id};for(const [name,blob]of Object.entries(disk.assets))if(!before.assets[name])created[name]={blob,url:URL.createObjectURL(blob)};
          set({document,assets:{...before.assets,...created},past:bounded([...before.past,{document:before.document,caret:before.caret}]),future:[],caret:{start:0,end:0},focusRequest:get().focusRequest+1,phase:'idle'});adopted=true;
          const ok=await get().save(false,revisionId);if(!ok)set({notice:'恢复尚未完成；恢复草稿保留，恢复前内容已保存到历史，可取消或重试。'});else set({notice:'已追加新的恢复版本，恢复前版本和旧版本均保留。'});return ok;
        }catch(e){if(!adopted)dispose(created);set({error:errorMessage(e,'历史恢复失败；当前已保存版本保留。')});return false;}finally{set({phase:'idle'});}
      },
      save:async(saveAs=false,restoredFrom)=>{
        const s=get();if(!s.document||s.phase!=='idle')return false;if(!saveAs&&!restoredFrom&&s.token&&!documentDirty(s)){set({notice:'内容未修改，本次未重写文件。',error:null});return true;}set({phase:'saving',error:null,notice:null});
        try{
          const markdown=serializeDocument(s.document),references=new Set([...markdown.matchAll(/\]\(assets\/(researchclip-[a-f0-9]{32}\.png)(?=[\s)])/g)].map(match=>match[1]));
          if([...references].some(name=>!s.assets[name]))throw new Error('Markdown 引用的本地图片缺失，请恢复图片或移除对应引用。');
          const assets=Object.fromEntries(Object.entries(s.assets).filter(([name])=>references.has(name)).map(([name,a])=>[name,a.blob]));
          const result=restoredFrom?await saveDocumentFile(markdown,assets,s.token,saveAs,s.fileName??'untitled.md',restoredFrom):await saveDocumentFile(markdown,assets,s.token,saveAs,s.fileName??'untitled.md');if(!result){set({notice:'已取消保存；文档未改动。'});return false;}
          set({token:result.token,path:result.path,fileName:result.path.split(/[\\/]/).at(-1)??s.fileName,savedFingerprint:JSON.stringify(s.document),notice:'Markdown 与 assets 已保存。移动时请保留整个文件夹。'});if(s.token!==result.token)await release(s.token);return true;
        }catch(error){set({error:errorMessage(error,'保存失败，草稿及已有文件已保留。')});return false;}
        finally{set({phase:'idle'});}
      },
    };
  });
}
export type DocumentStore=ReturnType<typeof createDocumentStore>;
const baseDocumentStore=createDocumentStore();
const activeDocument=create<{store:DocumentStore}>(()=>({store:baseDocumentStore}));
const liveDocuments=new Set<DocumentStore>([baseDocumentStore]);
export const DocumentSessionContext=createContext<DocumentStore|null>(null);
export function useDocumentApi(){const local=useContext(DocumentSessionContext),active=activeDocument(s=>s.store);return local??active;}
export function focusDocumentStore(store:DocumentStore){liveDocuments.add(store);activeDocument.setState({store});}
/** Capture a stable session owner before asynchronous work; active tabs may change. */
export function getActiveDocumentStore(){return activeDocument.getState().store;}
export function allDocumentStores(){return [...liveDocuments];}
export function retainDocumentStore(store:DocumentStore){liveDocuments.add(store);}
export function releaseDocumentStore(store:DocumentStore){liveDocuments.delete(store);if(activeDocument.getState().store===store)activeDocument.setState({store:liveDocuments.values().next().value??baseDocumentStore});}
export const useDocumentStore=Object.assign(function useDocumentStore<T=DocumentState>(selector:(s:DocumentState)=>T=(s=>s as unknown as T)){
  const store=useDocumentApi();return store(selector);
},{
  getState:()=>activeDocument.getState().store.getState(),
  getInitialState:()=>baseDocumentStore.getInitialState(),
  setState:((...args:Parameters<DocumentStore['setState']>)=>activeDocument.getState().store.setState(...args)) as DocumentStore['setState'],
  subscribe:(listener:Parameters<DocumentStore['subscribe']>[0])=>activeDocument.getState().store.subscribe(listener),
});
