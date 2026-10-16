import {useCardStore} from '../features/notes/store';
import {useUiStore} from './ui-store';
export function openKnowledgeCard(id:string){
  void useCardStore.getState().open(id);
  if(useCardStore.getState().activeId===id)useUiStore.getState().navigate('card');
}
