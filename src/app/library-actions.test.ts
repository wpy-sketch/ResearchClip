import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useEditorStore } from '../features/editor/store';
import { useCaptureStore } from '../features/capture/store';
import { useUiStore } from './ui-store';
import {useCardStore} from '../features/notes/store';
import {restoreEditorFromHistory, deleteLibraryScreenshot, openLibraryEditor, saveCaptureToLibrary, saveEditorToLibrary } from './library-actions';
const revisions=vi.hoisted(()=>({saveRevision:vi.fn(),restoreRevision:vi.fn()}));vi.mock('../features/editor/revisions-api',()=>revisions);
const api=vi.hoisted(()=>({createScreenshot:vi.fn(),deleteScreenshot:vi.fn(),updateScreenshot:vi.fn(),getScreenshot:vi.fn(),screenshotImage:vi.fn(),listScreenshots:vi.fn(),libraryStatus:vi.fn()}));vi.mock('../shared/api/library',()=>api);vi.mock('@tauri-apps/api/core',()=>({isTauri:()=>false}));
const id='a'.repeat(32),blob=new Blob(['original']),doc={schemaVersion:1 as const,image:{widthPx:32,heightPx:24},objects:[]};
const item={id,title:'保存图片',widthPx:32,heightPx:24,revision:1,annotationData:doc};
beforeEach(()=>{vi.clearAllMocks();useCardStore.setState(useCardStore.getInitialState(),true);useEditorStore.setState(useEditorStore.getInitialState(),true);useCaptureStore.setState(useCaptureStore.getInitialState(),true);useUiStore.setState(useUiStore.getInitialState(),true);vi.spyOn(URL,'createObjectURL').mockReturnValue('blob:controlled');vi.spyOn(URL,'revokeObjectURL').mockImplementation(()=>{});});
describe('library and editor save workflow',()=>{
  it('refuses to delete a card with an unsaved note without calling the backend',async()=>{
    useCardStore.setState({sessions:{[id]:{id,card:null,draft:null,version:1,savedVersion:0,status:'error',error:'保存失败'}}});
    await expect(deleteLibraryScreenshot(id,1)).rejects.toThrow('未保存笔记');expect(api.deleteScreenshot).not.toHaveBeenCalled();expect(useCardStore.getState().sessions[id].version).toBe(1);
  });
  it('releases deletion lock on backend failure and never clears the cached card',async()=>{
    useCardStore.setState({sessions:{[id]:{id,card:null,draft:null,version:0,savedVersion:0,status:'error',error:'读取失败'}}});api.deleteScreenshot.mockRejectedValueOnce(new Error('删除失败'));
    await expect(deleteLibraryScreenshot(id,1)).rejects.toThrow('删除失败');expect(useCardStore.getState().sessions[id]).toBeDefined();expect(useCardStore.getState().deletingIds).toEqual([]);
  });
  it('confirmed deletion clears stale saved bindings while retaining original preview and unsaved vectors',async()=>{
    const s=useEditorStore.getState();s.open({id:'capture',name:'A',widthPx:32,heightPx:24,blob,libraryId:id,libraryRevision:1});s.add({id:'text',type:'text',x:1,y:2,color:'#ff0000',strokeWidth:2,text:'草稿',fontSize:12});const before=s.serialize('capture');useCaptureStore.setState({preview:{url:'blob:preview',displayId:'screen',captureId:'capture',widthPx:32,heightPx:24,blob,library:{id,revision:1,title:'A'}}});api.deleteScreenshot.mockResolvedValue({deleted:true,pendingCleanup:1});await deleteLibraryScreenshot(id,1);expect(useCaptureStore.getState().preview?.library).toBeUndefined();expect(useCaptureStore.getState().preview?.blob).toBe(blob);expect(useEditorStore.getState().sessions.capture.image.libraryId).toBeUndefined();expect(s.serialize('capture')).toEqual(before);expect(URL.revokeObjectURL).not.toHaveBeenCalled();
  });
  it('creates once, then updates annotations with the saved revision and immutable source',async()=>{
    const s=useEditorStore.getState();s.open({id:'capture',name:'A',widthPx:32,heightPx:24,blob});api.createScreenshot.mockResolvedValue(item);await saveEditorToLibrary(useEditorStore.getState().sessions.capture);expect(useEditorStore.getState().sessions.capture.image.libraryId).toBe(id);expect(api.createScreenshot).toHaveBeenCalledWith(blob,'A',doc);
    revisions.saveRevision.mockResolvedValue({...item,revision:2});await saveEditorToLibrary(useEditorStore.getState().sessions.capture);expect(api.createScreenshot).toHaveBeenCalledTimes(1);expect(revisions.saveRevision).toHaveBeenCalledWith(id,1,doc,'保存标注');expect(useEditorStore.getState().sessions.capture.image.blob).toBe(blob);expect(useEditorStore.getState().sessions.capture.history.past).toEqual([]);
  });
  it('revision conflict leaves unsaved vectors and original session intact',async()=>{
    const s=useEditorStore.getState();s.open({id:'capture',name:'A',widthPx:32,heightPx:24,blob,libraryId:id,libraryRevision:1});s.add({id:'text',type:'text',x:2,y:3,color:'#ff0000',strokeWidth:3,text:'待保存',fontSize:12});const before=s.serialize('capture');revisions.saveRevision.mockRejectedValue({code:'REVISION_CONFLICT'});await expect(saveEditorToLibrary(useEditorStore.getState().sessions.capture)).rejects.toMatchObject({code:'REVISION_CONFLICT'});expect(s.serialize('capture')).toEqual(before);expect(useEditorStore.getState().sessions.capture.image.libraryRevision).toBe(1);
  });
  it('reopens persisted annotation data and keeps a live session without clobbering drafts',async()=>{
    api.getScreenshot.mockResolvedValue(item);api.screenshotImage.mockResolvedValue(blob);await openLibraryEditor(id);expect(useEditorStore.getState().activeId).toBe(`library-${id}`);expect(useUiStore.getState().activeView).toBe('editor');useEditorStore.getState().add({id:'text',type:'text',x:2,y:3,color:'#ff0000',strokeWidth:3,text:'草稿',fontSize:12});await openLibraryEditor(id);expect(api.getScreenshot).toHaveBeenCalledTimes(1);expect(useEditorStore.getState().serialize(`library-${id}`).objects).toHaveLength(1);
  });
  it('serializes one capture save, retains preview on failure, and avoids duplicate saved capture records',async()=>{
    const preview={captureId:'capture',displayId:'display',url:'blob:preview',widthPx:32,heightPx:24,blob};useCaptureStore.setState({preview});api.createScreenshot.mockRejectedValueOnce({message:'磁盘写入失败'});await saveCaptureToLibrary();expect(useCaptureStore.getState().preview).toBe(preview);expect(useCaptureStore.getState().error).toContain('磁盘');expect(useCaptureStore.getState().phase).toBe('idle');api.createScreenshot.mockResolvedValue(item);await Promise.all([saveCaptureToLibrary(),saveCaptureToLibrary()]);expect(api.createScreenshot).toHaveBeenCalledTimes(2);expect(useCaptureStore.getState().preview?.library?.id).toBe(id);await saveCaptureToLibrary();expect(api.createScreenshot).toHaveBeenCalledTimes(2);
  });
});

