import {it,expect} from 'vitest';
import {selectionRectangle,displayRectangle,cropDimensions,pdfLocationId} from './excerpt-geometry';
it('maps real viewport quarter-turn transforms with nonzero CropBox and UserUnit into canonical points',()=>{
 const view=[10,20,210,320],userUnit=2,expected={x:40,y:60,width:120,height:160};
 // Independent PDF.js-compatible matrices for scale1/UserUnit2, CropBox origin10,20.
 for(const [rotation,transform,width,height] of [[0,[2,0,0,-2,-20,640],400,600],[90,[0,2,2,0,-40,-20],600,400],[180,[-2,0,0,2,420,-40],400,600],[270,[0,-2,-2,0,640,420],600,400]] as const){
  const selected=displayRectangle(expected,400,600,rotation),actual=selectionRectangle({view,userUnit,transform,width,height},selected);
  for(const key of ['x','y','width','height'] as const)expect(actual[key]).toBeCloseTo(expected[key],9);
 }
});
it('remains invariant under viewport scale while CSS/DPR only change client bounds',()=>{const box={x:.1,y:.2,width:.3,height:.4};for(const scale of [.1,.5,1,2.5,4]){const r=selectionRectangle({view:[0,0,400,600],userUnit:1,transform:[scale,0,0,-scale,0,600*scale],width:400*scale,height:600*scale},box);expect(r.x).toBeCloseTo(40);expect(r.y).toBeCloseTo(120);expect(r.width).toBeCloseTo(120);expect(r.height).toBeCloseTo(240);}});
it('bounds output area and swaps axes for rotated crops instead of stretching low-resolution tiles',()=>{expect(cropDimensions({x:0,y:0,width:100,height:40},216,90)).toEqual({width:120,height:300});expect(cropDimensions({x:0,y:0,width:100,height:40},288,0)).toEqual({width:400,height:160});for(const rect of [{x:0,y:0,width:5000,height:5000},{x:0,y:0,width:NaN,height:1},{x:-1,y:0,width:10,height:10}])expect(()=>cropDimensions(rect,288,0)).toThrow();expect(()=>cropDimensions({x:0,y:0,width:1,height:1},600,0)).toThrow();});
it('rejects empty, outside and non-invertible selections',()=>{const g={view:[0,0,400,600],userUnit:1,transform:[1,0,0,-1,0,600],width:400,height:600};expect(()=>selectionRectangle(g,{x:0,y:0,width:0,height:.5})).toThrow();expect(()=>selectionRectangle(g,{x:.9,y:0,width:.5,height:.5})).toThrow();expect(()=>selectionRectangle({...g,transform:[0,0,0,0,0,0]},{x:0,y:0,width:.5,height:.5})).toThrow();});
it('accepts only stable PDF location links, never paths, scripts or query strings',()=>{expect(pdfLocationId('researchclip://pdf-location/'+'a'.repeat(32))).toBe('a'.repeat(32));for(const url of ['file:///C:/paper.pdf','javascript:alert(1)','researchclip://pdf-location/'+'a'.repeat(32)+'?path=x'])expect(pdfLocationId(url)).toBeNull();});
