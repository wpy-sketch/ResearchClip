import {describe,it,expect} from 'vitest';
import {comparisonExtent,fitComparison,dividerPercent} from './model';
import {imageToScreen,screenToImage,zoomViewport} from '../editor/geometry';
describe('independent comparison geometry',()=>{
  it('aligns differently sized images at their original-pixel origin without stretching',()=>{expect(comparisonExtent({widthPx:400,heightPx:200},{widthPx:300,heightPx:600})).toEqual({width:400,height:600});const v=fitComparison(400,600,500,400);expect(v.scale).toBeCloseTo(352/600);expect(imageToScreen({x:0,y:0},v)).toEqual({x:v.x,y:v.y});});
  it('supports an empty viewport and a large union domain without allocating a merged bitmap',()=>{expect(comparisonExtent(null,null)).toEqual({width:1,height:1});const extent=comparisonExtent({widthPx:16777216,heightPx:1},{widthPx:1,heightPx:16777216});expect(fitComparison(extent.width,extent.height,600,400).scale).toBeGreaterThan(0);expect(()=>comparisonExtent({widthPx:0,heightPx:1},null)).toThrow();});
  it('reuses one invertible transform for both images, keeps zoom anchor stable and clamps divider',()=>{const v=fitComparison(640,480,700,500),anchor={x:200,y:100},before=screenToImage(anchor,v),next=zoomViewport(v,v.scale*2,anchor);expect(screenToImage(anchor,next)).toEqual(before);expect(dividerPercent(150,100,200)).toBe(25);expect(dividerPercent(-10,100,200)).toBe(0);expect(dividerPercent(999,100,200)).toBe(100);expect(()=>dividerPercent(0,0,0)).toThrow();});
});
