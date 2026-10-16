import { beforeEach, describe, expect, it, vi } from 'vitest';
import { openCaptureEditor } from './open-editor';
import { useEditorStore } from '../features/editor/store';
import { useCaptureStore } from '../features/capture/store';
import { useUiStore } from './ui-store';
import { discardCapture } from '../shared/api/capture';
import type { CapturePreview } from '../shared/types/capture';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: () => false }));
vi.mock('../shared/api/capture', async (original) => ({ ...await original<typeof import('../shared/api/capture')>(), discardCapture: vi.fn() }));
const preview: CapturePreview = { captureId: 'capture-one', displayId: 'monitor-one', url: 'blob:preview', widthPx: 800, heightPx: 600, mode: 'region', blob: new Blob(['original-image']) };
beforeEach(() => {
  vi.restoreAllMocks(); vi.mocked(discardCapture).mockReset().mockResolvedValue();
  useEditorStore.setState(useEditorStore.getInitialState(),true);useCaptureStore.setState(useCaptureStore.getInitialState(),true);useUiStore.setState(useUiStore.getInitialState(),true);
  vi.spyOn(URL,'createObjectURL').mockReturnValue('blob:editor-owned');vi.spyOn(URL,'revokeObjectURL').mockImplementation(()=>{});
});
describe('capture to editor handoff',()=>{
  it('clearing the capture preview keeps the editor image, independent URL and editable pixel document',()=>{
    useCaptureStore.setState({preview});openCaptureEditor(preview);const editor=useEditorStore.getState().sessions['capture-one'];
    expect(useUiStore.getState().activeView).toBe('editor');expect(editor.image.blob).toBe(preview.blob);expect(editor.image.url).not.toBe(preview.url);
    useCaptureStore.getState().clearPreview();expect(useCaptureStore.getState().preview).toBeNull();expect(discardCapture).toHaveBeenCalledWith('capture-one');expect(URL.revokeObjectURL).toHaveBeenCalledExactlyOnceWith('blob:preview');
    expect(useEditorStore.getState().sessions['capture-one'].image).toBe(editor.image);expect(useEditorStore.getState().serialize('capture-one')).toEqual({schemaVersion:1,image:{widthPx:800,heightPx:600},objects:[]});
  });
  it('reopening the same capture preserves its viewport and objects; invalid source leaves navigation/session intact',()=>{
    openCaptureEditor(preview);const s=useEditorStore.getState();s.add({id:'box',type:'rectangle',x:10,y:20,width:30,height:40,color:'#112233',strokeWidth:2});s.setViewport({x:-10,y:50,scale:2,mode:'manual'});const before=s.serialize('capture-one');
    useUiStore.getState().navigate('screenshots');openCaptureEditor(preview);expect(s.serialize('capture-one')).toEqual(before);expect(useEditorStore.getState().sessions['capture-one'].viewport.scale).toBe(2);expect(URL.createObjectURL).toHaveBeenCalledTimes(1);
    useUiStore.getState().navigate('screenshots');for(const invalid of [{...preview,captureId:undefined},{...preview,blob:undefined},{...preview,captureId:'bad',widthPx:0}])expect(()=>openCaptureEditor(invalid)).toThrow();
    expect(useUiStore.getState().activeView).toBe('screenshots');expect(Object.keys(useEditorStore.getState().sessions)).toEqual(['capture-one']);expect(s.serialize('capture-one')).toEqual(before);
  });
});
