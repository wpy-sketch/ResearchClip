import {beforeEach,describe,it,expect,vi} from 'vitest';
vi.mock('@tauri-apps/api/core',()=>({isTauri:()=>true,invoke:vi.fn()}));
vi.mock('../document/store',()=>({useDocumentStore:{getState:()=>({dirty:false})},documentDirty:vi.fn(()=>false)}));
vi.mock('../editor/store',()=>({useEditorStore:{getState:()=>({sessions:{}})}}));
vi.mock('../notes/store',()=>({useCardStore:{getState:()=>({flushAll:async()=>true})}}));
import {invoke} from '@tauri-apps/api/core';
import {documentDirty} from '../document/store';
import {useSyncStore} from './store';
const status={config:null,credentialAvailable:false,pending:0,conflicts:[],job:{phase:'idle',message:'尚未启用',writable:false,progress:{completed:0,total:0,uploaded:0,downloaded:0,conflicts:0,skipped:0},remoteProjects:[]}};
describe('explicit sync actions',()=>{
 beforeEach(()=>{vi.clearAllMocks();vi.mocked(documentDirty).mockReturnValue(false);useSyncStore.setState({status:null,busy:false,error:null,notice:null,generation:0});vi.mocked(invoke).mockResolvedValue(status);});
 it('refresh only reads status; never starts network synchronization automatically',async()=>{await useSyncStore.getState().refresh();expect(invoke).toHaveBeenCalledExactlyOnceWith('sync_status',undefined);expect(useSyncStore.getState().status?.job.phase).toBe('idle');});
 it('keeps unsaved Markdown and blocks starting sync',async()=>{vi.mocked(documentDirty).mockReturnValue(true);await useSyncStore.getState().start(false);expect(invoke).not.toHaveBeenCalled();expect(useSyncStore.getState().error).toContain('草稿');expect(useSyncStore.getState().busy).toBe(false);});
 it('blocks conflict resolution while a Markdown draft is unsaved',async()=>{vi.mocked(documentDirty).mockReturnValue(true);expect(await useSyncStore.getState().resolve('a'.repeat(32),'remote')).toBe(false);expect(invoke).not.toHaveBeenCalled();expect(useSyncStore.getState().error).toContain('草稿');});
 it('reports connection failures and keeps retry available',async()=>{vi.mocked(invoke).mockRejectedValue({message:'服务器不支持安全条件写入'});await useSyncStore.getState().start(true);expect(useSyncStore.getState().error).toContain('条件写入');expect(useSyncStore.getState().busy).toBe(false);expect(useSyncStore.getState().status).toBeNull();});
 it('only resolves after an explicit confirmed action and supports stop',async()=>{vi.mocked(invoke).mockImplementation(async(name)=>name==='sync_status'?status:undefined);await useSyncStore.getState().resolve('a'.repeat(32),'both');expect(invoke).toHaveBeenCalledWith('sync_resolve',{id:'a'.repeat(32),choice:'both',confirmed:true});await useSyncStore.getState().stop();expect(invoke).toHaveBeenCalledWith('sync_stop',undefined);});
});
