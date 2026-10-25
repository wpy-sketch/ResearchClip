export const PDF_TILE_EDGE=1024;
export const PDF_TILE_PIXELS=PDF_TILE_EDGE**2;
export interface PdfTile {key:string;x:number;y:number;width:number;height:number}
export interface PdfClip {left:number;top:number;right:number;bottom:number}
/** All tile coordinates are physical pixels; CSS page size stays independent. */
export function visiblePdfTiles(width:number,height:number,density:number,clip:PdfClip):PdfTile[]{
 if(![width,height,density,clip.left,clip.top,clip.right,clip.bottom].every(Number.isFinite)||width<=0||height<=0||density<=0||density>32||width*density>10_000_000||height*density>10_000_000)throw Error('PDF显示尺寸超出支持范围。');
 const w=Math.max(1,Math.ceil(width*density)),h=Math.max(1,Math.ceil(height*density));
 const left=Math.max(0,Math.floor(clip.left*density)),top=Math.max(0,Math.floor(clip.top*density)),right=Math.min(w,Math.ceil(clip.right*density)),bottom=Math.min(h,Math.ceil(clip.bottom*density));
 if(right<=left||bottom<=top)return [];
 const tiles:PdfTile[]=[];
 for(let y=Math.floor(top/PDF_TILE_EDGE)*PDF_TILE_EDGE;y<bottom;y+=PDF_TILE_EDGE)for(let x=Math.floor(left/PDF_TILE_EDGE)*PDF_TILE_EDGE;x<right;x+=PDF_TILE_EDGE){if(tiles.length>=64)throw Error('PDF可见图块超过保护预算，请减小窗口或关闭其他PDF。');tiles.push({key:`${x}:${y}:${density}`,x,y,width:Math.min(PDF_TILE_EDGE,w-x),height:Math.min(PDF_TILE_EDGE,h-y)});}
 return tiles;
}
