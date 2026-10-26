export interface PdfPageLocation {pageIndex:number;rotation:number;zoom:number;zoomMode:'custom'|'width'|'page';spread:boolean;offsetX:number;offsetY:number}
export const initialPdfLocation=():PdfPageLocation=>({pageIndex:0,rotation:0,zoom:1,zoomMode:'width',spread:false,offsetX:0,offsetY:0});
export function validPdfLocation(value:unknown):value is PdfPageLocation{
 if(!value||typeof value!=='object')return false;const v=value as PdfPageLocation;
 return Number.isInteger(v.pageIndex)&&v.pageIndex>=0&&v.pageIndex<10_000&&[0,90,180,270].includes(v.rotation)&&Number.isFinite(v.zoom)&&v.zoom>=.25&&v.zoom<=4&&['custom','width','page'].includes(v.zoomMode)&&typeof v.spread==='boolean'&&[v.offsetX,v.offsetY].every(n=>Number.isFinite(n)&&n>=0&&n<=1);
}
export function normalizedOffset(offset:number,size:number){return size>0?Math.max(0,Math.min(1,offset/size)):0;}
export function pdfScale(location:PdfPageLocation,width:number,height:number,pageWidth:number,pageHeight:number){
 if(location.zoomMode==='custom')return location.zoom;
 const columnWidth=Math.max(1,(width-40-(location.spread?16:0))/(location.spread?2:1));
 const fitWidth=columnWidth/pageWidth;
 return Math.max(.000001,Math.min(4,location.zoomMode==='page'?Math.min(fitWidth,Math.max(1,height-32)/pageHeight):fitWidth));
}
export function pdfZoomLabel(scale:number){return scale*100<1?'<1%':`${Math.round(scale*100)}%`;}
export function visiblePdfPage(pages:{index:number;top:number}[],scrollTop:number,currentIndex:number):number|null{
 let rowTop=-Infinity;let row:{index:number;top:number}[]=[];
 for(const page of pages){if(page.top>scrollTop+32)break;if(page.top>rowTop){rowTop=page.top;row=[page];}else if(page.top===rowTop)row.push(page);}
 return row.find(p=>p.index===currentIndex)?.index??row[0]?.index??pages[0]?.index??null;
}
