import {Download,X} from 'lucide-react';
import {isTauri} from '@tauri-apps/api/core';
import {useMarkdownExportStore} from './store';

export function MarkdownExportButton({ids,label='导出 Markdown',disabled=false}:{ids:string[];label?:string;disabled?:boolean}){
  const busy=useMarkdownExportStore(s=>s.busy);
  return <button className="secondary-button" disabled={disabled||busy||!ids.length||!isTauri()} onClick={()=>void useMarkdownExportStore.getState().run(ids)} title="先保存所选卡片的笔记，再导出原图与Markdown；不会合成图片标注。"><Download size={14}/>{label}</button>;
}
export function MarkdownExportStatus(){
  const {busy,result,error,cancelled,dismiss}=useMarkdownExportStore();
  if(!busy&&!result&&!error&&!cancelled)return null;
  const partial=Boolean(result?.failures.length||result?.warnings.length);
  return <aside className={`markdown-export-status ${error||partial?'has-warning':''}`} role={error||partial?'alert':'status'} aria-label="Markdown 导出状态">
    <div className="flex items-start justify-between gap-3"><div>
      <p>{busy?'正在导出 Markdown，请完成目录选择并等待写入…':error??(cancelled?'已取消 Markdown 导出。':`Markdown 导出完成：${result?.exportedIds.length} 张卡片${partial?'，请查看下列问题':''}。`)}</p>
      {result&&<><p className="export-directory">保存位置：{result.directory}</p><p>打开 notes.md；图片位于同目录 assets/。移动时请保留整个文件夹。</p></>}
    </div>{!busy&&<button className="icon-button" aria-label="关闭导出提示" onClick={dismiss}><X size={14}/></button>}</div>
    {partial&&<details><summary>未导出 {result?.failures.length} 张 · 提醒 {result?.warnings.length} 项</summary><ul>{[...(result?.failures??[]),...(result?.warnings??[])].map((issue,index)=><li key={`${issue.id}:${index}`}>{issue.id.slice(0,8)}：{issue.message}</li>)}</ul></details>}
  </aside>;
}
