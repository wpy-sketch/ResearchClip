import { invoke, isTauri } from '@tauri-apps/api/core';
import type { CapturePreview, OverlayInfo } from '../types/capture';
import { decodePreview, parseDisplays, releasePreview } from './capture';
import type { Point, Size } from '../../features/capture/geometry';

export async function loadOverlay(sessionId: string): Promise<{ info: OverlayInfo; preview: CapturePreview }> {
  if (!isTauri()) throw new Error('截图选择层只能在 Windows 桌面应用中使用。');
  const value = await invoke<unknown>('region_overlay_info', { sessionId });
  if (!value || typeof value !== 'object' || !('sessionId' in value) || value.sessionId !== sessionId || !('display' in value)) throw new Error('截图选择层会话无效。');
  const display = parseDisplays([value.display])[0];
  const preview = await decodePreview(await invoke<unknown>('region_overlay_image', { sessionId }), display.id);
  if (preview.widthPx !== display.widthPx || preview.heightPx !== display.heightPx) { releasePreview(preview); throw new Error('冻结屏幕尺寸与显示器不一致。'); }
  return { info: { sessionId, display }, preview };
}

export function readyOverlay(sessionId: string, viewport: Size) {
  return invoke<void>('ready_region_overlay', { sessionId, viewportWidth: viewport.width, viewportHeight: viewport.height });
}
export function commitSelection(sessionId: string, start: Point, end: Point, viewport: Size) {
  return invoke<void>('commit_region_selection', { sessionId, selection: { start, end, viewportWidth: viewport.width, viewportHeight: viewport.height } });
}
export function cancelSelection(sessionId: string, reason: 'user' | 'error' | 'viewport_changed' = 'user') {
  return invoke<void>('cancel_region_selection', { sessionId, reason });
}
