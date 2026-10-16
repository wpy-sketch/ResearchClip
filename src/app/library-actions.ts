import { createScreenshot, deleteScreenshot, getScreenshot, screenshotImage, updateScreenshot } from '../shared/api/library';
import { useEditorStore } from '../features/editor/store';
import { useLibraryStore } from '../features/library/store';
import { useUiStore } from './ui-store';
import type { EditorSession } from '../features/editor/types';
import { useCaptureStore } from '../features/capture/store';
import { errorMessage } from '../shared/api/errors';
import {useCardStore} from '../features/notes/store';
export async function saveEditorToLibrary(session:EditorSession){
  const item=session.image.libraryId?await updateScreenshot(session.image.libraryId,session.image.libraryRevision??0,{annotationData:session.document}):await createScreenshot(session.image.blob,session.image.name,session.document);
  useEditorStore.getState().bindLibrary(session.image.id,item.id,item.revision,item.title);
  useEditorStore.getState().markSaved(session.image.id,session.document);
  const preview=useCaptureStore.getState().preview;if(preview?.captureId===session.image.id)useCaptureStore.setState({preview:{...preview,library:{id:item.id,revision:item.revision,title:item.title}}});
  await useLibraryStore.getState().refresh();return item;
}
export async function saveCaptureToLibrary(){
  const preview=useCaptureStore.getState().preview;if(!preview?.blob||!preview.captureId||preview.library||useCaptureStore.getState().phase!=='idle')return;
  useCaptureStore.setState({phase:'working',error:null,notice:null});
  try {
    const session=useEditorStore.getState().sessions[preview.captureId];
    const item=session?await saveEditorToLibrary(session):await createScreenshot(preview.blob,`${preview.mode==='region'?'区域':'全屏'}截图 ${new Date().toLocaleString('zh-CN')}`,{schemaVersion:1,image:{widthPx:preview.widthPx,heightPx:preview.heightPx},objects:[]});
    if(useCaptureStore.getState().preview?.captureId===preview.captureId)useCaptureStore.setState({preview:{...preview,library:{id:item.id,revision:item.revision,title:item.title}},notice:'截图已保存到本地素材库，重启后仍可打开。'});
    await useLibraryStore.getState().refresh();useCaptureStore.setState({phase:'idle'});
  }catch(error){useCaptureStore.setState({phase:'idle',error:errorMessage(error,'素材保存失败，请保留预览后重试。')});}
}
export async function openLibraryEditor(id:string){
  const sourceId=`library-${id}`,state=useEditorStore.getState();
  // Reopening a live session keeps its unsaved edits; explicit close/reopen loads fresh DB data.
  if(state.sessions[sourceId]){state.activate(sourceId);useUiStore.getState().navigate('editor');return;}
  const item=await getScreenshot(id),blob=await screenshotImage(id);
  state.open({id:sourceId,name:item.title,widthPx:item.widthPx,heightPx:item.heightPx,blob,libraryId:item.id,libraryRevision:item.revision},item.annotationData);
  if(useEditorStore.getState().activeId!==sourceId)throw new Error(useEditorStore.getState().error??'无法打开素材图片。');useUiStore.getState().navigate('editor');
}
export async function deleteLibraryScreenshot(id:string,revision:number){
  const cards=useCardStore.getState();
  if(!cards.beginDelete(id))throw new Error('该卡片仍有未保存笔记或正在读取/保存。请先保存笔记并刷新素材库，再重试删除；草稿已保留。');
  let deleted=false;
  try{
    const result=await deleteScreenshot(id,revision);if(!result.deleted)throw new Error('删除未完成，资料和草稿已保留，请刷新后重试。');deleted=true;
    useEditorStore.getState().unlinkLibrary(id);
    const preview=useCaptureStore.getState().preview;if(preview?.library?.id===id){const next={...preview};delete next.library;useCaptureStore.setState({preview:next,notice:'素材记录已删除；当前预览仍保留，重新存库会创建新资料。'});}
    if(useCardStore.getState().activeId===id&&useUiStore.getState().activeView==='card')useUiStore.getState().navigate('screenshots');
    return result;
  }finally{useCardStore.getState().finishDelete(id,deleted);}
}
