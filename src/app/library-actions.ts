import { createScreenshot, deleteScreenshot, getScreenshot, screenshotImage } from '../shared/api/library';
import {saveRevision,restoreRevision} from '../features/editor/revisions-api';
import { useEditorStore } from '../features/editor/store';
import { useLibraryStore } from '../features/library/store';
import { useUiStore } from './ui-store';
import type { EditorSession } from '../features/editor/types';
import { useCaptureStore } from '../features/capture/store';
import { errorMessage } from '../shared/api/errors';
import {useCardStore} from '../features/notes/store';
export async function restoreEditorFromHistory(session:EditorSession,revisionId:string,description:string){
  const current=useEditorStore.getState().sessions[session.image.id];
  if(!current||!current.image.libraryId||JSON.stringify(current.document)!==JSON.stringify(session.document))throw Error('当前草稿已改变，请重新打开历史后恢复。');
  const item=await restoreRevision(current.image.libraryId,current.image.libraryRevision??0,revisionId,description);
  const latest=useEditorStore.getState().sessions[session.image.id];
  const changed=Boolean(latest&&JSON.stringify(latest.document)!==JSON.stringify(session.document));
  if(changed){useEditorStore.getState().bindLibrary(session.image.id,item.id,item.revision,item.title,item.annotationVersion);useEditorStore.getState().markSaved(session.image.id,item.annotationData);}
  else if(latest)useEditorStore.getState().restoreRevision(session.image.id,item.annotationData,item.revision,item.annotationVersion!);
  const preview=useCaptureStore.getState().preview;
  if(preview?.library?.id===item.id)useCaptureStore.setState({preview:{...preview,library:{id:item.id,revision:item.revision,title:item.title}}});
  await useLibraryStore.getState().refresh();
  if(changed)throw Error('旧版已保存为新版本；等待期间草稿发生变化，当前草稿已保留，请核对后继续。');
  return item;
}
export async function saveEditorToLibrary(session:EditorSession,description="保存标注"){
  const item=session.image.libraryId?await saveRevision(session.image.libraryId,session.image.libraryRevision??0,session.document,description):await createScreenshot(session.image.blob,session.image.name,session.document);
  useEditorStore.getState().bindLibrary(session.image.id,item.id,item.revision,item.title,item.annotationVersion);
  useEditorStore.getState().markSaved(session.image.id,session.document);
  const preview=useCaptureStore.getState().preview;if(preview?.captureId===session.image.id)useCaptureStore.setState({preview:{...preview,library:{id:item.id,revision:item.revision,title:item.title}}});
  await useLibraryStore.getState().refresh();return item;
}
export async function saveCaptureToLibrary(){
  const preview=useCaptureStore.getState().preview;if(!preview?.blob||!preview.captureId||preview.library||useCaptureStore.getState().phase!=='idle')return;
  useCaptureStore.setState({phase:'working',error:null,notice:null});
  try {
    const session=useEditorStore.getState().sessions[preview.captureId];
    const item=session?await saveEditorToLibrary(session):await createScreenshot(preview.blob,`${preview.mode==='scroll'?'滚动长':preview.mode==='region'?'区域':'全屏'}截图 ${new Date().toLocaleString('zh-CN')}`,{schemaVersion:1,image:{widthPx:preview.widthPx,heightPx:preview.heightPx},objects:[]});
    if(useCaptureStore.getState().preview?.captureId===preview.captureId)useCaptureStore.setState({preview:{...preview,library:{id:item.id,revision:item.revision,title:item.title}},notice:'截图已保存到本地素材库，重启后仍可打开。'});
    await useLibraryStore.getState().refresh();useCaptureStore.setState({phase:'idle'});
  }catch(error){useCaptureStore.setState({phase:'idle',error:errorMessage(error,'素材保存失败，请保留预览后重试。')});}
}
export async function openLibraryEditor(id:string,allowed:()=>boolean=()=>true){
  const sourceId=`library-${id}`,state=useEditorStore.getState();
  // Reopening a live session keeps its unsaved edits; explicit close/reopen loads fresh DB data.
  if(state.sessions[sourceId]){if(allowed()){state.activate(sourceId);useUiStore.getState().navigate('editor');}return;}
  const item=await getScreenshot(id),blob=await screenshotImage(id);
  if(!allowed())return;
  state.open({id:sourceId,name:item.title,widthPx:item.widthPx,heightPx:item.heightPx,blob,libraryId:item.id,libraryRevision:item.revision,annotationVersion:item.annotationVersion},item.annotationData);
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
