import {GlobalWorkerOptions} from 'pdfjs-dist';
import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url';
GlobalWorkerOptions.workerSrc=workerUrl;
// No viewer scripting manager, QuickJS, XFA execution, annotation actions or remote assets.
export {getDocument,PDFWorker,TextLayer,PasswordResponses,PermissionFlag} from 'pdfjs-dist';
export type {PDFDocumentProxy,PDFPageProxy,PDFDocumentLoadingTask,RenderTask} from 'pdfjs-dist';
export const pdfEngineOptions={
 cMapUrl:'/pdfjs/cmaps/',cMapPacked:true,standardFontDataUrl:'/pdfjs/standard_fonts/',
 iccUrl:'/pdfjs/iccs/',wasmUrl:'/pdfjs/wasm/',useSystemFonts:true,enableXfa:false,
 maxImageSize:32*1024*1024,canvasMaxAreaInBytes:16*1024*1024,
};
