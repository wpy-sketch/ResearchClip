import type {Viewport} from '../editor/types';
export interface AnnotationAnchor {anchorId:string;screenshotId:string;imageRevision:string;x:number;y:number;width:number;height:number;title:string;description:string;revision:number}
export type AnchorInput=Omit<AnnotationAnchor,'anchorId'|'revision'>&{anchorId:string|null;revision:number|null};
export interface AnchorPage {imageRevision:string;widthPx:number;heightPx:number;anchors:AnnotationAnchor[]}
export interface AnchorCanvasState {anchors:AnnotationAnchor[];visible:boolean;highlight:string|null;mode:'point'|'region'|null;disabled:boolean;onCreate:(region:{x:number;y:number;width:number;height:number})=>void;onMove:(anchor:AnnotationAnchor,x:number,y:number)=>void}
export function validRegion(a:Pick<AnnotationAnchor,'x'|'y'|'width'|'height'>,width:number,height:number){return [a.x,a.y,a.width,a.height].every(n=>Number.isFinite(n)&&n>=0)&&a.x<width&&a.y<height&&a.x+a.width<=width&&a.y+a.height<=height&&((a.width===0&&a.height===0)||(a.width>0&&a.height>0));}
export function anchorViewport(a:Pick<AnnotationAnchor,'x'|'y'|'width'|'height'>,host:{width:number;height:number}):Viewport{
 if(![host.width,host.height].every(n=>Number.isFinite(n)&&n>0)||![a.x,a.y,a.width,a.height].every(n=>Number.isFinite(n)&&n>=0))throw Error('锚点视口无效。');
 const scale=Math.min(8,Math.max(1/33554432,Math.min(host.width/Math.max(64,a.width+48),host.height/Math.max(64,a.height+48))));
 return {scale,x:host.width/2-(a.x+a.width/2)*scale,y:host.height/2-(a.y+a.height/2)*scale,mode:'manual'};
}
