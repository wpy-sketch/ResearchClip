import {describe,it,expect} from 'vitest';
import {addExportSelection,toggleExportSelection} from './selection';
describe('bounded cross-page card selection',()=>{
  it('merges pages without duplicates and removes only the toggled card',()=>{const first=['a','b'],both=addExportSelection(first,['b','c']);expect(both).toEqual(['a','b','c']);expect(first).toEqual(['a','b']);expect(toggleExportSelection(both,'b')).toEqual(['a','c']);});
  it('refuses to exceed the limit without changing the previous selection',()=>{const ids=Array.from({length:100},(_,n)=>String(n));expect(()=>toggleExportSelection(ids,'new')).toThrow('100');expect(ids).toHaveLength(100);expect(toggleExportSelection(ids,'0')).toHaveLength(99);});
});
