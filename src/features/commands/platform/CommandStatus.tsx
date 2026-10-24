import {useEffect,useState,useSyncExternalStore} from 'react';
import {Activity,X} from 'lucide-react';
import {Modal} from '../../../shared/ui/Modal';
import {commandErrors,commandTasks,subscribeCommandNotices} from './runtime';
import type {CommandName} from './contracts';
import './status.css';
const labels:Record<CommandName,string>={'document.open':'打开科研资源','markdown.insertImage':'插入素材图','pdf.navigateTo':'PDF 定位','screenshot.capturePdfSelection':'截取 PDF 选区','annotation.create':'创建标准批注','codeWorkspace.open':'打开代码工程','agent.requestEdit':'请求 Agent 编辑'};
const phase={running:'正在处理',cancelling:'正在取消',cancelled:'已取消',completed:'已完成',failed:'未完成'} as const;
export function CommandStatus(){
 const errors=useSyncExternalStore(commandErrors.subscribe,commandErrors.getSnapshot),tasks=useSyncExternalStore(commandTasks.subscribe,commandTasks.getSnapshot),[open,setOpen]=useState(false),[notice,setNotice]=useState('');
 useEffect(()=>subscribeCommandNotices(setNotice),[]);
 const active=tasks.filter(t=>t.phase==='running'||t.phase==='cancelling'),latest=errors.at(-1);
 const cancel=(id:string)=>{try{commandTasks.cancel(id);}catch(e){commandErrors.report(e);}};
 return <><div className="application-command-status"><button className="secondary-button" aria-label="查看命令任务与诊断" onClick={()=>setOpen(true)}><Activity size={14}/>任务与诊断{active.length?` · ${active.length}`:''}</button>{active.map(t=><span role="status" key={t.id}>{labels[t.command]} · {t.phase==='cancelling'?'正在取消':t.cancellable?'正在检查资源':'正在完成操作'}{t.cancellable&&<button className="secondary-button" onClick={()=>cancel(t.id)}>取消任务</button>}</span>)}{notice&&!active.length&&tasks.at(-1)?.phase==='completed'&&<span role="status">{notice}<button className="icon-button" aria-label="关闭命令完成提示" onClick={()=>setNotice('')}><X size={12}/></button></span>}</div>
 {latest&&<div role="alert" className="error-banner application-command-error"><span>{latest.command?labels[latest.command]+' · 上次未完成：':''}{latest.message}</span><button className="secondary-button" onClick={()=>setOpen(true)}>查看诊断</button><button className="icon-button" aria-label="关闭命令错误提示" onClick={()=>commandErrors.dismiss(latest.id)}><X size={14}/></button></div>}
 {open&&<Modal title="命令任务与诊断" onClose={()=>setOpen(false)}><div className="command-diagnostics"><p>显示本次运行的受控资源命令。诊断不记录正文、完整文件路径或凭据。</p><h3>任务</h3>{!tasks.length&&<p className="text-muted">尚无资源命令任务。</p>}<ol>{[...tasks].reverse().map(t=><li key={t.id}><strong>{labels[t.command]}</strong><span>{phase[t.phase]}{t.phase==='running'&&!t.cancellable?' · 正在提交，请等待':''}</span>{t.cancellable&&t.phase==='running'&&<button className="secondary-button" onClick={()=>cancel(t.id)}>取消任务</button>}</li>)}</ol><h3>诊断</h3>{!errors.length&&<p className="text-muted">暂无命令错误。</p>}<ol>{[...errors].reverse().map(e=><li key={e.id}><p>{e.message}</p><small>{e.code}{e.command?' · '+e.command:''} · {new Date(e.time).toLocaleTimeString()}</small><button className="secondary-button" onClick={()=>commandErrors.dismiss(e.id)}>移除此提示</button></li>)}</ol><details><summary>模块接入状态</summary><p>Markdown 与已入库图片可通过稳定资源ID调用。PDF、标准PDF批注、代码工程和Agent运行时尚未接入；相关命令返回不可用，不执行写入。</p></details></div></Modal>}</>;
}
