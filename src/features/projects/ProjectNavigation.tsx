import {useEffect,useState} from 'react';
import {Folder,FolderOpen,ChevronDown,Layers,Inbox} from 'lucide-react';
import {useProjectStore} from './store';
import {useLibraryStore} from '../library/store';
import {useUiStore} from '../../app/ui-store';
import {useSelectionStore} from './selection';
export function ProjectNavigation(){
  const {items,error,refresh}=useProjectStore(),{projectId,unclassified}=useLibraryStore(),active=useUiStore(s=>s.activeView),[expanded,setExpanded]=useState(true);
  useEffect(()=>{const s=useLibraryStore.getState();void refresh(s.projectId,s.unclassified);},[refresh]);
  const choose=(id:string|null,none=false)=>{useSelectionStore.getState().clear();useUiStore.getState().navigate('screenshots');void useLibraryStore.getState().refresh(false,{projectId:id,unclassified:none,page:0});};
  return <div className="sidebar-projects"><button className="nav-button project-nav-heading" title="项目分类" aria-label="项目分类" aria-expanded={expanded} onClick={()=>setExpanded(!expanded)}><FolderOpen size={19}/><span className="sidebar-label">项目分类</span><ChevronDown size={13} className="sidebar-label"/></button>{expanded&&<nav className="project-nav-list" aria-label="项目分类导航">
    <select className="project-nav-compact" aria-label="侧栏项目分类" title="项目分类" value={projectId??(unclassified?'none':'all')} onChange={e=>choose(e.target.value==='none'||e.target.value==='all'?null:e.target.value,e.target.value==='none')}><option value="all">全部素材</option><option value="none">未分类</option>{items.map(p=><option key={p.id} value={p.id}>{p.name} · {p.imageCount} 图</option>)}</select>
    <button title="全部素材" aria-label="浏览全部素材" aria-current={active==='screenshots'&&!projectId&&!unclassified?'page':undefined} onClick={()=>choose(null)}><Layers size={15}/><span>全部素材</span></button>
    <button title="未分类" aria-label="浏览未分类" aria-current={active==='screenshots'&&unclassified?'page':undefined} onClick={()=>choose(null,true)}><Inbox size={15}/><span>未分类</span></button>
    {items.map(p=><button key={p.id} title={`${p.name} · ${p.imageCount} 张图片 · ${p.documentCount} 份文档`} aria-label={`项目：${p.name}`} aria-current={active==='screenshots'&&projectId===p.id?'page':undefined} onClick={()=>choose(p.id)}><Folder size={15}/><span>{p.name}</span><small>{p.imageCount}</small></button>)}
    {!items.length&&!error&&<p className="project-nav-hint">在素材库中新建科研项目</p>}
    {error&&<button title={error} onClick={()=>void refresh(projectId,unclassified)}>读取失败，重试</button>}
  </nav>}</div>;
}
