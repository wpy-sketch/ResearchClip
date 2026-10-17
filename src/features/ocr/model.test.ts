import {describe,it,expect} from 'vitest';
import {regionFromDrag,parseStatus,parseResult,validateText,MAX_OCR_TEXT_BYTES} from './model';
describe('OCR contracts and image pixel selection',()=>{
  it('converts scaled/reverse drags to original pixels independently from editor pan and DPI',()=>{
    for(const scale of [0.25,0.5,1,1.25,2])expect(regionFromDrag({x:100*scale,y:80*scale},{x:20*scale,y:10*scale},{width:200*scale,height:100*scale},{widthPx:200,heightPx:100})).toEqual({x:20,y:10,width:80,height:70});
  });
  it('clamps to bounds, rejects zero area and invalid dimensions, includes partial pixels',()=>{
    expect(regionFromDrag({x:-10,y:-20},{x:300,y:200},{width:200,height:100},{widthPx:400,heightPx:200})).toEqual({x:0,y:0,width:400,height:200});
    expect(regionFromDrag({x:1,y:2},{x:1,y:8},{width:200,height:100},{widthPx:400,heightPx:200})).toBeNull();
    expect(regionFromDrag({x:NaN,y:2},{x:3,y:8},{width:200,height:100},{widthPx:400,heightPx:200})).toBeNull();
    expect(regionFromDrag({x:1.1,y:2.1},{x:3.1,y:4.1},{width:100,height:100},{widthPx:100,heightPx:100})).toEqual({x:1,y:2,width:3,height:3});
  });
  it('validates installed languages and bounded text/size results including empty success',()=>{
    expect(parseStatus({languages:[],maxDimension:10000}).languages).toEqual([]);
    for(const v of [null,{}, {languages:[{tag:'../path',name:'bad'}],maxDimension:10000}])expect(()=>parseStatus(v)).toThrow();
    expect(parseResult({text:'中文 English ±\n第二行',language:'zh-Hans-CN',widthPx:200,heightPx:100}).text).toContain('\n');
    expect(parseResult({text:'',language:'en-US',widthPx:1,heightPx:1}).text).toBe('');
    expect(()=>parseResult({text:'text',language:'en-US',widthPx:0,heightPx:1})).toThrow();
    expect(()=>validateText('a'.repeat(MAX_OCR_TEXT_BYTES+1))).toThrow();expect(()=>validateText('\0')).toThrow();
  });
});
