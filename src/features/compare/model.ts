import {MAX_SCALE,MIN_SCALE,validateImageSize} from '../editor/geometry';
import type {Viewport} from '../editor/types';
export type CompareMode='side'|'slider'|'overlay';
export interface CompareImage {id:string;name:string;format:string;widthPx:number;heightPx:number;blob:Blob;url:string;broken?:boolean}
export function comparisonExtent(a:Pick<CompareImage,'widthPx'|'heightPx'>|null,b:Pick<CompareImage,'widthPx'|'heightPx'>|null){
  for(const image of [a,b])if(image)validateImageSize(image.widthPx,image.heightPx);
  return {width:Math.max(a?.widthPx??1,b?.widthPx??1),height:Math.max(a?.heightPx??1,b?.heightPx??1)};
}
// The union rectangle is a coordinate domain, never an allocated image buffer.
export function fitComparison(width:number,height:number,hostWidth:number,hostHeight:number):Viewport{
  if(![width,height,hostWidth,hostHeight].every(n=>Number.isFinite(n)&&n>0))throw new Error('对比视口尺寸无效。');
  const scale=Math.min(1,MAX_SCALE,Math.max(MIN_SCALE,Math.min((hostWidth-Math.min(48,hostWidth/2))/width,(hostHeight-Math.min(48,hostHeight/2))/height)));
  return {scale,x:(hostWidth-width*scale)/2,y:(hostHeight-height*scale)/2,mode:'fit'};
}
export function dividerPercent(clientX:number,left:number,width:number){if(![clientX,left,width].every(Number.isFinite)||width<=0)throw new Error('分割线坐标无效。');return Math.max(0,Math.min(100,(clientX-left)/width*100));}
