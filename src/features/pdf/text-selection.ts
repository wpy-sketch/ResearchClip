import type {PdfViewerSession} from './PdfViewerSession';
import {selectionRectangle} from './excerpt-geometry';
export async function selectedTextLocation(session:PdfViewerSession,selection:Selection|null){
 const s=session.state.getState(),record=s.record;
 if(!s.copyAllowed||s.phase!=='ready'||!record||!session.document||!selection||selection.rangeCount!==1||selection.isCollapsed)throw Error('请先在当前PDF一页内选择可复制文字。');
 const range=selection.getRangeAt(0),element=(n:Node)=>n.nodeType===Node.ELEMENT_NODE?n as Element:n.parentElement,start=element(range.startContainer)?.closest<HTMLElement>('[data-pdf-page]'),end=element(range.endContainer)?.closest<HTMLElement>('[data-pdf-page]');
 if(!start||end!==start||start.closest<HTMLElement>('[data-pdf-document-id]')?.dataset.pdfDocumentId!==record.documentId)throw Error('文字引用必须在当前PDF的同一页内。');
 const exact=selection.toString();if(!exact.trim()||Array.from(exact).length>4096)throw Error('文字引用限1–4096个字符。');
 const pageIndex=Number(start.dataset.pdfPage),page=await session.document.getPage(pageIndex+1),viewport=page.getViewport({scale:1,rotation:(page.rotate+s.location.rotation)%360}),bounds=start.getBoundingClientRect();
 if(!bounds.width||!bounds.height)throw Error('选区页面当前不可见。');
 const rects=[...range.getClientRects()].filter(r=>r.width>0&&r.height>0&&r.left>=bounds.left-1&&r.top>=bounds.top-1&&r.right<=bounds.right+1&&r.bottom<=bounds.bottom+1);
 if(!rects.length||rects.length>100)throw Error('文字选区无有效矩形或超过100段，请缩小范围。');
 const base=page.getViewport({scale:1,rotation:0}),rectangles=rects.map(r=>selectionRectangle({view:page.view,userUnit:page.userUnit,transform:viewport.transform,width:viewport.width,height:viewport.height},{x:Math.max(0,(r.left-bounds.left)/bounds.width),y:Math.max(0,(r.top-bounds.top)/bounds.height),width:Math.min(r.width/bounds.width,1),height:Math.min(r.height/bounds.height,1)}));
 return {attachmentId:record.attachmentId,contentHash:record.contentHash,pageIndex,pageLabel:null,pageWidth:base.width,pageHeight:base.height,rectangles,textAnchor:{exact,prefix:'',suffix:''}};
}
