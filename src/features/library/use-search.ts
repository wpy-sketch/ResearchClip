import {useEffect} from 'react';
import {useUiStore} from '../../app/ui-store';
import {useLibraryStore} from './store';
export const SEARCH_DELAY=300;
export function useDebouncedSearch(enabled:boolean,composing:boolean){
  const query=useUiStore(s=>s.searchQuery),activeView=useUiStore(s=>s.activeView),applied=useLibraryStore(s=>s.query);
  useEffect(()=>{
    const trimmed=query.trim();if(!enabled||composing||trimmed===applied)return;
    const timer=setTimeout(()=>{
      void useLibraryStore.getState().refresh(activeView==='favorites',{query:trimmed,page:0});
      if(useUiStore.getState().overlay!=='search'&&activeView!=='screenshots'&&activeView!=='favorites'&&document.activeElement?.getAttribute('aria-label')==='快捷搜索')useUiStore.getState().navigate('screenshots');
    },SEARCH_DELAY);
    return()=>clearTimeout(timer);
  },[query,applied,activeView,enabled,composing]);
}
export function clearLibraryFilters(){
  useUiStore.getState().setSearchQuery('');useUiStore.getState().navigate('screenshots');
  void useLibraryStore.getState().refresh(false,{source:'all',query:'',tagIds:[],favorite:null,projectId:null,unclassified:false,sort:'createdDesc',page:0});
}
