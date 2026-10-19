import {beforeEach,expect,it,vi} from 'vitest';
import {parseRevision,revisionDocument,revisionHistory,restoreRevision,saveRevision} from './revisions-api';
import {useEditorStore} from './store';
const ipc=vi.hoisted(()=>vi.fn());vi.mock('@tauri-apps/api/core',()=>({invoke:ipc,isTauri:()=>true}));
const id='a'.repeat(32),rid='b'.repeat(32),meta={revisionId:rid,screenshotId:id,versionNumber:1,parentRevisionId:null,createdAt:10,description:'初始',schemaVersion:1};
const doc={schemaVersion:1 as const,image:{widthPx:32,heightPx:24},objects:[]};
const image={id:'image',name:'原图',blob:new Blob(['immutable']),widthPx:32,heightPx:24,libraryId:id,libraryRevision:1,annotationVersion:1};
beforeEach(()=>{vi.clearAllMocks();useEditorStore.setState(useEditorStore.getInitialState(),true);vi.spyOn(URL,'createObjectURL').mockReturnValue('blob:own');vi.spyOn(URL,'revokeObjectURL').mockImplementation(()=>{});});
it('validates version metadata, page order, screenshot association and unsupported documents',async()=>{
 expect(parseRevision(meta)).toEqual(meta);expect(()=>parseRevision({...meta,versionNumber:0})).toThrow();
 ipc.mockResolvedValueOnce([meta]);expect(await revisionHistory(id)).toEqual([meta]);
 ipc.mockResolvedValueOnce([meta,{...meta,versionNumber:2}]);await expect(revisionHistory(id)).rejects.toThrow('顺序');
 ipc.mockResolvedValueOnce({revision:meta,annotationJson:{...doc,schemaVersion:99}});await expect(revisionDocument(id,rid)).rejects.toThrow('版本');
 ipc.mockResolvedValueOnce({revision:{...meta,screenshotId:'c'.repeat(32)},annotationJson:doc});await expect(revisionDocument(id,rid)).rejects.toThrow('不匹配');
});
it('preview loads editable data without changing draft, viewport, history, saved marker or source',async()=>{
 const store=useEditorStore.getState();store.open(image);store.add({id:'rectangle',type:'rectangle',x:2,y:3,width:5,height:6,color:'#abcdef',strokeWidth:2});store.setViewport({x:5,y:-8,scale:2,mode:'manual'});const before=useEditorStore.getState().sessions.image;
 ipc.mockResolvedValueOnce({revision:meta,annotationJson:doc});expect((await revisionDocument(id,rid)).document).toEqual(doc);expect(useEditorStore.getState().sessions.image).toBe(before);
});
it('restore creates new editing base and permits further edits and independent undo',()=>{
 const s=useEditorStore.getState();s.open(image);s.add({id:'draft',type:'number',x:3,y:4,value:1,size:20,color:'#123456',strokeWidth:2});const previous=s.serialize('image');s.setViewport({x:-50,y:40,scale:.5,mode:'manual'});
 s.restoreRevision('image',doc,3,3);let current=useEditorStore.getState().sessions.image;expect(current.image.annotationVersion).toBe(3);expect(current.savedDocument).toBe(JSON.stringify(doc));expect(current.image.blob).toBe(image.blob);expect(current.viewport.scale).toBe(.5);
 s.undo();expect(s.serialize('image')).toEqual(previous);expect(useEditorStore.getState().sessions.image.image.annotationVersion).toBe(3);s.redo();s.add({...previous.objects[0],id:'continued'});current=useEditorStore.getState().sessions.image;expect(current.document.objects[0].id).toBe('continued');expect(current.savedDocument).toBe(JSON.stringify(doc));
});
it('save and restore failures preserve the draft and never mutate original resources',async()=>{
 const s=useEditorStore.getState();s.open(image);const before=useEditorStore.getState().sessions.image;ipc.mockRejectedValueOnce(new Error('事务失败'));await expect(saveRevision(id,1,doc,'说明')).rejects.toThrow('事务失败');expect(useEditorStore.getState().sessions.image).toBe(before);
 ipc.mockRejectedValueOnce(new Error('版本损坏'));await expect(restoreRevision(id,1,rid,'恢复')).rejects.toThrow('损坏');expect(useEditorStore.getState().sessions.image).toBe(before);expect(URL.revokeObjectURL).not.toHaveBeenCalled();
 expect(()=>s.restoreRevision('image',{...doc,image:{widthPx:33,heightPx:24}},3,3)).toThrow('尺寸');expect(s.serialize('image')).toEqual(doc);
});
