export interface PdfPageLocation {pageIndex:number;rotation:number;zoom:number;zoomMode:'custom'|'width'|'page';spread:boolean;offsetX:number;offsetY:number}
export const initialPdfLocation=():PdfPageLocation=>({pageIndex:0,rotation:0,zoom:1,zoomMode:'width',spread:false,offsetX:0,offsetY:0});
export function validPdfLocation(value:unknown):value is PdfPageLocation{
 if(!value||typeof value!=='object')return false;const v=value as PdfPageLocation;
 return Number.isInteger(v.pageIndex)&&v.pageIndex>=0&&v.pageIndex<10_000&&[0,90,180,270].includes(v.rotation)&&Number.isFinite(v.zoom)&&v.zoom>=.25&&v.zoom<=4&&['custom','width','page'].includes(v.zoomMode)&&typeof v.spread==='boolean'&&[v.offsetX,v.offsetY].every(n=>Number.isFinite(n)&&n>=0&&n<=1);
}
export function normalizedOffset(offset:number,size:number){return size>0?Math.max(0,Math.min(1,offset/size)):0;}
export function pdfScale(location:PdfPageLocation,width:number,height:number,pageWidth:number,pageHeight:number){
 if(location.zoomMode==='custom')return location.zoom;
 const fitWidth=Math.max(80,width-40)/(pageWidth*(location.spread?2:1)+ (location.spread?16:0));
 return Math.max(.1,Math.min(4,location.zoomMode==='page'?Math.min(fitWidth,Math.max(80,height-32)/pageHeight):fitWidth));
}
