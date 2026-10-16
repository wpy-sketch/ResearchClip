import { useEditorStore } from '../features/editor/store';
import type { CapturePreview } from '../shared/types/capture';
import { useUiStore } from './ui-store';
export function openCaptureEditor(preview: CapturePreview) {
  if (!preview.blob || !preview.captureId) throw new Error('截图资源不可用，请重新截图后打开编辑器。');
  useEditorStore.getState().open({ id: preview.captureId, name: preview.library?.title??`${preview.mode === 'region' ? '区域' : '全屏'} ${preview.widthPx}×${preview.heightPx}`, widthPx: preview.widthPx, heightPx: preview.heightPx, blob: preview.blob, ...(preview.library?{libraryId:preview.library.id,libraryRevision:preview.library.revision}:{}) });
  if (useEditorStore.getState().activeId !== preview.captureId) throw new Error(useEditorStore.getState().error ?? '编辑器无法打开。');
  useUiStore.getState().navigate('editor');
}
