import {useState,type DragEvent} from 'react';
import {useImportStore} from './store';
export function useImageDrop(target:'library'|'markdown'){
  const [over,setOver]=useState(false);
  const fileDrag=(e:DragEvent)=>Array.from(e.dataTransfer.types).includes('Files');
  return {over,handlers:{
    onDragOver:(e:DragEvent)=>{if(!fileDrag(e))return;e.preventDefault();e.stopPropagation();e.dataTransfer.dropEffect=useImportStore.getState().busy?'none':'copy';setOver(true);},
    onDragLeave:(e:DragEvent)=>{if(!(e.relatedTarget instanceof Node)||!e.currentTarget.contains(e.relatedTarget))setOver(false);},
    onDrop:(e:DragEvent)=>{if(!fileDrag(e))return;e.preventDefault();e.stopPropagation();setOver(false);void useImportStore.getState().run(Array.from(e.dataTransfer.files),target);},
  }};
}
