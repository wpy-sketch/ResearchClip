import { invoke, isTauri } from '@tauri-apps/api/core';
import type { CaptureInfo, CapturePreview, DisplayInfo } from '../types/capture';

const MAX_PIXELS = 16_777_216;
const MAX_PNG_BYTES = MAX_PIXELS * 4 + 1_048_576;
const integer = (value: unknown): value is number => typeof value === 'number' && Number.isInteger(value);

export function parseDisplays(value: unknown): DisplayInfo[] {
  if (!Array.isArray(value) || value.length === 0 || value.length > 64) throw new Error('当前没有有效的显示器，请刷新后重试。');
  const ids = new Set<string>();
  for (const item of value) {
    if (!item || typeof item !== 'object') throw new Error('显示器信息与前端协议不兼容。');
    const display = item as Record<string, unknown>;
    if (typeof display.id !== 'string' || !display.id || display.id.length > 256 || ids.has(display.id)
      || typeof display.name !== 'string' || !display.name
      || !integer(display.widthPx) || display.widthPx <= 0 || display.widthPx > 2_147_483_647
      || !integer(display.heightPx) || display.heightPx <= 0 || display.heightPx > 2_147_483_647
      || !integer(display.xPx) || !integer(display.yPx) || typeof display.isPrimary !== 'boolean') {
      throw new Error('显示器信息与前端协议不兼容。');
    }
    ids.add(display.id);
  }
  return value as DisplayInfo[];
}

export function parsePngSize(buffer: unknown): { widthPx: number; heightPx: number } {
  if (!(buffer instanceof ArrayBuffer) || buffer.byteLength < 33 || buffer.byteLength > MAX_PNG_BYTES) throw new Error('截图数据为空、无效或超出大小上限。');
  const header = new Uint8Array(buffer, 0, 16);
  const signature = [137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82];
  if (!signature.every((value, index) => header[index] === value)) throw new Error('截图不是有效的 PNG 图像。');
  const view = new DataView(buffer);
  const widthPx = view.getUint32(16);
  const heightPx = view.getUint32(20);
  if (!widthPx || !heightPx || widthPx * heightPx > MAX_PIXELS) throw new Error('截图像素尺寸超过当前支持范围。');
  return { widthPx, heightPx };
}

function requireDesktop() {
  if (!isTauri()) throw new Error('屏幕截图只能在 Windows 桌面应用中使用。');
}

export async function listDisplays(): Promise<DisplayInfo[]> {
  requireDesktop();
  return parseDisplays(await invoke<unknown>('list_displays'));
}

export async function captureFullscreen(displayId: string): Promise<CapturePreview> {
  requireDesktop();
  const info = parseCaptureInfo(await invoke<unknown>('capture_fullscreen', { displayId }), displayId);
  return readCapturePreview(info);
}

export async function decodePreview(buffer: unknown, displayId: string): Promise<CapturePreview> {
  const size = parsePngSize(buffer);
  const blob = new Blob([buffer as ArrayBuffer], { type: 'image/png' });
  // Decode before replacing the current preview; invalid PNG never discards it.
  const bitmap = await createImageBitmap(blob);
  try {
    if (bitmap.width !== size.widthPx || bitmap.height !== size.heightPx) throw new Error('截图解码尺寸与返回像素尺寸不一致。');
  } finally { bitmap.close(); }
  return { ...size, displayId, blob, url: URL.createObjectURL(blob) };
}

export function parseCaptureInfo(value: unknown, displayId?: string): CaptureInfo {
  if (!value || typeof value !== 'object') throw new Error('区域截图返回了无效的结果。');
  const result = value as Record<string, unknown>;
  if (typeof result.captureId !== 'string' || !/^[a-zA-Z0-9-]{1,128}$/.test(result.captureId) || typeof result.displayId !== 'string' || !result.displayId || (displayId !== undefined && result.displayId !== displayId)
    || !integer(result.widthPx) || !integer(result.heightPx) || result.widthPx <= 0 || result.heightPx <= 0 || result.widthPx * result.heightPx > MAX_PIXELS
    || !['fullscreen', 'region', 'scroll', 'window'].includes(String(result.mode))) throw new Error('截图结果与前端协议不兼容。');
  return value as CaptureInfo;
}

export async function readCapturePreview(info: CaptureInfo): Promise<CapturePreview> {
  requireDesktop();
  const preview = await decodePreview(await invoke<unknown>('capture_result_png', { captureId: info.captureId }), info.displayId);
  if (preview.widthPx !== info.widthPx || preview.heightPx !== info.heightPx) { releasePreview(preview); throw new Error('PNG 的实际尺寸与截图结果不一致。'); }
  return { ...preview, captureId: info.captureId, mode: info.mode };
}

export async function captureRegion(displayId: string): Promise<CapturePreview | null> {
  requireDesktop();
  let value: unknown;
  try { value = await invoke<unknown>('capture_region', { displayId }); }
  catch (error) {
    if (error && typeof error === 'object' && 'code' in error && error.code === 'CANCELLED') return null;
    throw error;
  }
  return readCapturePreview(parseCaptureInfo(value, displayId));
}

export async function copyCaptureImage(captureId: string) { requireDesktop(); await invoke('copy_capture_image', { captureId }); }
export async function discardCapture(captureId: string) { requireDesktop(); await invoke('discard_capture', { captureId }); }
export async function saveCapturePng(captureId: string): Promise<string | null> {
  requireDesktop(); const result = await invoke<unknown>('save_capture_png', { captureId });
  if (result === null) return null;
  if (typeof result !== 'string' || !/\.png$/i.test(result)) throw new Error('保存结果无效，请检查所选文件。');
  return result;
}

export function releasePreview(preview: CapturePreview | null) {
  if (preview) URL.revokeObjectURL(preview.url);
}

export async function captureWindow(): Promise<CapturePreview | null> {
  requireDesktop();
  try { return await readCapturePreview(parseCaptureInfo(await invoke<unknown>("capture_window"))); }
  catch(error) { if(error && typeof error === "object" && "code" in error && error.code === "CANCELLED") return null; throw error; }
}
