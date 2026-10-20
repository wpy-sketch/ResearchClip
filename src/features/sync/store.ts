import {create} from 'zustand';
import {isTauri} from '@tauri-apps/api/core';
import {syncStatus,configureSync,startSync,stopSync,resolveSync,authorizeSyncDocuments,type SyncConfig,type SyncStatus} from './api';
import {errorMessage} from '../../shared/api/errors';
import {useDocumentStore,documentDirty} from '../document/store';
import {useEditorStore} from '../editor/store';
import {useCardStore} from '../notes/store';
interface State {
 status:SyncStatus|null;busy:boolean;operation:string|null;stopping:boolean;stopRequested:boolean;
 refreshing:boolean;statusError:string|null;error:string|null;notice:string|null;generation:number;
 refresh:()=>Promise<void>;save:(config:SyncConfig,username:string,password:string)=>Promise<boolean>;
 start:(test:boolean)=>Promise<void>;stop:()=>Promise<void>;
 resolve:(id:string,choice:'local'|'remote'|'both')=>Promise<boolean>;authorize:()=>Promise<void>;
}
export const useSyncStore=create<State>((set,get)=>{
 let read:Promise<void>|null=null;
 const requireSaved=async()=>{if(documentDirty(useDocumentStore.getState())||Object.values(useEditorStore.getState().sessions).some(s=>JSON.stringify(s.document)!==s.savedDocument))throw Error('请先保存或关闭未保存的 Markdown 与标注草稿，再开始同步；草稿不会自动上传。');if(!await useCardStore.getState().flushAll())throw Error('笔记保存失败，未开始同步。');};
 // Only the mutation owns busy. A delayed status poll must not lock every control.
 const action=async(operation:string,fn:()=>Promise<void>)=>{
  if(get().busy||get().stopping)return false;
  set({busy:true,operation,error:null,notice:null,generation:get().generation+1});
  let succeeded=false;
  try{await fn();succeeded=true;return true;}
  catch(e){set({error:errorMessage(e,'同步操作失败；已有资料保留。')});return false;}
  finally{set({busy:false,operation:null,generation:get().generation+1});if(succeeded)void get().refresh();}
 };
 const start=async(fn:()=>Promise<void>)=>{
  set({stopRequested:false});await fn();
  // Honour cancellation even if it preceded a delayed start acknowledgement.
  if(get().stopRequested)await stopSync();
 };
 return {status:null,busy:false,operation:null,stopping:false,stopRequested:false,refreshing:false,statusError:null,error:null,notice:null,generation:0,
 refresh:()=>{
  if(!isTauri())return Promise.resolve();if(read)return read;
  const generation=get().generation;set({refreshing:true});
  read=(async()=>{try{const status=await syncStatus();if(get().generation===generation)set({status,statusError:null});}
   catch(e){if(get().generation===generation)set({statusError:errorMessage(e,'同步状态读取失败，请重试；已有配置保留。')});}
   finally{read=null;set({refreshing:false});}})();return read;
 },
 save:(config,username,password)=>action('正在保存本机连接与凭据…',async()=>{
  const status=await configureSync(config,username,password);
  set({status,statusError:null,generation:get().generation+1,notice:'连接与同步范围已保存在本机。接下来勾选探针授权，再测试连接。'});
 }),
 start:async(test)=>{await action(test?'正在启动连接测试…':'正在启动手动同步…',()=>start(async()=>{if(!test)await requireSaved();await startSync(test);}));},
 stop:async()=>{
  if(get().stopping)return;set({stopping:true,stopRequested:true,error:null});
  try{await stopSync();set({notice:'已请求停止；正在等待当前网络请求结束，已有资料保留。'});}
  catch(e){set({error:errorMessage(e,'停止请求失败，请重试。')});}
  finally{set({stopping:false});void get().refresh();}
 },
 resolve:(id,choice)=>action('正在启动冲突处理…',()=>start(async()=>{await requireSaved();await resolveSync(id,choice);})),
 authorize:async()=>{await action('等待选择 Markdown 授权目录…',async()=>{const count=await authorizeSyncDocuments();set({notice:`已授权 ${count} 份已登记文档；取消选择不会扩大授权。`});});},
 };
});
