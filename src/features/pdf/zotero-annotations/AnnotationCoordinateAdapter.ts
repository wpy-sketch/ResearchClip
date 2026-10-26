import type {ZoteroAnnotation} from './model';
import type {PageRectangle} from '../excerpt-geometry';
export interface AnnotationViewport {width:number;height:number;convertToViewportPoint:(x:number,y:number)=>number[]}
export interface AnnotationRegion {box:PageRectangle;line:{x1:number;y1:number;x2:number;y2:number}|null}
export function pageAnnotationRects(a:ZoteroAnnotation,pageIndex:number){const p=a.position;return !p?[]:p.pageIndex===pageIndex?p.rects:p.pageIndex+1===pageIndex?p.nextPageRects:[];}
export class AnnotationCoordinateAdapter {
 static regions(a:ZoteroAnnotation,pageIndex:number,viewport:AnnotationViewport):AnnotationRegion[]{
  if(![viewport.width,viewport.height].every(Number.isFinite)||!(viewport.width>0&&viewport.height>0))throw Error('批注页面尺寸无效。');
  return pageAnnotationRects(a,pageIndex).flatMap(r=>{const left=viewport.convertToViewportPoint(r[0],r[1]),right=viewport.convertToViewportPoint(r[2],r[3]);if(left.length!==2||right.length!==2||![...left,...right].every(Number.isFinite))throw Error('批注页面变换无效。');
   const x=Math.max(0,Math.min(left[0],right[0])/viewport.width),y=Math.max(0,Math.min(left[1],right[1])/viewport.height),x2=Math.min(1,Math.max(left[0],right[0])/viewport.width),y2=Math.min(1,Math.max(left[1],right[1])/viewport.height);if(x2<=x||y2<=y)return [];
   let line:AnnotationRegion['line']=null;if(a.annotationType==='underline'){const end=viewport.convertToViewportPoint(r[2],r[1]),ax=left[0]/viewport.width,ay=left[1]/viewport.height,bx=end[0]/viewport.width,by=end[1]/viewport.height;if(ax<0&&bx<0||ax>1&&bx>1||ay<0&&by<0||ay>1&&by>1)return [];const bound=(v:number)=>Math.max(0,Math.min(1,v));line={x1:bound(ax),y1:bound(ay),x2:bound(bx),y2:bound(by)};}
   return [{box:{x,y,width:x2-x,height:y2-y},line}];
  });
 }
 static canonical(a:ZoteroAnnotation,pageIndex:number,base:AnnotationViewport):PageRectangle {const box=this.regions(a,pageIndex,base)[0]?.box;if(!box)throw Error('该批注区域位于当前页面裁切范围之外，未猜测位置。');return {x:box.x*base.width,y:box.y*base.height,width:box.width*base.width,height:box.height*base.height};}
}
