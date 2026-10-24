import {runUserCommand} from '../commands/platform/runtime';
import {useProjectStore} from '../projects/store';
import {useDocumentApi,useDocumentStore} from './store';
export function InsertButton({id,editor=false,disabled=false}:{id?:string;editor?:boolean;disabled?:boolean}){
  const documentApi=useDocumentApi();
  const open=useDocumentStore(s=>Boolean(s.document)),busy=useDocumentStore(s=>s.phase!=='idle');
  if(!open)return null;
  return <button className="secondary-button markdown-insert" disabled={disabled||busy} onClick={()=>{const s=documentApi.getState(),resource=s.token?useProjectStore.getState().byToken[s.token]:null;if(!editor&&id&&resource){runUserCommand('markdown.insertImage',{document:{kind:'document',id:resource.id},image:{kind:'screenshot',id}});return;}void (editor?s.insertEditor():id?s.addImages([id]):Promise.resolve(false));}}>插入到 Markdown</button>;
}
