import {beforeEach,describe,it,expect,vi} from 'vitest';
import {useProjectStore} from './store';
const api=vi.hoisted(()=>({listProjects:vi.fn(),listProjectDocuments:vi.fn(),syncProjectDocument:vi.fn()}));vi.mock('./api',()=>api);vi.mock('@tauri-apps/api/core',()=>({isTauri:()=>true}));
const id='a'.repeat(32),document={id,name:'研究.md',projectId:null,updatedAt:1,available:true};
beforeEach(()=>{vi.clearAllMocks();useProjectStore.setState(useProjectStore.getInitialState(),true);api.listProjects.mockResolvedValue([]);api.listProjectDocuments.mockResolvedValue([]);});
describe('project index and query state (mocked IPC)',()=>{
 it('keeps index failure separate from a successful file/list operation and permits retry',async()=>{api.syncProjectDocument.mockRejectedValueOnce(new Error('index disk error')).mockResolvedValueOnce(document);await useProjectStore.getState().sync('grant');expect(useProjectStore.getState().indexError).toContain('index disk error');await useProjectStore.getState().refresh();expect(useProjectStore.getState().indexError).toContain('index disk error');await useProjectStore.getState().sync('grant');expect(useProjectStore.getState()).toMatchObject({current:document,token:'grant',indexError:null});});
 it('ignores late documents from the previous project filter',async()=>{let finish:(v:unknown)=>void=()=>{};api.listProjectDocuments.mockImplementationOnce(()=>new Promise(r=>{finish=r;})).mockResolvedValueOnce([]);const old=useProjectStore.getState().refresh(id,false);await useProjectStore.getState().refresh(null,true);finish([document]);await old;expect(useProjectStore.getState().documents).toEqual([]);expect(api.listProjectDocuments).toHaveBeenLastCalledWith(null,true);});
});

it('strict file saves surface failed index/history commits instead of claiming completion',async()=>{api.syncProjectDocument.mockRejectedValueOnce(Error('history commit failed'));await expect(useProjectStore.getState().sync('grant',false,null,true)).rejects.toThrow('history commit failed');expect(useProjectStore.getState().indexError).toContain('history commit failed');expect(useProjectStore.getState().byToken.grant).toBeUndefined();});
