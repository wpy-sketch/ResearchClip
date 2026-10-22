import {describe,it,expect} from 'vitest';
import {MODEL_ID,parseSemanticStatus,validateSemanticQuery,mergeSemanticStatus,type SemanticStatus} from './api';
const fixture=()=>({modelId:MODEL_ID,modelBytes:228030489,modelsPresent:false,configured:false,stats:{total:2,ready:0},job:{phase:'',kind:'',generation:0,total:0,scanned:0,computed:0,skipped:0,error:null,results:null,query:null,scope:null}});
describe('semantic IPC boundary',()=>{
 it('accepts offline unavailable state without depending on OCR or network',()=>expect(parseSemanticStatus(fixture()).modelsPresent).toBe(false));
 it('rejects unknown models, phases and malformed counts',()=>{for(const value of [null,{...fixture(),modelId:'other'},{...fixture(),modelBytes:0},{...fixture(),job:{...fixture().job,phase:'script'}},{...fixture(),stats:{total:-1,ready:0}}])expect(()=>parseSemanticStatus(value)).toThrow();});
 it('validates natural language and rejects control characters and excessive queries',()=>{for(const q of ['神经网络结构图','a natural landscape','图'.repeat(500)])expect(()=>validateSemanticQuery(q)).not.toThrow();for(const q of ['','  ','a\nfile','\u007f','图'.repeat(501)])expect(()=>validateSemanticQuery(q)).toThrow();});
 it('rejects unsafe target IDs and invalid cosine values',()=>{for(const item of [{id:'../../file',title:'x',score:.2},{id:'a'.repeat(32),title:'x',score:NaN},{id:'a'.repeat(32),title:'x',score:1.01}])expect(()=>parseSemanticStatus({...fixture(),job:{...fixture().job,results:{items:[item],stale:0,searched:1}}})).toThrow();});
});
describe('semantic asynchronous status ordering',()=>{
 const state=(generation:number,phase:SemanticStatus['job']['phase'],scanned=0):SemanticStatus=>({...fixture(),job:{...fixture().job,generation,phase,scanned}});
 it('keeps a new query or cleared index when an older poll arrives',()=>{const latest=state(4,'');expect(mergeSemanticStatus(latest,state(3,'completed'))).toBe(latest);expect(mergeSemanticStatus(state(3,'completed'),latest)).toBe(latest);});
 it('does not revive completed, failed or cancelled tasks',()=>{for(const phase of ['completed','failed','cancelled'] as const){const done=state(2,phase,6);expect(mergeSemanticStatus(done,state(2,'running',6))).toBe(done);}});
 it('preserves cancellation and completed scan progress',()=>{const cancelling=state(2,'cancelling',3);expect(mergeSemanticStatus(cancelling,state(2,'running',4))).toBe(cancelling);expect(mergeSemanticStatus(state(2,'running',4),state(2,'running',3)).job.scanned).toBe(4);expect(mergeSemanticStatus(cancelling,state(2,'cancelled',3)).job.phase).toBe('cancelled');});
 it('accepts current idle statistics and initial state without forcing an index',()=>{const idle=state(0,'');const next={...idle,stats:{total:3,ready:1}};expect(mergeSemanticStatus(null,next)).toBe(next);expect(mergeSemanticStatus(idle,next)).toBe(next);});
});
