import {useDesktopStore} from './store';
import {Modal} from '../../shared/ui/Modal';
import {useDocumentStore} from '../document/store';
export function DesktopExperience(){
  const {phase,error,finishQuit,documentQuit}=useDesktopStore();
  const documentError=useDocumentStore(s=>s.error),documentBusy=useDocumentStore(s=>s.phase!=='idle');
  return <>
    {error&&<div className="error-banner" role="alert">{error}</div>}
    {phase==='document'&&<Modal title="退出前保存 Markdown" onClose={()=>{if(!documentBusy)void documentQuit('cancel');}}><div className="p-5"><p>Markdown 文档存在未保存修改。</p>{documentError&&<p role="alert" className="error-banner">{documentError}</p>}<div className="mt-5 flex flex-wrap justify-end gap-2"><button className="secondary-button" disabled={documentBusy} onClick={()=>void documentQuit('cancel')}>取消退出</button><button className="secondary-button text-error" disabled={documentBusy} onClick={()=>void documentQuit('discard')}>放弃文档修改并继续退出</button><button className="capture-button document-save-action" disabled={documentBusy} onClick={()=>void documentQuit('save')}>保存文档并继续退出</button></div></div></Modal>}
    {(phase==='checking'||phase==='closing')&&<p className="px-5 py-2 text-xs text-muted" role="status">{phase==='checking'?'正在保存笔记并准备退出…':'正在释放系统资源…'}</p>}
    {phase==='confirm'&&<Modal title="退出 ResearchClip" onClose={()=>void finishQuit(false)}><div className="p-5"><p className="text-sm leading-relaxed">笔记已保存。仍有尚未保存的图片标注或正在绘制的对象；退出后这些编辑会丢失。请先保存到素材库或另存可编辑项目。</p><div className="mt-5 flex flex-wrap justify-end gap-2"><button className="secondary-button" onClick={()=>void finishQuit(false)}>返回保存</button><button className="secondary-button text-error" onClick={()=>void finishQuit(true)}>放弃未保存标注并退出</button></div></div></Modal>}
  </>;
}
