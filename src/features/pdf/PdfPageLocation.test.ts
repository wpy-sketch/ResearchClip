import {describe,it,expect} from 'vitest';
import {initialPdfLocation,validPdfLocation,pdfScale,normalizedOffset} from './PdfPageLocation';
describe('PDF location and zoom policy',()=>{
 it('validates persisted positions without silently accepting future shape/NaN/invalid pages',()=>{const v=initialPdfLocation();expect(validPdfLocation(v)).toBe(true);for(const patch of [{pageIndex:-1},{pageIndex:1.5},{pageIndex:10000},{rotation:45},{zoom:NaN},{zoom:0},{offsetY:1.01},{zoomMode:'future'},{spread:1}])expect(validPdfLocation({...v,...patch})).toBe(false);});
 it('keeps normalized reading offsets bounded across resize and empty extents',()=>{expect(normalizedOffset(30,120)).toBe(.25);expect(normalizedOffset(-1,120)).toBe(0);expect(normalizedOffset(200,120)).toBe(1);expect(normalizedOffset(10,0)).toBe(0);});
 it('fits single/double pages and rotated dimensions without changing source geometry',()=>{const v=initialPdfLocation();expect(pdfScale(v,640,840,600,800)).toBe(1);expect(pdfScale({...v,spread:true},1256,840,600,800)).toBe(1);expect(pdfScale({...v,zoomMode:'page'},840,640,800,600)).toBe(1);expect(pdfScale({...v,zoomMode:'page'},640,432,600,800)).toBe(.5);expect(pdfScale({...v,zoomMode:'custom',zoom:3},10,10,600,800)).toBe(3);expect(pdfScale(v,0,0,10000,10000)).toBe(.1);});
});
