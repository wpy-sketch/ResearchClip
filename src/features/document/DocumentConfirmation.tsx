import {useDocumentStore} from './store';
import {Modal} from '../../shared/ui/Modal';
export function DocumentConfirmation(){
  const s=useDocumentStore(),busy=s.phase!=='idle';
  if(!s.pending)return null;
  return <Modal title="保存 Markdown 修改" onClose={()=>{if(!busy)void s.resolvePending('cancel');}}><div className="p-5"><p>当前文档存在未保存修改。请选择保存、放弃或取消。</p>{s.error&&<p role="alert" className="error-banner">{s.error}</p>}<div className="mt-5 flex flex-wrap justify-end gap-2"><button className="secondary-button" disabled={busy} onClick={()=>void s.resolvePending('cancel')}>取消</button><button className="secondary-button text-error" disabled={busy} onClick={()=>void s.resolvePending('discard')}>放弃修改</button><button className="capture-button document-save-action" disabled={busy} onClick={()=>void s.resolvePending('save')}>保存后继续</button></div></div></Modal>;
}
