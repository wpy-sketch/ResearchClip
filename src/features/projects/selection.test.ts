import {beforeEach,describe,it,expect} from 'vitest';
import {chooseItems,useSelectionStore} from './selection';
const rows=Array.from({length:6},(_,i)=>({id:String(i+1).padStart(32,'0'),revision:1}));
beforeEach(()=>useSelectionStore.getState().clear());
describe('library multiple selection',()=>{
 it('toggles Ctrl-style selection without replacing existing items',()=>{const s=useSelectionStore.getState();s.choose(rows,rows[0].id);s.choose(rows,rows[3].id);expect(useSelectionStore.getState().items).toEqual([rows[0],rows[3]]);s.choose(rows,rows[0].id);expect(useSelectionStore.getState().items).toEqual([rows[3]]);});
 it('extends Shift range in either direction and keeps image IDs paired with revisions',()=>{expect(chooseItems([rows[5]],rows,rows[3].id,rows[1].id,true)).toEqual([rows[5],rows[1],rows[2],rows[3]]);expect(chooseItems([],rows,rows[1].id,rows[4].id,true)).toEqual(rows.slice(1,5));});
 it('uses visible-page range and merges cross-page/all-result IDs without duplication',()=>{const s=useSelectionStore.getState();s.choose(rows.slice(0,3),rows[0].id);s.choose(rows.slice(3),rows[4].id,true);expect(useSelectionStore.getState().items).toEqual([rows[0],rows[4]]);s.add(rows);expect(useSelectionStore.getState().items).toHaveLength(6);expect(new Set(useSelectionStore.getState().items.map(i=>i.id)).size).toBe(6);});
 it('strips thumbnail/path metadata from manual selection snapshots',()=>{const rich={...rows[0],title:'图片',originalImagePath:'originals/a.png'};expect(chooseItems([],[rich],rich.id,null,false)).toEqual([rows[0]]);useSelectionStore.getState().add([rich]);expect(Object.keys(useSelectionStore.getState().items[0])).toEqual(['id','revision']);});
 it('rejects over-limit select-all atomically and clears anchor with the selection',()=>{useSelectionStore.getState().add([rows[0]]);expect(()=>useSelectionStore.getState().add(Array.from({length:5001},(_,i)=>({id:i.toString(16).padStart(32,'0'),revision:1})))).toThrow('5000');expect(useSelectionStore.getState().items).toEqual([rows[0]]);useSelectionStore.getState().clear();expect(useSelectionStore.getState().anchor).toBeNull();});
});
