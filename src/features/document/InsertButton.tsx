import {useDocumentStore} from './store';
export function InsertButton({id,editor=false,disabled=false}:{id?:string;editor?:boolean;disabled?:boolean}){
  const open=useDocumentStore(s=>Boolean(s.document)),busy=useDocumentStore(s=>s.phase!=='idle');
  if(!open)return null;
  return <button className="secondary-button markdown-insert" disabled={disabled||busy} onClick={()=>{const s=useDocumentStore.getState();void (editor?s.insertEditor():id?s.addImages([id]):Promise.resolve(false));}}>插入到 Markdown</button>;
}
