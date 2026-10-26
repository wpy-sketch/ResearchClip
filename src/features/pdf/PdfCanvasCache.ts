import {PDF_TILE_PIXELS} from './PdfRenderGeometry';
/** Explicit RGBA canvas budget across all PDF sessions, including thumbnails. */
export class PdfCanvasCache {
 private entries=new Map<object,{owner:string;pixels:number}>();
 constructor(readonly maxPixels=72_000_000,readonly ownerPixels=24_000_000){}
 reserve(key:object,owner:string,pixels:number){
  this.reservePixels(key,owner,pixels,PDF_TILE_PIXELS);
 }
 reserveExcerpt(key:object,owner:string,pixels:number){this.reservePixels(key,owner,pixels,16_777_216);}
 private reservePixels(key:object,owner:string,pixels:number,max:number){
  if(!Number.isSafeInteger(pixels)||pixels<=0||pixels>max)throw Error('PDF页面画布尺寸超过保护预算。');
  this.release(key);let total=0,owned=0;for(const entry of this.entries.values()){total+=entry.pixels;if(entry.owner===owner)owned+=entry.pixels;}
  if(total+pixels>this.maxPixels||owned+pixels>this.ownerPixels)throw Error('PDF画布缓存达到保护上限，请减小窗口或关闭其他PDF后重试。');
  this.entries.set(key,{owner,pixels});
 }
 release(key:object){this.entries.delete(key);}
 get pixels(){return [...this.entries.values()].reduce((sum,entry)=>sum+entry.pixels,0);}
}
export const pdfCanvasCache=new PdfCanvasCache();
