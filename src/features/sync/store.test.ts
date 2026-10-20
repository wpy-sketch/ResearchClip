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
 beforeEach(()=>{vi.clearAllMocks();vi.mocked(documentDirty).mockReturnValue(false);useSyncStore.setState({status:null,busy:false,operation:null,stopping:false,stopRequested:false,refreshing:false,statusError:null,error:null,notice:null,generation:0});vi.mocked(invoke).mockResolvedValue(status);});
 it('refresh only reads status; never starts network synchronization automatically',async()=>{await useSyncStore.getState().refresh();expect(invoke).toHaveBeenCalledExactlyOnceWith('sync_status',undefined);expect(useSyncStore.getState().status?.job.phase).toBe('idle');});
 it('keeps unsaved Markdown and blocks starting sync',async()=>{vi.mocked(documentDirty).mockReturnValue(true);await useSyncStore.getState().start(false);expect(invoke).not.toHaveBeenCalled();expect(useSyncStore.getState().error).toContain('草稿');expect(useSyncStore.getState().busy).toBe(false);});
 it('blocks conflict resolution while a Markdown draft is unsaved',async()=>{vi.mocked(documentDirty).mockReturnValue(true);expect(await useSyncStore.getState().resolve('a'.repeat(32),'remote')).toBe(false);expect(invoke).not.toHaveBeenCalled();expect(useSyncStore.getState().error).toContain('草稿');});
 it('reports connection failures and keeps retry available',async()=>{vi.mocked(invoke).mockRejectedValue({message:'服务器不支持安全条件写入'});await useSyncStore.getState().start(true);expect(useSyncStore.getState().error).toContain('条件写入');expect(useSyncStore.getState().busy).toBe(false);expect(useSyncStore.getState().status).toBeNull();});
 it('only resolves after an explicit confirmed action and supports stop',async()=>{vi.mocked(invoke).mockImplementation(async(name)=>name==='sync_status'?status:undefined);await useSyncStore.getState().resolve('a'.repeat(32),'both');expect(invoke).toHaveBeenCalledWith('sync_resolve',{id:'a'.repeat(32),choice:'both',confirmed:true});await useSyncStore.getState().stop();expect(invoke).toHaveBeenCalledWith('sync_stop',undefined);});
 it('releases action controls when only a background status read is delayed',async()=>{
  let finish!: (value:unknown)=>void;
  vi.mocked(invoke).mockImplementation(async name=>name==='sync_status'?new Promise(resolve=>{finish=resolve;}):undefined);
  const action=useSyncStore.getState().start(true);
  await vi.waitFor(()=>expect(invoke).toHaveBeenCalledWith('sync_status',undefined));
  const busy=useSyncStore.getState().busy;
  finish(status);await action;
  expect(busy).toBe(false);
 });
 it('can request stop even while a different command acknowledgement is pending',async()=>{
  let finish!: (value:unknown)=>void;
  vi.mocked(invoke).mockImplementation(async name=>name==='sync_start'?new Promise(resolve=>{finish=resolve;}):name==='sync_status'?status:undefined);
  const start=useSyncStore.getState().start(true);
  await vi.waitFor(()=>expect(invoke).toHaveBeenCalledWith('sync_start',{test:true}));
  await useSyncStore.getState().stop();const stopped=vi.mocked(invoke).mock.calls.some(([name])=>name==='sync_stop');
  finish(undefined);await start;
  expect(stopped).toBe(true);
  expect(vi.mocked(invoke).mock.calls.filter(([name])=>name==='sync_stop')).toHaveLength(2);
 });
 it('acknowledges a real save without waiting for status-only transport and ignores stale reads',async()=>{
  let finish!:(v:unknown)=>void;const saved={...status,credentialAvailable:true,config:{server:'https://example.com/dav/',folder:'researchclip-test',projects:[],unclassified:true}};
  vi.mocked(invoke).mockImplementation(async name=>name==='sync_status'?new Promise(resolve=>{finish=resolve;}):saved);
  const read=useSyncStore.getState().refresh();expect(await useSyncStore.getState().save(saved.config,'user','secret')).toBe(true);
  expect(useSyncStore.getState().status?.config).toEqual(saved.config);expect(useSyncStore.getState().busy).toBe(false);
  finish(status);await read;expect(useSyncStore.getState().status?.config).toEqual(saved.config);
 });
 it('coalesces repeated status reads and preserves a failed save error across polls',async()=>{
  let finish!:(v:unknown)=>void;vi.mocked(invoke).mockImplementation(async name=>name==='sync_status'?new Promise(resolve=>{finish=resolve;}):Promise.reject(Error('凭据写入失败')));
  const reads=[useSyncStore.getState().refresh(),useSyncStore.getState().refresh()];expect(invoke).toHaveBeenCalledTimes(1);finish(status);await Promise.all(reads);
  expect(await useSyncStore.getState().save({server:'https://example.com/',folder:'researchclip-test',projects:[],unclassified:false},'user','secret')).toBe(false);
  expect(useSyncStore.getState().busy).toBe(false);expect(useSyncStore.getState().error).toContain('凭据写入失败');
 });
 it('reports status-only failure separately and allows recovery',async()=>{
  vi.mocked(invoke).mockRejectedValue(Error('读取失败'));await useSyncStore.getState().refresh();expect(useSyncStore.getState().statusError).toContain('读取失败');expect(useSyncStore.getState().busy).toBe(false);
  vi.mocked(invoke).mockResolvedValue(status);await useSyncStore.getState().refresh();expect(useSyncStore.getState().statusError).toBeNull();
 });
});
