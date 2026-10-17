import {create} from 'zustand';
import {batchLibraryScreenshots,type BatchAction,type SelectionItem} from './api';
import {useCardStore,isDirty} from '../notes/store';
import {useEditorStore} from '../editor/store';
import {useCaptureStore} from '../capture/store';
import {useLibraryStore} from '../library/store';
import {useProjectStore} from './store';
import {useSelectionStore} from './selection';
import {useTagStore} from '../tags/store';
import {errorMessage} from '../../shared/api/errors';
interface State {busy:boolean;error:string|null;notice:string|null;run:(items:SelectionItem[],action:BatchAction)=>Promise<boolean>}
export const useBatchStore=create<State>((set,get)=>({busy:false,error:null,notice:null,
 run:async(input,action)=>{if(get().busy)return false;set({busy:true,error:null,notice:null});const locked:string[]=[],items=input.map(i=>({...i}));let committed=false;
 try{
  for(const item of items){const before=useCardStore.getState().sessions[item.id];if(before&&isDirty(before)){if(!await useCardStore.getState().flush(item.id))throw new Error('所选卡片有未保存笔记，请保存后重试。');const saved=useCardStore.getState().sessions[item.id];if(before.card?.screenshot.revision===item.revision&&saved?.card)item.revision=saved.card.screenshot.revision;}
   if(!useCardStore.getState().beginDelete(item.id))throw new Error('所选卡片正在加载或保存；本批次尚未执行，请稍后重试。');locked.push(item.id);
  }
  const result=await batchLibraryScreenshots(items,action);committed=true;
  for(const item of items){const editor=useEditorStore.getState();if(action.kind==='delete'){editor.unlinkLibrary(item.id);const preview=useCaptureStore.getState().preview;if(preview?.library?.id===item.id){const next={...preview};delete next.library;useCaptureStore.setState({preview:next});}}
   else for(const session of Object.values(editor.sessions))if(session.image.libraryId===item.id&&session.image.libraryRevision===item.revision)editor.bindLibrary(session.image.id,item.id,item.revision+1,session.image.name);
  }
  useSelectionStore.getState().clear();set({notice:`已处理 ${result.affected} 张图片。${result.pendingCleanup?`仍有 ${result.pendingCleanup} 个图片文件待清理，可重试恢复与清理。`:''}`});return true;
 }catch(e){set({error:errorMessage(e,'批量操作失败，尚未完成项未自动重试；请刷新后检查。')});return false;}
 finally{for(const id of locked)useCardStore.getState().finishDelete(id,committed&&action.kind==='delete');if(committed){for(const id of locked)if(action.kind!=='delete'&&useCardStore.getState().sessions[id])await useCardStore.getState().reload(id);await useLibraryStore.getState().refresh();const lib=useLibraryStore.getState();await useProjectStore.getState().refresh(lib.projectId,lib.unclassified);await useTagStore.getState().refresh();}set({busy:false});}
 },
}));
