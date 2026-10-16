import {useState} from 'react';
import {Search,X} from 'lucide-react';
import {isTauri} from '@tauri-apps/api/core';
import {useUiStore} from '../../app/ui-store';
import {useLibraryStore} from './store';
import {useDebouncedSearch} from './use-search';
import {openKnowledgeCard} from '../../app/card-actions';
import {Modal} from '../../shared/ui/Modal';
export function SearchDialog(){
  const {searchQuery,setSearchQuery,closeOverlay,navigate}=useUiStore(),{items,total,loading,error}=useLibraryStore();
  const [composing,setComposing]=useState(false);useDebouncedSearch(true,composing);
  return <Modal title="搜索素材与笔记" onClose={closeOverlay}><div className="p-5"><div className="search-field"><Search size={18}/><input data-initial-focus aria-label="搜索关键词" maxLength={200} placeholder="输入标题或笔记关键词…" value={searchQuery} onChange={e=>setSearchQuery(e.target.value)} onCompositionStart={()=>setComposing(true)} onCompositionEnd={()=>setComposing(false)}/>{searchQuery&&<button className="icon-button" aria-label="清空搜索" onClick={()=>setSearchQuery('')}><X size={15}/></button>}</div>
    {!isTauri()?<p className="py-8 text-center text-xs text-muted">请在桌面应用中搜索真实素材，浏览器预览不创建示例资料。</p>:<><p className="my-4 text-xs text-muted" role="status">{loading?'正在搜索…':`找到 ${total} 张素材 · 包含当前标签和收藏条件`}</p>{error&&<p role="alert" className="error-banner">{error}</p>}<div className="search-results">{items.slice(0,8).map(item=><button key={item.id} disabled={loading} onClick={()=>openKnowledgeCard(item.id)}><span>{item.title}</span><time>{new Date(item.createdAt).toLocaleDateString('zh-CN')}</time></button>)}{!loading&&!error&&!items.length&&<p className="py-8 text-center text-xs text-muted">暂时没有匹配的素材，试试其他关键词或清除筛选。</p>}</div><button className="secondary-button mt-4" onClick={()=>navigate(useLibraryStore.getState().favorites?'favorites':'screenshots')}>查看全部搜索结果</button></>}
    <div className="mt-5 flex justify-between border-t border-edge pt-4 text-[11px] text-muted"><span>标题与笔记正文 · 标签可在素材库筛选</span><span><kbd>Esc</kbd> 关闭</span></div></div></Modal>;
}
