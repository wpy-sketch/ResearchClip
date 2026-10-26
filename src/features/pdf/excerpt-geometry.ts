export interface PageRectangle {x:number;y:number;width:number;height:number}
export interface PdfGeometry {view:readonly number[];userUnit:number;transform:readonly number[];width:number;height:number}
const finite=(v:number)=>Number.isFinite(v);
export function selectionRectangle(g:PdfGeometry,selection:PageRectangle):PageRectangle {
 const [a,b,c,d,e,f]=g.transform,[x0,y0,x1,y1]=g.view,unit=g.userUnit;
 if(g.transform.length!==6||g.view.length!==4||![...g.transform,...g.view,unit,g.width,g.height,...Object.values(selection)].every(finite)||unit<=0||g.width<=0||g.height<=0||x1<=x0||y1<=y0||selection.x<0||selection.y<0||selection.width<=0||selection.height<=0||selection.x+selection.width>1.00000001||selection.y+selection.height>1.00000001)throw Error('PDF选区或页面变换无效。');
 const det=a*d-b*c;if(!finite(det)||Math.abs(det)<1e-12)throw Error('PDF页面变换不可逆。');
 const points=[[selection.x*g.width,selection.y*g.height],[(selection.x+selection.width)*g.width,(selection.y+selection.height)*g.height]].map(([px,py])=>{const x=(d*(px-e)-c*(py-f))/det,y=(-b*(px-e)+a*(py-f))/det;return [(x-x0)*unit,(y1-y)*unit];});
 const w=(x1-x0)*unit,h=(y1-y0)*unit,x=Math.max(0,Math.min(...points.map(p=>p[0]))),y=Math.max(0,Math.min(...points.map(p=>p[1]))),right=Math.min(w,Math.max(...points.map(p=>p[0]))),bottom=Math.min(h,Math.max(...points.map(p=>p[1])));
 if(right<=x||bottom<=y)throw Error('请选择页面内非空区域。');
 return {x,y,width:right-x,height:bottom-y};
}
/** Canonical unrotated top-left points to normalized displayed page, including intrinsic rotation. */
export function displayRectangle(r:PageRectangle,w:number,h:number,rotation:number):PageRectangle {
 const points=[[r.x,r.y],[r.x+r.width,r.y+r.height]].map(([x,y])=>{switch(rotation){case 0:return [x/w,y/h];case 90:return [1-y/h,x/w];case 180:return [1-x/w,1-y/h];case 270:return [y/h,1-x/w];default:throw Error('页面旋转无效。');}});
 return {x:Math.min(...points.map(p=>p[0])),y:Math.min(...points.map(p=>p[1])),width:Math.abs(points[1][0]-points[0][0]),height:Math.abs(points[1][1]-points[0][1])};
}
export function cropDimensions(rect:PageRectangle,dpi:number,rotation:number){
 if(![144,216,288].includes(dpi)||![0,90,180,270].includes(rotation)||![rect.x,rect.y,rect.width,rect.height].every(finite)||rect.x<0||rect.y<0||rect.width<=0||rect.height<=0)throw Error('摘录清晰度或区域无效。');
 const scale=dpi/72,w=rotation%180?rect.height:rect.width,h=rotation%180?rect.width:rect.height;
 const width=Math.ceil(w*scale),height=Math.ceil(h*scale);
 if(width<1||height<1||width>8192||height>8192||width*height>16_777_216)throw Error('摘录超过16Mi像素或8192边长，请降低DPI或缩小选区。');
 return {width,height};
}
export function pdfLocationId(url:string){return /^researchclip:\/\/pdf-location\/([a-f0-9]{32})$/.exec(url)?.[1]??null;}
