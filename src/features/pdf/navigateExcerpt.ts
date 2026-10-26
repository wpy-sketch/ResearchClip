import {CommandFailure} from '../commands/platform/errors';
import {sourceOffset} from './source-model';
import {invoke} from '@tauri-apps/api/core';
import {openPdf,pdfSessions} from './PdfViewerSession';
import type {PageRectangle} from './excerpt-geometry';
import {workspaceManager} from '../workspace/manager';
export interface PdfLocationTarget {documentId:string;location:{attachmentId:string;contentHash:string;pageIndex:number;pageWidth:number;pageHeight:number;rectangles:PageRectangle[]}}
export function parseLocationTarget(value:unknown):PdfLocationTarget {
 const t=value as PdfLocationTarget|null,l=t?.location,id=(v:unknown)=>typeof v==='string'&&/^[a-f0-9]{32}$/.test(v);
 if(!t||!id(t.documentId)||!l||!id(l.attachmentId)||typeof l.contentHash!=='string'||!/^[a-f0-9]{64}$/.test(l.contentHash)||!Number.isInteger(l.pageIndex)||l.pageIndex<0||l.pageIndex>=2000||![l.pageWidth,l.pageHeight].every(n=>Number.isFinite(n)&&n>0&&n<=20000)||!Array.isArray(l.rectangles)||l.rectangles.length<1||l.rectangles.length>100||l.rectangles.some(r=>!r||![r.x,r.y,r.width,r.height].every(Number.isFinite)||r.x<0||r.y<0||r.width<=0||r.height<=0||r.x+r.width>l.pageWidth+.00001||r.y+r.height>l.pageHeight+.00001))throw Error('PDF来源位置无效，未打开其他文件。');
 return t;
}
export async function navigateExcerpt(locationId:string,select=false){
 let value:unknown;try{value=await invoke('resolve_pdf_location',{locationId});}catch{throw new CommandFailure('PDF_SOURCE_MISMATCH');}const target=parseLocationTarget(value),l=target.location;
 await openPdf(target.documentId);
 const session=[...pdfSessions.values()].find(s=>s.state.getState().record?.documentId===target.documentId),s=session?.state.getState(),shape=s?.pages[l.pageIndex];
 if(!session||s?.phase!=='ready'||!shape)throw Error('PDF尚未准备好，请在阅读器确认后重试。');
 if(s.record?.contentHash!==l.contentHash||s.record?.attachmentId!==l.attachmentId)throw new CommandFailure('PDF_SOURCE_MISMATCH');
 const width=shape.rotation%180?shape.height:shape.width,height=shape.rotation%180?shape.width:shape.height;
 if(Math.abs(width-l.pageWidth)>.01||Math.abs(height-l.pageHeight)>.01)throw new CommandFailure('PDF_SOURCE_MISMATCH');
 if(select&&!s.copyAllowed)throw Error('此PDF权限限制内容摘录。');
 session.setExcerptMode(select);session.state.setState({excerptSelection:select?{pageIndex:l.pageIndex,rectangle:l.rectangles[0]}:null});
 const data=value as {title?:unknown;zotero?:unknown[]},zotero=Array.isArray(data.zotero)?data.zotero.flatMap(v=>{if(!v||typeof v!=='object')return [];const z=v as Record<string,unknown>;return typeof z.itemKey==='string'&&typeof z.attachmentKey==='string'&&Number.isSafeInteger(z.libraryId)?[`${z.libraryType==='group'?'群组':'个人'}库 ${z.libraryId} · item ${z.itemKey} · 附件 ${z.attachmentKey}`]:[];}):[];session.highlightSource({pageIndex:l.pageIndex,rectangle:l.rectangles[0]},{title:typeof data.title==='string'?data.title:'PDF来源',pageIndex:l.pageIndex,zotero});
 const offset=sourceOffset(l.rectangles[0],width,height,(shape.rotation+s.location.rotation)%360);session.update({pageIndex:l.pageIndex,...offset},true);workspaceManager.focus(session.id);
}
