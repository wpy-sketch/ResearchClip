import {create} from 'zustand';
import {importFiles,importClipboard,type ImportReport} from './api';
import {useDocumentStore,getActiveDocumentStore} from '../document/store';
import {useLibraryStore} from '../library/store';
import {errorMessage} from '../../shared/api/errors';
type Target='library'|'markdown';
interface State {busy:boolean;report:ImportReport|null;error:string|null;inserted:boolean;run:(source:readonly File[]|'clipboard',target:Target)=>Promise<void>;insertLast:()=>Promise<void>;clear:()=>void}
export const useImportStore=create<State>((set,get)=>({
  busy:false,report:null,error:null,inserted:false,
  clear:()=>{if(!get().busy)set({report:null,error:null,inserted:false});},
  run:async(source,target)=>{
    if(get().busy)return;const documentStore=getActiveDocumentStore(),doc=documentStore.getState(),documentId=target==='markdown'?doc.document?.id:null;
    if(target==='markdown'&&(!documentId||doc.phase!=='idle'||doc.pending)){set({error:'请先打开可编辑的 Markdown 文档，图片尚未导入。'});return;}
    set({busy:true,error:null,report:null,inserted:false});
    // Reserve the existing session while native decoding/storage runs. Close/save/edit remain guarded.
    if(documentId)documentStore.setState({phase:'adding'});
    try{
      const report=await(source==='clipboard'?importClipboard():importFiles(source));set({report});
      const ids=report.items.flatMap(i=>i.screenshot?[i.screenshot.id]:[]);
      if(documentId){
        const current=documentStore.getState();if(current.document?.id!==documentId)throw new Error('图片已导入素材库，但原 Markdown 会话已改变；请手动插入。');
        documentStore.setState({phase:'idle'});
        if(ids.length){const ok=await documentStore.getState().addImages(ids);if(!ok)throw new Error('图片已存入素材库，但未插入 Markdown。请检查文档提示并从素材库重新插入。');set({inserted:true});}
      }
    }catch(e){set({error:errorMessage(e,'图片导入失败；请刷新素材库确认是否存在已完成项，不会覆盖外部原图。')});}
    finally{const current=documentStore.getState();if(documentId&&current.phase==='adding')documentStore.setState({phase:'idle'});set({busy:false});await useLibraryStore.getState().refresh(undefined,{page:0});}
  },
  insertLast:async()=>{const s=get(),doc=useDocumentStore.getState(),ids=s.report?.items.flatMap(i=>i.screenshot?[i.screenshot.id]:[])??[];if(s.busy||!ids.length||!doc.document||doc.phase!=='idle')return;set({busy:true,error:null});try{if(await doc.addImages(ids))set({inserted:true});else set({error:'插入失败，已导入的素材仍在素材库；请检查 Markdown 提示。'});}catch(e){set({error:errorMessage(e,'插入失败。')});}finally{set({busy:false});}},
}));
