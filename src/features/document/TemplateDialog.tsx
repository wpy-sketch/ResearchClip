import {useEffect,useState} from 'react';
import {Modal} from '../../shared/ui/Modal';
import {useDocumentApi} from './store';
import {builtInTemplates,templateMarkdown,type TemplateInfo} from './templates';
import {listTemplates,loadTemplate,saveTemplate,renameTemplate,deleteTemplate} from './template-api';
import {errorMessage} from '../../shared/api/errors';
import {normalizeMarkdownName} from './file-name';
export function TemplateDialog({mode}:{mode:'choose'|'save'}){
  const documentApi=useDocumentApi();
  const [custom,setCustom]=useState<TemplateInfo[]>([]),[selected,setSelected]=useState('builtin:0'),[name,setName]=useState(mode==='save'?'我的模板':'untitled'),[rename,setRename]=useState(''),[busy,setBusy]=useState(true),[error,setError]=useState<string|null>(null),[deleting,setDeleting]=useState(false);
  const close=()=>documentApi.getState().setTemplateDialog(null),item=custom.find(t=>t.id===selected);
  useEffect(()=>{let active=true;listTemplates().then(rows=>{if(active)setCustom(rows);}).catch(e=>{if(active)setError(errorMessage(e,'无法读取模板。'));}).finally(()=>{if(active)setBusy(false);});return()=>{active=false;};},[]);
  const task=async(action:()=>Promise<void>)=>{if(busy)return;setBusy(true);setError(null);try{await action();}catch(e){setError(errorMessage(e,'模板操作失败；当前文档未改动。'));}finally{setBusy(false);}};
  return <Modal fitViewport title={mode==='choose'?'新建模板文档':'保存为模板'} onClose={()=>{if(!busy)close();}}><form className="template-form p-5" onSubmit={event=>{
    event.preventDefault();void task(async()=>{
      const normalized=normalizeMarkdownName(name),display=normalized.slice(0,-3);if(!display.trim()||display.length>80)throw new Error('模板或文档名称最多80个字符。');
      if(mode==='save'){
        const s=documentApi.getState();if(!s.document||s.phase!=='idle')throw new Error('文档正在处理，请稍后重试。');
        const markdown=templateMarkdown(s.document),refs=new Set([...markdown.matchAll(/\]\(assets\/(researchclip-[a-f0-9]{32}\.png)\)/g)].map(m=>m[1]));
        if([...refs].some(n=>!s.assets[n]))throw new Error('文档图片缺失，未保存模板。');
        await saveTemplate({name:display,markdown,assets:Object.fromEntries(Object.entries(s.assets).filter(([n])=>refs.has(n)).map(([n,a])=>[n,a.blob]))});documentApi.setState({notice:'自定义模板已保存；当前 Markdown 文件保持不变。'});close();
      }else{
        const seed=item?await loadTemplate(item):builtInTemplates[Number(selected.split(':')[1])];if(!seed)throw new Error('模板已不存在，请重新选择。');
        await documentApi.getState().createFromTemplate({...seed,name:display});close();
      }
    });
  }}>
    {mode==='choose'&&<label className="block text-sm">文档模板<select data-initial-focus aria-label="文档模板" disabled={busy} value={selected} className="mt-2 w-full rounded border border-edge bg-surface p-3" onChange={e=>{const value=e.target.value;setSelected(value);setDeleting(false);const entry=custom.find(t=>t.id===value);setName(entry?.name??(value==='builtin:0'?'untitled':builtInTemplates[Number(value.split(':')[1])].name));setRename(entry?.name??'');}}><optgroup label="内置模板">{builtInTemplates.map((t,i)=><option value={`builtin:${i}`} key={t.name}>{t.name}</option>)}</optgroup><optgroup label="自定义模板">{custom.map(t=><option value={t.id} key={t.id}>{t.name}</option>)}</optgroup></select></label>}
    <label className="mt-3 block text-sm">{mode==='save'?'模板名称':'新文档名称'}<input data-initial-focus={mode==='save'||undefined} aria-label={mode==='save'?'模板名称':'新文档名称'} className="mt-2 w-full rounded border border-edge bg-surface p-3" disabled={busy} maxLength={80} value={name} onChange={e=>setName(e.target.value)}/></label>
    {mode==='choose'&&item&&<fieldset className="mt-4 rounded border border-edge p-3"><legend className="text-xs">管理选中的自定义模板</legend><input aria-label="模板新名称" disabled={busy} value={rename} maxLength={80} className="w-full rounded border border-edge bg-surface p-2" onChange={e=>setRename(e.target.value)}/><div className="mt-2 flex flex-wrap gap-2"><button type="button" className="secondary-button" disabled={busy||!rename.trim()} onClick={()=>void task(async()=>{await renameTemplate(item.id,rename);setCustom(await listTemplates());})}>重命名模板</button><button type="button" className="secondary-button text-error" disabled={busy} onClick={()=>{if(!deleting){setDeleting(true);return;}void task(async()=>{await deleteTemplate(item.id);setCustom(await listTemplates());setSelected('builtin:0');setName('untitled');setDeleting(false);});}}>{deleting?'确认删除模板':'删除模板'}</button></div>{deleting&&<p className="mt-2 text-xs text-muted">从列表移除，文件保留在 templates-v1/deleted-* 中供手动恢复；不删除原文档。</p>}</fieldset>}
    <p className="my-3 text-xs text-muted">{mode==='save'?'将当前正文及受管图片复制为独立模板；不会保存或修改当前 Markdown 文件。同名模板不会覆盖。':'创建独立的普通 Markdown 草稿，模板保持不变。自定义模板仅在桌面应用中持久保存。'}</p>
    {busy&&<p role="status">正在处理模板…</p>}{error&&<p role="alert" className="error-banner">{error}</p>}
    <div className="mt-4 flex justify-end gap-2"><button type="button" className="secondary-button" disabled={busy} onClick={close}>取消</button><button type="submit" className="capture-button" disabled={busy||!name.trim()}>{mode==='save'?'保存模板':'创建文档'}</button></div>
  </form></Modal>;
}
