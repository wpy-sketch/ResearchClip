import {useEffect,useState} from 'react';
import {Tags,Trash2} from 'lucide-react';
import {isTauri} from '@tauri-apps/api/core';
import {useTagStore} from './store';
import {useCardStore} from '../notes/store';
import {saveTag,deleteTag} from '../../shared/api/library';
import type {Tag} from '../../shared/api/library';
import {useLibraryStore} from '../library/store';
import {useUiStore} from '../../app/ui-store';
import {errorMessage} from '../../shared/api/errors';
import {Modal} from '../../shared/ui/Modal';
import {EmptyState} from '../../shared/ui/EmptyState';

export function TagsView(){
  const {items,loading,error,refresh}=useTagStore();const [name,setName]=useState(''),[target,setTarget]=useState<Tag|null>(null),[working,setWorking]=useState(false),[failure,setFailure]=useState<string|null>(null);
  useEffect(()=>{void refresh();},[refresh]);
  const mutate=async(action:()=>Promise<unknown>)=>{setWorking(true);setFailure(null);try{await action();await refresh();}catch(e){setFailure(errorMessage(e,'标签操作失败，截图与笔记已保留。'));}finally{setWorking(false);}};
  return <div className="page-container"><div className="page-heading"><div><h2>标签</h2><p>按课题、方法或文献来源整理科研素材。</p></div><span className="subtle-badge">{items.length} 个标签</span></div>
    <form className="tag-create-form" onSubmit={e=>{e.preventDefault();void mutate(async()=>{await saveTag(name);setName('');});}}><input aria-label="新标签名称" placeholder="输入新标签名称" maxLength={50} value={name} onChange={e=>setName(e.target.value)} disabled={working||!isTauri()}/><button className="secondary-button" disabled={working||!isTauri()||!name.trim()}>创建标签</button></form>
    {(error||failure)&&<p role="alert" className="error-banner my-4">{failure??error}</p>}
    <section aria-label="标签管理">{loading&&<p role="status" className="text-xs text-muted">正在读取标签…</p>}{items.length?<div className="tag-management-list">{items.map(tag=><article key={tag.id}><button onClick={()=>{useUiStore.getState().navigate('screenshots');void useLibraryStore.getState().refresh(false,{tagIds:[tag.id],page:0});}}><Tags size={15}/>{tag.name}</button><button className="icon-button" aria-label={`删除标签 ${tag.name}`} disabled={working} onClick={()=>setTarget(tag)}><Trash2 size={14}/></button></article>)}</div>:!loading&&<EmptyState icon={Tags} title="让素材形成自己的脉络" description={isTauri()?'在这里创建标签，或在知识卡片中直接添加。':'请在桌面应用中管理真实标签。'}/>}
    </section>
    {target&&<Modal title="删除标签" onClose={()=>{if(!working)setTarget(null);}}><div className="p-5"><p className="text-sm">删除“{target.name}”及其所有关联。截图和笔记会保留；如有未保存卡片，将先尝试保存。</p><div className="mt-4 flex justify-end gap-2"><button className="secondary-button" disabled={working} onClick={()=>setTarget(null)}>保留标签</button><button className="secondary-button text-error" disabled={working} onClick={()=>void mutate(async()=>{if(!await useCardStore.getState().flushAll())throw new Error('有卡片保存失败，请先处理待保存内容再删除标签。');await deleteTag(target.id);setTarget(null);const filter=useLibraryStore.getState();await filter.refresh(filter.favorites,{tagIds:filter.tagIds.filter(id=>id!==target.id)});})}>确认删除标签</button></div>{failure&&<p className="error-banner mt-3" role="alert">{failure}</p>}</div></Modal>}
  </div>;
}
