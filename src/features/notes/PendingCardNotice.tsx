import {useCardStore,isDirty} from './store';
import {openKnowledgeCard} from '../../app/card-actions';
export function PendingCardNotice(){
  const sessions=useCardStore(s=>s.sessions),error=useCardStore(s=>s.error);
  const pending=Object.values(sessions).filter(isDirty);
  if(!pending.length&&!error)return null;
  return <div className="card-pending-notice" aria-label="待保存卡片">
    <span>{error??`${pending.length} 张卡片${pending.some(s=>s.error)?'保存失败，草稿已保留':'待保存'}。退出前请确认保存完成。`}</span>
    {pending.map(s=><button key={s.id} onClick={()=>openKnowledgeCard(s.id)} title={s.draft?.title}>{s.draft?.title||'未命名卡片'}</button>)}
    <button onClick={()=>void useCardStore.getState().flushAll()}>保存全部</button>
  </div>;
}
