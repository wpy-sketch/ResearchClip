import {beforeEach,describe,it,expect,vi} from 'vitest';
vi.mock('./api',()=>({status:vi.fn(),start:vi.fn(),cancel:vi.fn(),find:vi.fn(),groups:vi.fn()}));
import * as api from './api';
import {createSimilarityStore} from './store';
const a={id:'a'.repeat(32),title:'A'},b={id:'b'.repeat(32),title:'B'},matches={items:[],page:0,total:0,stale:0},job={phase:'completed',total:2,scanned:2,computed:2,skipped:0,failed:0,error:null,generation:1};
describe('similarity session and asynchronous queries',()=>{
 beforeEach(()=>{vi.resetAllMocks();vi.mocked(api.find).mockResolvedValue(matches);vi.mocked(api.groups).mockResolvedValue({items:[],total:0,page:0});vi.mocked(api.status).mockResolvedValue({stats:{total:2,ready:2,failed:0,algorithmVersion:1},job});});
 it('does not scan automatically and keeps thresholds bounded',()=>{const s=createSimilarityStore();s.getState().show(a);expect(api.start).not.toHaveBeenCalled();s.getState().configure({threshold:99,colorLimit:-5});expect(s.getState().threshold).toBe(64);expect(s.getState().colorLimit).toBe(0);});
 it('discards results for another reference and for a closed panel',async()=>{const s=createSimilarityStore();let finish!:(v:api.Matches)=>void;vi.mocked(api.find).mockReturnValueOnce(new Promise(resolve=>{finish=resolve;}));s.getState().show(a);const query=s.getState().query();s.getState().show(b);finish({...matches,total:9});await query;expect(s.getState().matches).toBeNull();await s.getState().query();expect(api.find).toHaveBeenLastCalledWith(b.id,false,10,35,0);s.getState().close();expect(s.getState().open).toBe(false);});
 it('explicitly builds or cancels without a delete command and preserves retry',async()=>{const s=createSimilarityStore();vi.mocked(api.start).mockRejectedValueOnce(Error('文件失败'));await s.getState().start(false);expect(s.getState().busy).toBe(false);expect(s.getState().error).toContain('文件失败');vi.mocked(api.start).mockResolvedValue(job);await s.getState().start(true);await s.getState().cancel();expect(api.start).toHaveBeenLastCalledWith(true);expect(api.cancel).toHaveBeenCalledOnce();expect(s.getState().status?.stats.ready).toBe(2);});
 it('opens duplicate groups separately from approximate matches',async()=>{const s=createSimilarityStore();s.getState().show();await s.getState().query();expect(api.groups).toHaveBeenCalledWith(0);expect(api.find).not.toHaveBeenCalled();});
});
