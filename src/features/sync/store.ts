import {create} from 'zustand';
import {isTauri} from '@tauri-apps/api/core';
import {syncStatus,configureSync,startSync,stopSync,resolveSync,authorizeSyncDocuments,type SyncConfig,type SyncStatus} from './api';
import {errorMessage} from '../../shared/api/errors';
import {useDocumentStore,documentDirty} from '../document/store';
import {useEditorStore} from '../editor/store';
import {useCardStore} from '../notes/store';
interface State {status:SyncStatus|null;busy:boolean;error:string|null;notice:string|null;generation:number;refresh:()=>Promise<void>;save:(config:SyncConfig,username:string,password:string)=>Promise<boolean>;start:(test:boolean)=>Promise<void>;stop:()=>Promise<void>;resolve:(id:string,choice:'local'|'remote'|'both')=>Promise<boolean>;authorize:()=>Promise<void>}
export const useSyncStore=create<State>((set,get)=>{
 const requireSaved=async()=>{if(documentDirty(useDocumentStore.getState())||Object.values(useEditorStore.getState().sessions).some(s=>JSON.stringify(s.document)!==s.savedDocument))throw Error('请先保存或关闭未保存的 Markdown 与标注草稿，再开始同步；草稿不会自动上传。');if(!await useCardStore.getState().flushAll())throw Error('笔记保存失败，未开始同步。');};
 const action=async(fn:()=>Promise<void>)=>{if(get().busy)return false;set({busy:true,error:null,notice:null});try{await fn();await get().refresh();return true;}catch(e){set({error:errorMessage(e,'同步操作失败；已有资料保留。')});return false;}finally{set({busy:false});}};
 return {status:null,busy:false,error:null,notice:null,generation:0,
 refresh:async()=>{if(!isTauri())return;const generation=get().generation+1;set({generation});try{const status=await syncStatus();if(get().generation===generation)set({status});}catch(e){if(get().generation===generation)set({error:errorMessage(e,'同步状态读取失败。')});}},
 save:(config,username,password)=>action(()=>configureSync(config,username,password)),
 start:async(test)=>{await action(async()=>{if(!test)await requireSaved();await startSync(test);});},
 stop:async()=>{await action(stopSync);},
 resolve:(id,choice)=>action(async()=>{await requireSaved();await resolveSync(id,choice);}),
 authorize:async()=>{await action(async()=>{const count=await authorizeSyncDocuments();set({notice:`已授权 ${count} 份已登记文档；取消选择不会扩大授权。`});});},
 };
});
