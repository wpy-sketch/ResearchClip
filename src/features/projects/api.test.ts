import {describe,it,expect} from 'vitest';
import {parseProjects,parseProjectDocument,parseSelection} from './api';
const id='a'.repeat(32);
describe('project and bulk IPC contracts',()=>{
 it('validates real project counts and rejects malformed identifiers/counts',()=>{expect(parseProjects([{id,name:'研究',imageCount:12,documentCount:3}])[0].documentCount).toBe(3);for(const value of [null,[{id:'bad',name:'P',imageCount:0,documentCount:0}],[{id,name:'P',imageCount:-1,documentCount:0}]])expect(()=>parseProjects(value)).toThrow();});
 it('accepts a missing external document without hiding its record, rejects unsafe DTOs',()=>{expect(parseProjectDocument({id,name:'论文.md',projectId:null,updatedAt:1,available:false}).available).toBe(false);expect(()=>parseProjectDocument({id,name:'a',projectId:'bad',updatedAt:1,available:true})).toThrow();});
 it('selection includes each optimistic revision, supports empty data and refuses duplicates',()=>{expect(parseSelection([])).toEqual([]);expect(parseSelection([{id,revision:2}])).toEqual([{id,revision:2}]);expect(()=>parseSelection([{id,revision:1},{id,revision:2}])).toThrow();expect(()=>parseSelection([{id,revision:0}])).toThrow();});
});
