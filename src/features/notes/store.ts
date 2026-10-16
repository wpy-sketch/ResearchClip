import {create} from 'zustand';
import {getKnowledgeCard,saveKnowledgeCard} from '../../shared/api/library';
import type {CardDraft,KnowledgeCard} from '../../shared/api/library';
import {errorMessage} from '../../shared/api/errors';

export const CARD_SAVE_DELAY=700;
export const MAX_CARD_SESSIONS=20;
export interface CardSession {id:string;card:KnowledgeCard|null;draft:CardDraft|null;version:number;savedVersion:number;status:'loading'|'ready'|'saving'|'error';error:string|null}
interface CardState {
  activeId:string|null;sessions:Record<string,CardSession>;error:string|null;
  deletingIds:string[];beginDelete:(id:string)=>boolean;finishDelete:(id:string,deleted:boolean)=>void;
  open:(id:string)=>Promise<void>;
  edit:(id:string,patch:Partial<CardDraft>)=>void;
  flush:(id:string)=>Promise<boolean>;
  flushAll:()=>Promise<boolean>;
  reload:(id:string)=>Promise<void>;
}
export const draftFromCard=(card:KnowledgeCard):CardDraft=>({title:card.screenshot.title,content:card.note?.content??'',sourceUrl:card.note?.sourceUrl??'',tagNames:card.tags.map(t=>t.name),isFavorite:card.screenshot.isFavorite});
export const isDirty=(session:CardSession)=>session.version!==session.savedVersion;

// Per-card timers and one in-flight write keep drafts alive across navigation.
export function createCardStore(){
  const timers=new Map<string,ReturnType<typeof setTimeout>>(),writes=new Map<string,Promise<boolean>>(),loads=new Map<string,Promise<void>>();
  const clear=(id:string)=>{clearTimeout(timers.get(id));timers.delete(id);};
  const store=create<CardState>((set,get)=>{
    const update=(id:string,patch:Partial<CardSession>)=>set(state=>state.sessions[id]?{sessions:{...state.sessions,[id]:{...state.sessions[id],...patch}}}:{});
    const schedule=(id:string)=>{clear(id);timers.set(id,setTimeout(()=>{timers.delete(id);void get().flush(id);},CARD_SAVE_DELAY));};
    const load=async(id:string)=>{
      if(loads.has(id))return loads.get(id);
      const request=(async()=>{
        try{const card=await getKnowledgeCard(id);const current=get().sessions[id];if(current&&!isDirty(current))update(id,{card,draft:draftFromCard(card),status:'ready',error:null});}
        catch(error){update(id,{status:'error',error:errorMessage(error,'卡片读取失败，已有草稿已保留。')});}
      })();loads.set(id,request);try{await request;}finally{loads.delete(id);}
    };
    return {
      activeId:null,sessions:{},error:null,deletingIds:[],
      beginDelete:(id)=>{
        const session=get().sessions[id];
        if(get().deletingIds.includes(id)||writes.has(id)||loads.has(id)||session&&(isDirty(session)||session.status==='saving'||session.status==='loading'))return false;
        clear(id);set({deletingIds:[...get().deletingIds,id]});return true;
      },
      finishDelete:(id,deleted)=>{
        const sessions={...get().sessions};if(deleted){clear(id);delete sessions[id];}
        set({sessions,deletingIds:get().deletingIds.filter(value=>value!==id),...(deleted&&get().activeId===id?{activeId:null}:{})});
      },
      open:async(id)=>{
        if(get().deletingIds.includes(id)){set({error:'该卡片正在删除，请稍后再试。'});return;}
        const previous=get().activeId;
        if(previous&&previous!==id&&get().sessions[previous]&&isDirty(get().sessions[previous]))void get().flush(previous);
        const existing=get().sessions[id];
        if(!existing){
          const sessions={...get().sessions};
          if(Object.keys(sessions).length>=MAX_CARD_SESSIONS){
            const clean=Object.values(sessions).find(s=>!isDirty(s)&&s.status!=='saving'&&s.status!=='loading'&&s.id!==previous);
            if(!clean){set({error:'已保留20张未完成卡片，请先保存待保存内容再打开更多卡片。'});return;}
            clear(clean.id);delete sessions[clean.id];
          }
          sessions[id]={id,card:null,draft:null,version:0,savedVersion:0,status:'loading',error:null};
          set({activeId:id,sessions,error:null});await load(id);
        }else{
          set({activeId:id,error:null});
          if(!isDirty(existing)&&existing.status!=='saving'){update(id,{status:'loading',error:null});await load(id);}
        }
      },
      edit:(id,patch)=>{
        if(get().deletingIds.includes(id))return;
        const session=get().sessions[id];if(!session?.draft)return;
        const draft={...session.draft,...patch};if(JSON.stringify(draft)===JSON.stringify(session.draft))return;
        update(id,{draft,version:session.version+1,error:null,status:session.status==='saving'?'saving':'ready'});schedule(id);
      },
      flush:async(id)=>{
        clear(id);
        if(writes.has(id)){await writes.get(id);const current=get().sessions[id];return current?.error?false:current&&isDirty(current)?get().flush(id):true;}
        const session=get().sessions[id];if(!session?.card||!session.draft)return false;if(!isDirty(session))return true;
        const version=session.version,card=session.card,draft=session.draft;
        update(id,{status:'saving',error:null});
        const request=(async()=>{
          try{
            const saved=await saveKnowledgeCard(card,draft),current=get().sessions[id];
            if(!current)return true;
            update(id,{card:saved,savedVersion:version,status:'ready',error:null,...(current.version===version?{draft:draftFromCard(saved)}:{})});
            if(current.version!==version)schedule(id);
            return true;
          }catch(error){clear(id);update(id,{status:'error',error:errorMessage(error,'保存失败，当前卡片草稿已保留；可点击保存重试。')});return false;}
        })();writes.set(id,request);
        let success:boolean;
        try{success=await request;}finally{writes.delete(id);}
        const current=get().sessions[id];return success&&current&&isDirty(current)?get().flush(id):success;
      },
      flushAll:async()=>{const results=await Promise.all(Object.values(get().sessions).filter(isDirty).map(s=>get().flush(s.id)));return results.every(Boolean);},
      reload:async(id)=>{
        if(get().deletingIds.includes(id))return;
        clear(id);if(writes.has(id))await get().flush(id);if(!get().sessions[id])return;update(id,{status:'loading',error:null});
        try{const card=await getKnowledgeCard(id);update(id,{card,draft:draftFromCard(card),version:0,savedVersion:0,status:'ready',error:null});}
        catch(error){update(id,{status:'error',error:errorMessage(error,'重新加载失败，原有草稿仍保留。')});}
      },
    };
  });
  return {store,dispose:()=>{for(const id of timers.keys())clear(id);}};
}
export const useCardStore=createCardStore().store;
