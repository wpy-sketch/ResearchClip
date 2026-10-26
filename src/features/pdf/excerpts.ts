import {invoke} from '@tauri-apps/api/core';
import type {PdfViewerSession} from './PdfViewerSession';
import type {RenderTask} from './engine';
import {pdfCanvasCache} from './PdfCanvasCache';
import {cropDimensions,selectionRectangle,type PageRectangle} from './excerpt-geometry';
export interface ExcerptSelection {pageIndex:number;rectangle:PageRectangle}
export interface PdfExcerpt {
 screenshotId:string;literatureId:string|null;pdfAttachmentId:string;pdfContentHash:string;pageIndex:number;pageRectangle:PageRectangle;
 imageAssetPath:string;createdAt:number;locationId:string;documentId:string;dpi:number;rotation:number;
}
const id=(v:unknown):v is string=>typeof v==='string'&&/^[a-f0-9]{32}$/.test(v);
export function parseExcerpt(value:unknown):PdfExcerpt {
 const v=value as Partial<PdfExcerpt>|null,r=v?.pageRectangle;
 if(!v||!id(v.screenshotId)||!id(v.pdfAttachmentId)||!id(v.locationId)||!id(v.documentId)||!(v.literatureId===null||id(v.literatureId))||typeof v.pdfContentHash!=='string'||!/^[a-f0-9]{64}$/.test(v.pdfContentHash)||!Number.isInteger(v.pageIndex)||v.pageIndex!<0||v.pageIndex!>=2000||v.imageAssetPath!==`originals/${v.screenshotId}.png`||!Number.isSafeInteger(v.createdAt)||v.createdAt!<0||!r||![r.x,r.y,r.width,r.height].every(Number.isFinite))throw Error('PDF摘录响应无效。');
 cropDimensions(r,v.dpi!,v.rotation!);return v as PdfExcerpt;
}
export async function readExcerpts(documentId:string){const value=await invoke<unknown>('pdf_excerpts',{documentId});if(!Array.isArray(value)||value.length>100)throw Error('PDF摘录列表无效。');return value.map(parseExcerpt);}
export async function normalizeSelection(session:PdfViewerSession,pageIndex:number,selection:PageRectangle):Promise<ExcerptSelection> {
 const s=session.state.getState();if(s.phase!=='ready'||!s.copyAllowed||!session.document||!Number.isInteger(pageIndex)||pageIndex<0||pageIndex>=s.pages.length)throw Error('当前页面不可摘录。');
 const page=await session.document.getPage(pageIndex+1),viewport=page.getViewport({scale:1,rotation:(page.rotate+s.location.rotation)%360});
 return {pageIndex,rectangle:selectionRectangle({view:page.view,userUnit:page.userUnit,transform:viewport.transform,width:viewport.width,height:viewport.height},selection)};
}
export async function captureExcerpt(session:PdfViewerSession,selection:ExcerptSelection,dpi:number,literatureId:string|null,signal:AbortSignal,onCommit:()=>void):Promise<PdfExcerpt> {
 const s=session.state.getState(),record=s.record;
 if(s.phase!=='ready'||!record||!s.copyAllowed||!session.document||!Number.isInteger(selection.pageIndex)||selection.pageIndex<0||selection.pageIndex>=s.pages.length||!(literatureId===null||id(literatureId)))throw Error('PDF会话或摘录来源无效。');
 signal.throwIfAborted();const canvas=document.createElement('canvas'),release=session.holdPage(selection.pageIndex);let task:RenderTask|undefined,closed=false;
 const cancel=()=>{closed=true;task?.cancel();};const unbind=session.onDispose(cancel);signal.addEventListener('abort',cancel,{once:true});
 try{
  const data=await session.render(async()=>{
   signal.throwIfAborted();if(closed||!session.document)throw Error('PDF已关闭，摘录未保存。');
   const page=await session.document.getPage(selection.pageIndex+1),rotation=(page.rotate+s.location.rotation)%360,base=page.getViewport({scale:1,rotation:0}),r=selection.rectangle;
   if(r.x<0||r.y<0||r.x+r.width>base.width+.00001||r.y+r.height>base.height+.00001)throw Error('选区已超出PDF页面。');
   cropDimensions(r,dpi,rotation);
   const viewport=page.getViewport({scale:dpi/72,rotation}),unit=page.userUnit,[x0,, ,y1]=page.view;
   const bounds=[...viewport.convertToViewportPoint(x0+r.x/unit,y1-r.y/unit),...viewport.convertToViewportPoint(x0+(r.x+r.width)/unit,y1-(r.y+r.height)/unit)];
   const left=Math.floor(Math.min(bounds[0],bounds[2])),top=Math.floor(Math.min(bounds[1],bounds[3])),width=Math.ceil(Math.max(bounds[0],bounds[2]))-left,height=Math.ceil(Math.max(bounds[1],bounds[3]))-top;
   if(width>8192||height>8192||width*height>16_777_216)throw Error('裁切取整后超过图片预算，请缩小选区。');
   pdfCanvasCache.reserveExcerpt(canvas,session.id,width*height);canvas.width=width;canvas.height=height;
   signal.throwIfAborted();if(closed)throw Error('PDF已关闭。');task=page.render({canvas,viewport,transform:[1,0,0,1,-left,-top],annotationMode:1});await task.promise;
   signal.throwIfAborted();if(closed)throw Error('PDF已关闭，摘录未保存。');
   const blob=await new Promise<Blob>((resolve,reject)=>canvas.toBlob(b=>b?resolve(b):reject(Error('摘录图片编码失败。')),'image/png'));
   return {blob,rotation,pageWidth:base.width,pageHeight:base.height};
  });
  signal.throwIfAborted();if(closed||!data)throw Error('PDF已关闭，摘录未保存。');
  const draft={location:{attachmentId:record.attachmentId,contentHash:record.contentHash,pageIndex:selection.pageIndex,pageLabel:null,pageWidth:data.pageWidth,pageHeight:data.pageHeight,rectangles:[selection.rectangle],textAnchor:null},literatureId,dpi,rotation:data.rotation};
  const meta=new TextEncoder().encode(JSON.stringify({token:session.token,title:`${record.title} · 第${selection.pageIndex+1}页摘录`,draft})),png=new Uint8Array(await data.blob.arrayBuffer());
  if(meta.length>16384||png.length>64*1024*1024)throw Error('摘录封包超过预算。');
  const bytes=new Uint8Array(4+meta.length+png.length);new DataView(bytes.buffer).setUint32(0,meta.length,true);bytes.set(meta,4);bytes.set(png,4+meta.length);signal.throwIfAborted();if(closed)throw Error('PDF已关闭。');onCommit();
  const result=parseExcerpt(await invoke('save_pdf_excerpt',bytes.buffer));
  if(result.pdfAttachmentId!==record.attachmentId||result.pdfContentHash!==record.contentHash||result.pageIndex!==selection.pageIndex)throw Error('摘录已提交，但来源回执不符；请刷新素材库核查，不要重复保存。');
  return result;
 }finally{task?.cancel();signal.removeEventListener('abort',cancel);unbind();canvas.width=0;canvas.height=0;pdfCanvasCache.release(canvas);release();}
}
