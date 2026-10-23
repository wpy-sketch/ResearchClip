import {useBatchStore} from '../projects/batch';
import {create} from 'zustand';
import {getDesktopStatus,desktopAction,completeDesktopExit} from '../../shared/api/desktop';
import type {DesktopAction,DesktopSnapshot} from '../../shared/api/desktop';
import {errorMessage} from '../../shared/api/errors';
import {useCardStore} from '../notes/store';
import {useEditorStore} from '../editor/store';
import {useMarkdownExportStore} from '../export/store';
import {useSettingsStore} from '../settings/store';
import {useImportStore} from '../import/store';
import {workspaceManager} from '../workspace/manager';
import {allDocumentStores,useDocumentStore,documentDirty} from '../document/store';
interface State {discardedDocuments:string[];snapshot:DesktopSnapshot|null;error:string|null;phase:'idle'|'checking'|'confirm'|'closing'|'document';refresh:()=>Promise<void>;action:(action:DesktopAction)=>Promise<void>;requestQuit:(skipDocument?:boolean)=>Promise<void>;finishQuit:(approved:boolean)=>Promise<void>;documentQuit:(choice:'save'|'discard'|'cancel')=>Promise<void>}
export const useDesktopStore=create<State>((set,get)=>({discardedDocuments:[],snapshot:null,error:null,phase:'idle',
  refresh:async()=>{try{const snapshot=await getDesktopStatus();set({snapshot,error:snapshot.error});}catch(error){set({error:errorMessage(error,'桌面状态读取失败。')});}},
  action:async(action)=>{try{set({error:null});await desktopAction(action);}catch(error){set({error:errorMessage(error,'桌面操作失败，请重试。')});}},
  finishQuit:async(approved)=>{
    if(approved)set({phase:'closing'});
    try{await completeDesktopExit(approved);if(!approved)set({phase:'idle',discardedDocuments:[]});}
    catch(error){set({phase:'idle',error:errorMessage(error,'退出失败，窗口已保留。')});}
  },
  documentQuit:async(choice)=>{if(get().phase!=='document')return;if(choice==='cancel'){await get().finishQuit(false);return;}const current=useDocumentStore.getState();if(choice==='save'&&!await current.save())return;if(choice==='discard')set({discardedDocuments:[...get().discardedDocuments,current.document?.id??'active']});set({phase:'idle'});await get().requestQuit(true);},
  requestQuit:async(resume=false)=>{
    if(get().phase!=='idle')return;set({phase:'checking',error:null,...(!resume?{discardedDocuments:[]}:{})});
    try{
      if(useEditorStore.getState().pendingIo>0)throw new Error('正在处理图片文件，请完成后重试退出。');
      if(useBatchStore.getState().busy)throw new Error('正在批量处理资料，请完成后重试退出。');
      if(useImportStore.getState().busy)throw new Error('正在导入图片，请完成后重试退出。');
      if(useMarkdownExportStore.getState().busy||useSettingsStore.getState().phase==='saving'||allDocumentStores().some(store=>store.getState().phase!=='idle'||store.getState().pending))throw new Error('正在导出、处理文档或保存设置，请完成操作后重试退出。');
      const dirty=allDocumentStores().find(store=>documentDirty(store.getState())&&!get().discardedDocuments.includes(store.getState().document?.id??'active'));if(dirty){workspaceManager.focusDocument(dirty);set({phase:'document'});return;}
      if(!await useCardStore.getState().flushAll())throw new Error('笔记尚未成功保存，退出已取消，草稿已保留。请保存后重试。');
      const editor=useEditorStore.getState();
      if(editor.gesture||Object.values(editor.sessions).some(s=>JSON.stringify(s.document)!==s.savedDocument)){set({phase:'confirm'});return;}
      await get().finishQuit(true);
    }catch(error){const message=errorMessage(error,'退出前保存失败，退出已取消。');await get().finishQuit(false);set({phase:'idle',error:message});}
  },
}));
