import {describe,it,expect} from 'vitest';
import {parseSavedClip,useClipperStore} from './store';
describe('web clip notification boundary',()=>{
 it('accepts only opaque document IDs and bounded text',()=>{expect(parseSavedClip({documentId:'a'.repeat(32),title:'中文 English',duplicate:false}).title).toBe('中文 English');for(const documentId of ['../secret','C:/private.md','a'])expect(()=>parseSavedClip({documentId,title:'x',duplicate:false})).toThrow();});
 it('retains result until explicit dismissal; malformed events do not replace it',()=>{const s=useClipperStore.getState();s.receive({documentId:'b'.repeat(32),title:'研究',duplicate:true});s.receive({path:'arbitrary'});expect(useClipperStore.getState().notice?.documentId).toBe('b'.repeat(32));expect(useClipperStore.getState().error).toBeTruthy();s.dismiss();expect(useClipperStore.getState().notice).toBeNull();});
});
