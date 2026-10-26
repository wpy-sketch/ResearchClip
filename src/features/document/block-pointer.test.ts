import {it,expect} from 'vitest';
import {pointerBlockTarget} from './block-pointer';
it('selects insertion boundaries in the visible document without changing block identities',()=>{const rows=[{index:0,top:-100,bottom:20},{index:1,top:40,bottom:300},{index:2,top:320,bottom:600}];expect(pointerBlockTarget(rows,45,3)).toBe(1);expect(pointerBlockTarget(rows,350,3)).toBe(2);expect(pointerBlockTarget(rows,700,3)).toBe(3);expect(pointerBlockTarget([],0,0)).toBe(0);expect(()=>pointerBlockTarget(rows,NaN,3)).toThrow();expect(()=>pointerBlockTarget([{index:4,top:0,bottom:10}],2,3)).toThrow();});
