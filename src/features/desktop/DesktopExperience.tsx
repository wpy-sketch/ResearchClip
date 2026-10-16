import {useDesktopStore} from './store';
import {Modal} from '../../shared/ui/Modal';
export function DesktopExperience(){
  const {phase,error,finishQuit}=useDesktopStore();
  return <>
    {error&&<div className="error-banner" role="alert">{error}</div>}
    {(phase==='checking'||phase==='closing')&&<p className="px-5 py-2 text-xs text-muted" role="status">{phase==='checking'?'正在保存笔记并准备退出…':'正在释放系统资源…'}</p>}
    {phase==='confirm'&&<Modal title="退出 ResearchClip" onClose={()=>void finishQuit(false)}><div className="p-5"><p className="text-sm leading-relaxed">笔记已保存。仍有尚未保存的图片标注或正在绘制的对象；退出后这些编辑会丢失。请先保存到素材库或另存可编辑项目。</p><div className="mt-5 flex flex-wrap justify-end gap-2"><button className="secondary-button" onClick={()=>void finishQuit(false)}>返回保存</button><button className="secondary-button text-error" onClick={()=>void finishQuit(true)}>放弃未保存标注并退出</button></div></div></Modal>}
  </>;
}
