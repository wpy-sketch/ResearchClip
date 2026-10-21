import {useDocumentStore} from '../document/store';
import {useCardStore} from '../notes/store';
import {useUiStore} from '../../app/ui-store';
import {internalTarget,type Target} from './model';
export async function openInternalTarget(target:Target,offset=0){
 if(!internalTarget(`researchclip://${target.kind}/${target.id}`))throw Error('内部引用目标无效。');
 if(target.kind==='document'){
  // Reopen the registered saved source, including self-links, so raw UTF16
  // positions stay correct for composed documents and dirty drafts are confirmed.
  const s=useDocumentStore.getState();
  await s.openProject(target.id,offset);
  const latest=useDocumentStore.getState();if(latest.error)throw Error(latest.error);
 }else{
  await useCardStore.getState().open(target.id);
  const state=useCardStore.getState(),session=state.sessions[target.id];
  if(state.activeId!==target.id||!session?.card)throw Error(session?.error??state.error??'截图卡片不存在或无法打开。');
  useUiStore.getState().navigate('card');
 }
}
