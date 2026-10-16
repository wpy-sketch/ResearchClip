import {useState} from 'react';
import {Search,X} from 'lucide-react';
import {useUiStore} from '../../app/ui-store';
import {useDebouncedSearch} from './use-search';
export function QuickSearch(){
  const {searchQuery,setSearchQuery,openSearch,overlay}=useUiStore();
  const [composing,setComposing]=useState(false);useDebouncedSearch(overlay!=='search',composing);
  return <div className="toolbar-quick-search"><Search size={15}/><input aria-label="快捷搜索" placeholder="搜索标题或笔记…" value={searchQuery} maxLength={200}
    onCompositionStart={()=>setComposing(true)} onCompositionEnd={()=>setComposing(false)} onChange={e=>setSearchQuery(e.target.value)}
    onKeyDown={e=>{if(e.key==='Enter'&&!composing){e.preventDefault();openSearch();}}}/>
    {searchQuery&&<button className="icon-button" aria-label="清空快捷搜索" onClick={()=>setSearchQuery('')}><X size={12}/></button>}
    <button className="icon-button" aria-label="搜索素材" title="搜索素材（Ctrl+K / ⌘K）" onClick={openSearch}><kbd>⌃ K</kbd></button>
  </div>;
}
