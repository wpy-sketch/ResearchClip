import {beforeEach,describe,expect,it,vi} from 'vitest';
import {exportKnowledgeMarkdown,parseExportReport,validateExportIds} from './markdown';
const native=vi.hoisted(()=>({invoke:vi.fn(),isTauri:vi.fn(()=>true)}));vi.mock('@tauri-apps/api/core',()=>native);
const a='a'.repeat(32),b='b'.repeat(32),report={directory:'D:\\科研 导出\\ResearchClip-Export',exportedIds:[a],failures:[],warnings:[]};
beforeEach(()=>{vi.resetAllMocks();native.isTauri.mockReturnValue(true);});
describe('bounded native Markdown export contract',()=>{
  it('sends only card IDs, receives a directory and accepts cancellation',async()=>{native.invoke.mockResolvedValue(report);expect(await exportKnowledgeMarkdown([a])).toEqual(report);expect(native.invoke).toHaveBeenCalledWith('export_knowledge_markdown',{ids:[a]});native.invoke.mockResolvedValue(null);expect(await exportKnowledgeMarkdown([a])).toBeNull();});
  it('accepts explicit partial results and source warnings',()=>{const partial={...report,failures:[{id:b,message:'原图缺失'}],warnings:[{id:a,message:'来源无效'}]};expect(parseExportReport(partial,[a,b])).toEqual(partial);});
  it('rejects wrong IDs, duplicates, missing outcomes and malformed issues',()=>{for(const value of [{...report,exportedIds:[b]},{...report,exportedIds:[a,a]},{...report,exportedIds:[]},{...report,failures:[{id:b,message:'missing'}]},{...report,warnings:[{id:b,message:'wrong'}]},{...report,failures:'bad'},{...report,directory:'x\0y'}])expect(()=>parseExportReport(value,[a])).toThrow();expect(()=>parseExportReport(report,[a,b])).toThrow();});
  it('validates bounds and refuses browser export before IPC',async()=>{for(const ids of [[],[a,a],['../a'],Array.from({length:101},(_,n)=>n.toString(16).padStart(32,'0'))])expect(()=>validateExportIds(ids)).toThrow();native.isTauri.mockReturnValue(false);await expect(exportKnowledgeMarkdown([a])).rejects.toThrow('桌面');expect(native.invoke).not.toHaveBeenCalled();});
  it('preserves native filesystem failures',async()=>{native.invoke.mockRejectedValue({code:'FILE_UNAVAILABLE',message:'所选目录无法写入'});await expect(exportKnowledgeMarkdown([a])).rejects.toMatchObject({code:'FILE_UNAVAILABLE'});});
});
