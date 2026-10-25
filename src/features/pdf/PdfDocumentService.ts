import {invoke} from '@tauri-apps/api/core';
import type {PdfPageLocation} from './PdfPageLocation';
export interface PdfRecord {documentId:string;attachmentId:string;title:string;contentHash:string;byteSize:number;reading:{revision:number;location:PdfPageLocation}|null}
export const pdfDocuments={
 adoptSource:(from:string,to:string)=>invoke<void>('adopt_pdf_source',{from,to}),
 open:(token:string,documentId?:string,requestId?:string)=>invoke<PdfRecord|null>('open_pdf_document',{token,documentId:documentId??null,requestId:requestId??null}),
 progress:(token:string)=>invoke<{loaded:number;total:number}>('pdf_load_progress',{token}),
 close:(token:string)=>invoke<void>('close_pdf_document',{token}),
 read:async(token:string,offset:number,length:number)=>new Uint8Array(await invoke<ArrayBuffer>('read_pdf_chunk',{token,offset,length})),
 recent:()=>invoke<PdfRecord[]>('recent_pdf_documents'),
 save:(record:PdfRecord,expected:number,location:PdfPageLocation)=>invoke<number>('save_pdf_reading',{documentId:record.documentId,contentHash:record.contentHash,expected,location}),
};
