import {beforeEach,describe,it,expect,vi} from 'vitest';
vi.mock('@tauri-apps/api/core',()=>({invoke:vi.fn(),isTauri:vi.fn(()=>true)}));
import {invoke,isTauri} from '@tauri-apps/api/core';
import {CitationService} from './CitationService';
const id='a'.repeat(32),record={id,title:'未核对练习',authors:[],year:null,journalOrConference:'',doi:'',url:'',bibtexKey:'Local',bibtexEntry:'',verifiedSource:'manual',sourceMetadata:{},updatedAt:1,revision:1},service=new CitationService();
describe('CitationService has local typed boundaries and no automatic metadata requests',()=>{
 beforeEach(()=>{vi.clearAllMocks();vi.mocked(isTauri).mockReturnValue(true);});
 it('searches existing library by DOI/BibTeX fields without a network lookup',async()=>{vi.mocked(invoke).mockResolvedValue({items:[record],total:1});await service.search('customField',2);expect(invoke).toHaveBeenCalledOnce();expect(invoke).toHaveBeenCalledWith('reference_page',{query:'customField',page:2,target:null});});
 it('rejects unrelated or duplicate returned literature identities',async()=>{for(const rows of [[{...record,id:'b'.repeat(32)}],[record,record]]){vi.mocked(invoke).mockResolvedValue(rows);await expect(service.resolve([id])).rejects.toThrow();}vi.mocked(invoke).mockResolvedValue([]);expect(await service.resolve([id])).toEqual([]);});
 it('keeps BetterBibTeX mappings scoped by instance and library',async()=>{const key={provider:'betterBibtex' as const,instanceId:'b'.repeat(32),libraryId:2,externalKey:'Shared'};vi.mocked(invoke).mockResolvedValue(id);expect(await service.resolveExternalKey(key)).toBe(id);expect(invoke).toHaveBeenLastCalledWith('resolve_citation_key',{key});vi.mocked(invoke).mockResolvedValue(undefined);await service.bindExternalKey(key,id);expect(invoke).toHaveBeenLastCalledWith('bind_citation_key',{key,id});});
 it('validates import/export and reports desktop absence without pretending success',async()=>{vi.mocked(invoke).mockResolvedValue(null);await expect(service.exportBibtex([id])).rejects.toThrow();vi.mocked(invoke).mockResolvedValue([{...record,verifiedSource:'verified'}]);await expect(service.previewBibtex('raw')).rejects.toThrow();vi.mocked(isTauri).mockReturnValue(false);await expect(service.resolve([id])).rejects.toThrow('桌面');});
});
