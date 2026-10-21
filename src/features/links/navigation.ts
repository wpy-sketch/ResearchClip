import {useDocumentStore} from '../document/store';
import {useCardStore} from '../notes/store';
import {useUiStore} from '../../app/ui-store';
import {internalTarget,targetUrl,type Target} from './model';
import {resolveAnchor} from '../anchors/api';
import {openLibraryEditor} from '../../app/library-actions';
import {useEditorStore} from '../editor/store';
let navigationSerial=0;
export async function openInternalTarget(target:Target,offset=0){
 const serial=++navigationSerial;
 if(!internalTarget(targetUrl(target)))throw Error('内部引用目标无效。');
 if(target.anchorId){
  const anchor=await resolveAnchor(target.id,target.anchorId);if(serial!==navigationSerial)return;
  // Verify even a retained editor session; an old cached original must not silently relocate.
  await openLibraryEditor(target.id,()=>serial===navigationSerial);if(serial!==navigationSerial)return;
  const s=useEditorStore.getState(),session=s.sessions[s.activeId??''];if(!session||session.image.libraryId!==target.id)throw Error('截图编辑器未能打开。');
  const digest=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',await session.image.blob.arrayBuffer())),b=>b.toString(16).padStart(2,'0')).join('');
  if(serial!==navigationSerial)return;if(digest!==anchor.imageRevision)throw Error('当前编辑器原图版本不匹配，未跳转；请保留草稿后关闭并重新打开图片。');
  s.focusAnchor(anchor);return;
 }
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
