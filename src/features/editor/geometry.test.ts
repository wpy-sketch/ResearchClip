import { describe, expect, it } from 'vitest';
import { fitViewport, imageToScreen, MAX_IMAGE_PIXELS, MIN_SCALE, screenToImage, validateImageSize, zoomViewport } from './geometry';
describe('original-pixel editor viewport', () => {
  it('validates original dimensions before creating a canvas', () => { validateImageSize(2560, 1440); for (const pair of [[0,10], [NaN,10], [10.5,10], [4097,4096]]) expect(() => validateImageSize(...pair as [number,number])).toThrow(); });
  it('fits large images without changing their dimensions', () => {
    const view = fitViewport(1600, 900, 848, 498); expect(view).toEqual({ scale: 0.5, x: 24, y: 24, mode: 'fit' });
    expect(() => fitViewport(100, 100, 0, 400)).toThrow();
  });
  it.each([[16384,512,260,150],[MAX_IMAGE_PIXELS,1,1,1],[1,MAX_IMAGE_PIXELS,1,1]])('fits wide/tall valid images entirely even below 2 percent', (w,h,aw,ah) => {
    const view=fitViewport(w,h,aw,ah);expect(view.scale).toBeLessThan(0.02);expect(view.scale).toBeGreaterThanOrEqual(MIN_SCALE);
    expect(view.x).toBeGreaterThanOrEqual(0);expect(view.y).toBeGreaterThanOrEqual(0);expect(view.x+w*view.scale).toBeLessThanOrEqual(aw);expect(view.y+h*view.scale).toBeLessThanOrEqual(ah);
    const anchor={x:aw/2,y:ah/2};const zoomed=zoomViewport(view,view.scale/1.25,anchor);expect(zoomed.scale).toBeLessThanOrEqual(view.scale);expect(screenToImage(anchor,zoomed).x).toBeCloseTo(screenToImage(anchor,view).x);
  });
  it('rejects invalid coordinates in both directions and preserves sub-percent zoom round trips',()=>{
    const view={x:10,y:-20,scale:0.005,mode:'manual' as const},p={x:12000,y:300};expect(screenToImage(imageToScreen(p,view),view)).toEqual(p);
    for(const invalid of [{...view,scale:0},{...view,x:Infinity},{...view,scale:NaN}]){expect(()=>screenToImage(p,invalid)).toThrow();expect(()=>imageToScreen(p,invalid)).toThrow();}
    expect(()=>imageToScreen({x:NaN,y:1},view)).toThrow();
  });
  it.each([0.25, 0.5, 1, 1.5, 4])('round trips mouse coordinates after pan and scale %s', (scale) => {
    const view = { x: -120, y: 85, scale, mode: 'manual' as const }; const original = { x: 360, y: 128 };
    expect(screenToImage(imageToScreen(original, view), view)).toEqual(original);
  });
  it('zoom keeps the image coordinate under the pointer unchanged and clamps bounds', () => {
    const view = { x: 48, y: -80, scale: 0.5, mode: 'manual' as const }; const anchor = { x: 360, y: 240 };
    expect(screenToImage(anchor, zoomViewport(view, 1.5, anchor))).toEqual(screenToImage(anchor, view));
    expect(zoomViewport(view, 100, anchor).scale).toBe(8); expect(() => zoomViewport(view, NaN, anchor)).toThrow();
  });
});
