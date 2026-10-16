import {useEffect} from 'react';
import {useCardStore,isDirty} from '../features/notes/store';
import {useEditorStore} from '../features/editor/store';
import {useCaptureStore} from '../features/capture/store';
import {useLibraryStore} from '../features/library/store';
import {useTagStore} from '../features/tags/store';
import {useUiStore} from './ui-store';

export function useCardWorkflow(){
  useEffect(()=>{
    const unsubscribe=useCardStore.subscribe((state,previous)=>{
      for(const session of Object.values(state.sessions)){
        const old=previous.sessions[session.id];
        // Only our successful metadata transaction is known to leave annotation JSON unchanged.
        if(!old?.card||!session.card||session.savedVersion<=old.savedVersion)continue;
        const before=old.card.screenshot,after=session.card.screenshot;
        for(const editor of Object.values(useEditorStore.getState().sessions)){
          if(editor.image.libraryId===after.id&&editor.image.libraryRevision===before.revision)useEditorStore.getState().bindLibrary(editor.image.id,after.id,after.revision,after.title);
        }
        const preview=useCaptureStore.getState().preview;
        if(preview?.library?.id===after.id&&preview.library.revision===before.revision)useCaptureStore.setState({preview:{...preview,library:{id:after.id,revision:after.revision,title:after.title}}});
        if(JSON.stringify(old.card.tags)!==JSON.stringify(session.card.tags))void useTagStore.getState().refresh();
        const ui=useUiStore.getState();if(ui.activeView==='screenshots'||ui.activeView==='favorites'||ui.overlay==='search')void useLibraryStore.getState().refresh();
      }
    });
    const beforeUnload=(event:BeforeUnloadEvent)=>{if(Object.values(useCardStore.getState().sessions).some(isDirty)){event.preventDefault();event.returnValue='';}};
    window.addEventListener('beforeunload',beforeUnload);
    return()=>{unsubscribe();window.removeEventListener('beforeunload',beforeUnload);};
  },[]);
}
