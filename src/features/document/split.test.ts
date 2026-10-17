import {describe,it,expect} from 'vitest';
import {splitWidth,splitPercent} from './split';
import {fitViewport,imageToScreen,screenToImage} from '../editor/geometry';
describe('split layout and editor image coordinates',()=>{
  it('starts at half, enforces minima and cannot overflow a narrow workspace',()=>{
    expect(splitWidth(1008,50)).toBe(500);expect(splitWidth(1008,20)).toBe(280);expect(splitWidth(1008,80)).toBe(720);
    for(const width of [0,100,400,1008,2000])for(const percent of [-100,20,50,80,200]){const left=splitWidth(width,percent);expect(left).toBeGreaterThanOrEqual(0);expect(left).toBeLessThanOrEqual(Math.max(0,width-8));}
  });
  it('clamps divider drag to the permitted range',()=>{expect(splitPercent(1008,-1000)).toBe(28);expect(splitPercent(1008,2000)).toBe(72);expect(splitPercent(2008,1000)).toBe(50);});
  it('keeps original-pixel coordinates invertible as the left pane resizes',()=>{
    for(const width of [180,280,500,1000]){const viewport=fitViewport(320,200,width,400),point={x:123.5,y:75.25};const restored=screenToImage(imageToScreen(point,viewport),viewport);expect(restored.x).toBeCloseTo(point.x);expect(restored.y).toBeCloseTo(point.y);}
  });
});
