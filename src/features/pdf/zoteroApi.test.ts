import {describe,it,expect,vi} from 'vitest';
vi.mock('@tauri-apps/api/core',()=>({invoke:vi.fn()}));
import {invoke} from '@tauri-apps/api/core';
import {preview,sources} from './zoteroApi';
const data={title:'未核实练习',authors:[],year:null,journalOrConference:'',doi:'',url:'',bibtexKey:'Practice',bibtexEntry:'',verifiedSource:'manual',sourceMetadata:{provider:'zoteroLocal',status:'unverified'}};
const c={instanceId:'a'.repeat(32),libraryType:'user',libraryId:0,itemKey:'PARENT01',attachmentKey:'ATTACH01',reference:data,warnings:[]};
const value={health:{apiVersion:3,schemaVersion:44,productVersion:null},candidates:[{id:'b'.repeat(32),candidate:c}],message:'待核对'};
describe('Zotero source IPC validation (unit mocks)',()=>{
 it('preserves unknown fields as absent and requires scoped stable keys',()=>{expect(preview(value).candidates[0].candidate.reference.year).toBeNull();for(const bad of [{...c,libraryId:-1},{...c,libraryId:1.5},{...c,itemKey:'bad key'},{...c,instanceId:'raw-server'}])expect(()=>preview({...value,candidates:[{id:'b'.repeat(32),candidate:bad}]})).toThrow();});
 it('rejects unsupported versions, oversized candidates and forged preview IDs',()=>{expect(()=>preview({...value,health:{apiVersion:2}})).toThrow();expect(()=>preview({...value,candidates:Array(9).fill(value.candidates[0])})).toThrow();expect(()=>preview({...value,candidates:[{...value.candidates[0],id:'../path'}]})).toThrow();});
 it('reads saved sources offline by document ID without requesting API writes',async()=>{vi.mocked(invoke).mockResolvedValue([{...c,reference:{...data,id:'c'.repeat(32),revision:1,updatedAt:1},snapshot:{status:'unverified'}}]);expect(await sources('d'.repeat(32))).toHaveLength(1);expect(invoke).toHaveBeenLastCalledWith('pdf_zotero_sources',{documentId:'d'.repeat(32)});vi.mocked(invoke).mockRejectedValueOnce(Error('damaged'));await expect(sources('d'.repeat(32))).rejects.toThrow('damaged');});
});
