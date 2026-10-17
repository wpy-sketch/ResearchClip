import { invoke, isTauri } from '@tauri-apps/api/core';
import { parsePngSize } from '../../shared/api/capture';
import type { EditorSession } from '../editor/types';

export interface PinInfo { widthPx: number; heightPx: number; opacity: number }
export type PinAction = 'ready'|'drag'|'zoom'|'opacity'|'close'|'copy'|'save'|'recover';
export function parsePinInfo(value: unknown): PinInfo {
  if (!value || typeof value !== 'object') throw new Error('贴图状态无效。');
  const v = value as Record<string, unknown>;
  if (![v.widthPx,v.heightPx,v.opacity].every(n=>typeof n==='number'&&Number.isFinite(n)) || !Number.isInteger(v.widthPx) || !Number.isInteger(v.heightPx) || Number(v.widthPx)<=0 || Number(v.heightPx)<=0 || Number(v.opacity)<20 || Number(v.opacity)>100) throw new Error('贴图状态无效。');
  return {widthPx:Number(v.widthPx),heightPx:Number(v.heightPx),opacity:Number(v.opacity)};
}
export async function createPin(blob: Blob) {
  if (!isTauri()) throw new Error('桌面贴图需要在 ResearchClip 桌面应用中使用。');
  const bytes = await blob.arrayBuffer(); parsePngSize(bytes);
  const result = await invoke<unknown>('create_pin', bytes);
  if (typeof result!=='string'||!/^pin-\d+$/.test(result)) throw new Error('贴图窗口返回无效标识。');
  return result;
}
export async function pinEditor(session: EditorSession) {
  // Reuse the original-resolution, handle-free annotation export renderer.
  const { renderComposite } = await import('../editor/export');
  return createPin(await renderComposite(session.image.blob, session.document));
}
export async function pinAction(action: PinAction, value?: number) {
  const result = await invoke<unknown>('pin_action', {action,value:value??null});
  return result===null ? null : parsePinInfo(result);
}
export async function loadPin() {
  const bytes = await invoke<ArrayBuffer>('pin_image'); parsePngSize(bytes);
  return URL.createObjectURL(new Blob([bytes],{type:'image/png'}));
}
export async function recoverPins() { await invoke('recover_pins'); }