it('restoring a saved capture synchronizes preview CAS and preserves drafts edited while waiting',async()=>{
 const s=useEditorStore.getState();s.open({id:'capture',name:'A',widthPx:32,heightPx:24,blob,libraryId:id,libraryRevision:1,annotationVersion:1});
 useCaptureStore.setState({preview:{captureId:'capture',displayId:'display',url:'blob:preview',widthPx:32,heightPx:24,blob,library:{id,revision:1,title:'A'}}});
 revisions.restoreRevision.mockResolvedValueOnce({...item,revision:2,annotationVersion:2});await restoreEditorFromHistory(useEditorStore.getState().sessions.capture,'b'.repeat(32),'恢复');expect(useCaptureStore.getState().preview?.library?.revision).toBe(2);expect(useEditorStore.getState().sessions.capture.image.annotationVersion).toBe(2);
 let finish!:(value:unknown)=>void;revisions.restoreRevision.mockReturnValueOnce(new Promise(resolve=>{finish=resolve;}));const task=restoreEditorFromHistory(useEditorStore.getState().sessions.capture,'b'.repeat(32),'恢复');s.add({id:'new',type:'text',text:'等待期间编辑',fontSize:12,x:1,y:2,color:'#123456',strokeWidth:2});finish({...item,revision:3,annotationVersion:3});await expect(task).rejects.toThrow('草稿已保留');expect(s.serialize('capture').objects[0].id).toBe('new');expect(useEditorStore.getState().sessions.capture.image.libraryRevision).toBe(3);
});
