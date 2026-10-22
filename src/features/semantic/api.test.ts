import {describe,it,expect} from 'vitest';
import {MODEL_ID,parseSemanticStatus,validateSemanticQuery} from './api';
const fixture=()=>({modelId:MODEL_ID,modelBytes:228030489,modelsPresent:false,configured:false,stats:{total:2,ready:0},job:{phase:'',kind:'',generation:0,total:0,scanned:0,computed:0,skipped:0,error:null,results:null,query:null,scope:null}});
describe('semantic IPC boundary',()=>{
 it('accepts offline unavailable state without depending on OCR or network',()=>expect(parseSemanticStatus(fixture()).modelsPresent).toBe(false));
 it('rejects unknown models, phases and malformed counts',()=>{for(const value of [null,{...fixture(),modelId:'other'},{...fixture(),modelBytes:0},{...fixture(),job:{...fixture().job,phase:'script'}},{...fixture(),stats:{total:-1,ready:0}}])expect(()=>parseSemanticStatus(value)).toThrow();});
 it('validates natural language and rejects control characters and excessive queries',()=>{for(const q of ['神经网络结构图','a natural landscape','图'.repeat(500)])expect(()=>validateSemanticQuery(q)).not.toThrow();for(const q of ['','  ','a\nfile','\u007f','图'.repeat(501)])expect(()=>validateSemanticQuery(q)).toThrow();});
 it('rejects unsafe target IDs and invalid cosine values',()=>{for(const item of [{id:'../../file',title:'x',score:.2},{id:'a'.repeat(32),title:'x',score:NaN},{id:'a'.repeat(32),title:'x',score:1.01}])expect(()=>parseSemanticStatus({...fixture(),job:{...fixture().job,results:{items:[item],stale:0,searched:1}}})).toThrow();});
});
