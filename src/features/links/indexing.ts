import {documentLinkSource,replaceLinks} from './api';
import type {IndexedLink} from './model';
export async function indexDocumentLinks(id:string,signal:AbortSignal){
 const source=await documentLinkSource(id);signal.throwIfAborted();
 const links=await new Promise<IndexedLink[]>((resolve,reject)=>{
  const worker=new Worker(new URL('./index-worker.ts',import.meta.url),{type:'module'});
  const clean=()=>{worker.terminate();signal.removeEventListener('abort',abort);};
  const abort=()=>{clean();reject(new DOMException('索引已取消','AbortError'));};
  signal.addEventListener('abort',abort,{once:true});
  worker.onmessage=(e:MessageEvent<{links:IndexedLink[];error?:string}>)=>{clean();if(e.data.error)reject(Error(e.data.error));else resolve(e.data.links);};
  worker.onerror=()=>{clean();reject(Error('后台链接解析失败，原有索引保留。'));};
  worker.postMessage(source.markdown);if(signal.aborted)abort();
 });
 signal.throwIfAborted();await replaceLinks(id,source.stamp,links);window.dispatchEvent(new Event('researchclip-links-updated'));signal.throwIfAborted();
}
