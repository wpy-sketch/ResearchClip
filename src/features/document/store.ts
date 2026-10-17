import {create} from 'zustand';
import {getScreenshot,getKnowledgeCard,screenshotImage} from '../../shared/api/library';
import {useEditorStore} from '../editor/store';
import {useCardStore} from '../notes/store';
import {errorMessage} from '../../shared/api/errors';
import {createDocument,newId,moveBlock,parseMarkdown,serializeDocument,MAX_BLOCKS,MAX_IMAGES,MAX_ASSET_BYTES,createBlankDocument,asSource,sourceText,escapeInline} from './model';
import type {MarkdownDocument,DocumentAsset,ImageBlock} from './model';
import {saveDocumentFile,openDocumentFile,releaseDocument,renameDocumentFile} from './api';
import {normalizeMarkdownName} from './file-name';
export type DocumentAction='new'|'blank'|'open'|'close';
interface TextEdit {document:MarkdownDocument;caret:{start:number;end:number}}
const bounded=(items:TextEdit[])=>{let bytes=0;return items.slice(-100).reverse().filter(item=>{bytes+=JSON.stringify(item.document).length*2;return bytes<=8*1024*1024;}).reverse();};
interface State {
  caret:{start:number;end:number};past:TextEdit[];future:TextEdit[];focusRequest:number;
  setCaret:(start:number,end:number)=>void;editSource:(text:string,start?:number,end?:number)=>void;toSource:()=>void;undoText:()=>void;redoText:()=>void;insertEditor:()=>Promise<boolean>;
  document:MarkdownDocument|null;assets:Record<string,DocumentAsset>;token:string|null;path:string|null;fileName:string|null;savedFingerprint:string|null;
  phase:'idle'|'adding'|'saving'|'opening'|'renaming';error:string|null;notice:string|null;pending:DocumentAction|null;
  request:(action:DocumentAction)=>Promise<void>;resolvePending:(choice:'save'|'discard'|'cancel')=>Promise<void>;
  editTitle:(title:string)=>void;editBlock:(id:string,text:string)=>void;addText:(afterId?:string)=>void;move:(id:string,delta:-1|1)=>void;remove:(id:string)=>void;
  addImages:(ids:string[])=>Promise<boolean>;save:(saveAs?:boolean)=>Promise<boolean>;rename:(name:string)=>Promise<boolean>;
}
export const documentDirty=(s:Pick<State,'document'|'savedFingerprint'>)=>Boolean(s.document&&(s.savedFingerprint===null||JSON.stringify(s.document)!==s.savedFingerprint));
const dispose=(assets:Record<string,DocumentAsset>)=>Object.values(assets).forEach(a=>URL.revokeObjectURL(a.url));
export function createDocumentStore(){
  return create<State>((set,get)=>{
    const release=async(token:string|null)=>{try{await releaseDocument(token);}catch{set({error:'旧文档文件授权未能释放；已有文件未改动，请保存当前文档后重启。'});}};
    const run=async(action:DocumentAction)=>{
      if(action==='open'){
        set({phase:'opening',error:null,notice:null,pending:null});let openedToken:string|null=null;
        try{
          const disk=await openDocumentFile();if(!disk)return;
          openedToken=disk.token;const document=parseMarkdown(disk.markdown,disk.path.split(/[\\/]/).at(-1));
          for(const b of document.blocks)if(b.type==='image'&&!disk.assets[b.assetName])throw new Error('文档引用的图片缺失，当前草稿已保留。');
          const assets:Record<string,DocumentAsset>={};
          try{for(const [name,blob] of Object.entries(disk.assets))assets[name]={blob,url:URL.createObjectURL(blob)};}catch(error){dispose(assets);throw error;}
          const old=get();dispose(old.assets);set({document,caret:{start:0,end:0},past:[],future:[],focusRequest:get().focusRequest+1,assets,token:disk.token,path:disk.path,fileName:disk.path.split(/[\\/]/).at(-1)??'untitled.md',savedFingerprint:JSON.stringify(document),pending:null,notice:'Markdown 已打开；图片与注释顺序已恢复。'});openedToken=null;await release(old.token);
        }catch(error){set({error:errorMessage(error,'打开失败，当前文档已保留。')});}
        finally{if(openedToken)await release(openedToken);set({phase:'idle'});}
      }else{
        const old=get();dispose(old.assets);set({document:action==='new'?createDocument():action==='blank'?createBlankDocument():null,caret:{start:0,end:0},past:[],future:[],focusRequest:get().focusRequest+1,assets:{},token:null,path:null,fileName:action==='new'?'未命名文档.md':action==='blank'?'untitled.md':null,savedFingerprint:null,pending:null,error:null,notice:null});await release(old.token);
      }
    };
    const change=(fn:(doc:MarkdownDocument)=>MarkdownDocument)=>{const s=get();if(s.document&&s.phase==='idle'){const document=fn(s.document);if(document!==s.document)set({document,notice:null,error:null});}};
    const updateSource=(text:string,start:number,end:number)=>{
      const s=get();if(!s.document||s.phase!=='idle'||s.document.mode!=='source')return;
      const document={...s.document,blocks:[{id:s.document.blocks[0].id,type:'text' as const,markdown:text}]};
      try{serializeDocument(document);}catch(error){set({error:errorMessage(error,'文档过大。')});return;}
      if(text===sourceText(s.document)){set({caret:{start,end}});return;}
      set({document,caret:{start,end},past:bounded([...s.past,{document:s.document,caret:s.caret}]),future:[],notice:null,error:null});
    };
    const append=(document:MarkdownDocument,blocks:ImageBlock[])=>{
      if(document.mode!=='source')return {...document,blocks:[...document.blocks,...blocks]};
      const s=get(),text=sourceText(document),start=Math.min(s.caret.start,text.length),end=Math.min(s.caret.end,text.length);
      const inserted=blocks.map(b=>`![${escapeInline(b.alt)}](assets/${b.assetName})\n\n${b.caption}`).join('\n\n')+'\n\n';
      const next={...document,blocks:[{id:document.blocks[0].id,type:'text' as const,markdown:text.slice(0,start)+inserted+text.slice(end)}]};
      serializeDocument(next);
      set({caret:{start:start+inserted.length,end:start+inserted.length},past:bounded([...s.past,{document,caret:s.caret}]),future:[],focusRequest:s.focusRequest+1});return next;
    };
    return {
      caret:{start:0,end:0},past:[],future:[],focusRequest:0,
      setCaret:(start,end)=>{if(Number.isSafeInteger(start)&&Number.isSafeInteger(end)&&start>=0&&end>=start)set({caret:{start,end}});},
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
          const id=newId(),assetName=`researchclip-${id}.png`,block:ImageBlock={id,type:'image',assetName,sourceScreenshotId:session.image.libraryId??null,alt:session.image.name.slice(0,200),caption:''};
          url=URL.createObjectURL(blob);const document=append(s.document,[block]);serializeDocument(document);
          set({document,assets:{...s.assets,[assetName]:{blob,url}},notice:'当前标注合成图已插入 Markdown，原图未改动。'});return true;
        }catch(error){if(url)URL.revokeObjectURL(url);set({error:errorMessage(error,'插入图片失败。')});return false;}finally{set({phase:'idle'});}
      },
      document:null,assets:{},token:null,path:null,fileName:null,savedFingerprint:null,phase:'idle',error:null,notice:null,pending:null,
      request:async(action)=>{if(get().phase!=='idle'){set({error:'文档正在处理，请等待完成。'});return;}if(documentDirty(get())){set({pending:action});return;}await run(action);},
      resolvePending:async(choice)=>{const action=get().pending;if(!action||get().phase!=='idle')return;if(choice==='cancel'){set({pending:null});return;}if(choice==='save'&&!await get().save())return;await run(action);},
      editTitle:(title)=>change(doc=>({...doc,title})),
      editBlock:(id,text)=>change(doc=>({...doc,blocks:doc.blocks.map(b=>b.id===id?b.type==='image'?{...b,caption:text.replaceAll('\r\n','\n')}:{...b,markdown:text.replaceAll('\r\n','\n')}:b)})),
      addText:(afterId)=>change(doc=>{if(doc.blocks.length>=MAX_BLOCKS){set({error:'每份文档最多100个内容块。'});return doc;}const blocks=[...doc.blocks],index=afterId?blocks.findIndex(b=>b.id===afterId)+1:blocks.length;blocks.splice(index,0,{id:newId(),type:'text',markdown:''});return {...doc,blocks};}),
      move:(id,delta)=>change(doc=>moveBlock(doc,id,delta)),
      remove:(id)=>change(doc=>{const block=doc.blocks.find(b=>b.id===id);if(block?.type==='image'){const assets={...get().assets};if(assets[block.assetName])URL.revokeObjectURL(assets[block.assetName].url);delete assets[block.assetName];set({assets});}return {...doc,blocks:doc.blocks.filter(b=>b.id!==id)};}),
      addImages:async(ids)=>{
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
            assets[name]={blob,url:URL.createObjectURL(blob)};blocks.push({id:blockId,type:'image',assetName:name,sourceScreenshotId:id,alt:image.title,caption:caption.replaceAll('\r\n','\n')});
          }
          const document=append(s.document,blocks);serializeDocument(document);
          set({document,assets:{...s.assets,...assets},notice:`已添加 ${blocks.length} 张含标注的图片；注释为添加时的笔记快照。`});return true;
        }catch(error){dispose(assets);set({error:errorMessage(error,'添加失败，已有内容未改动。'),notice:null});return false;}
        finally{set({phase:'idle'});}
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
      save:async(saveAs=false)=>{
        const s=get();if(!s.document||s.phase!=='idle')return false;set({phase:'saving',error:null,notice:null});
        try{
          const markdown=serializeDocument(s.document),references=new Set([...markdown.matchAll(/\]\(assets\/(researchclip-[a-f0-9]{32}\.png)\)/g)].map(match=>match[1]));
          if([...references].some(name=>!s.assets[name]))throw new Error('Markdown 引用的本地图片缺失，请恢复图片或移除对应引用。');
          const assets=Object.fromEntries(Object.entries(s.assets).filter(([name])=>references.has(name)).map(([name,a])=>[name,a.blob]));
          const result=await saveDocumentFile(markdown,assets,s.token,saveAs,s.fileName??'untitled.md');if(!result){set({notice:'已取消保存；文档未改动。'});return false;}
          set({token:result.token,path:result.path,fileName:result.path.split(/[\\/]/).at(-1)??s.fileName,savedFingerprint:JSON.stringify(s.document),notice:'Markdown 与 assets 已保存。移动时请保留整个文件夹。'});if(s.token!==result.token)await release(s.token);return true;
        }catch(error){set({error:errorMessage(error,'保存失败，草稿及已有文件已保留。')});return false;}
        finally{set({phase:'idle'});}
      },
    };
  });
}
export const useDocumentStore=createDocumentStore();
