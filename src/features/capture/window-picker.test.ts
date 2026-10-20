import {describe,it,expect} from 'vitest';
import {pickerRect} from './window-picker';
describe('window picker physical coordinates',()=>{
  it('maps negative monitor origins without an extra DPI factor',()=>{expect(pickerRect({x:-1500,y:150,width:600,height:300},{x:-1920,y:0,width:3840,height:1080},2560,720)).toEqual({left:280,top:100,width:400,height:200});});
  it.each([1,1.25,1.5,2])('matches a %s scaled viewport',scale=>{const rect=pickerRect({x:125,y:250,width:500,height:750},{x:0,y:0,width:2000,height:1500},2000/scale,1500/scale);expect(rect.left).toBe(125/scale);expect(rect.top).toBe(250/scale);expect(rect.width).toBe(500/scale);expect(rect.height).toBe(750/scale);});
  it('rejects invalid surface dimensions',()=>{expect(()=>pickerRect({x:0,y:0,width:1,height:1},{x:0,y:0,width:0,height:1},100,100)).toThrow();});
});
