import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { captureFullscreen, captureRegion, listDisplays, parseDisplays, parsePngSize, parseCaptureInfo } from './capture';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
const display = { id: 'display-1', name: '显示器 1', widthPx: 2560, heightPx: 1440, xPx: -2560, yPx: 0, isPrimary: true };
// Header fixture for contract tests only. Native acceptance decodes a real screenshot.
function pngHeader(width = 2, height = 1) {
  const buffer = new ArrayBuffer(33);
  new Uint8Array(buffer).set([137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82]);
  const view = new DataView(buffer);
  view.setUint32(16, width); view.setUint32(20, height);
  return buffer;
}
beforeEach(() => { vi.resetAllMocks(); vi.mocked(isTauri).mockReturnValue(true); });
afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); });

describe('capture IPC boundary', () => {
  it('treats only explicit user cancellation as an empty region result', async () => {
    vi.mocked(invoke).mockRejectedValue({ code: 'CANCELLED' });
    expect(await captureRegion('display-1')).toBeNull();
    vi.mocked(invoke).mockRejectedValue({ code: 'DISPLAY_CHANGED', message: '尺寸变化' });
    await expect(captureRegion('display-1')).rejects.toMatchObject({ code: 'DISPLAY_CHANGED' });
  });
  it('validates saved region metadata and reads only the returned resource ID', async () => {
    const result = { captureId: 'opaque', displayId: 'display-1', widthPx: 2, heightPx: 1, mode: 'region' };
    vi.mocked(invoke).mockResolvedValueOnce(result).mockResolvedValueOnce(pngHeader());
    vi.stubGlobal('createImageBitmap', vi.fn().mockResolvedValue({ width: 2, height: 1, close: vi.fn() }));
    vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:region');
    expect(await captureRegion('display-1')).toMatchObject({ mode: 'region', captureId: result.captureId, widthPx: 2 });
    expect(invoke).toHaveBeenNthCalledWith(1, 'capture_region', { displayId: 'display-1' });
    expect(invoke).toHaveBeenNthCalledWith(2, 'capture_result_png', { captureId: 'opaque' });
    expect(() => parseCaptureInfo({ ...result, displayId: 'other' }, 'display-1')).toThrow();
    expect(() => parseCaptureInfo({ ...result, widthPx: 0 }, 'display-1')).toThrow();
  });
  it('rejects preview without invoking native commands', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(listDisplays()).rejects.toThrow('Windows');
    await expect(captureFullscreen('display-1')).rejects.toThrow('Windows');
    expect(invoke).not.toHaveBeenCalled();
  });
  it('uses the real enumeration command and supports negative physical origins', async () => {
    vi.mocked(invoke).mockResolvedValue([display]);
    expect(await listDisplays()).toEqual([display]);
    expect(invoke).toHaveBeenCalledWith('list_displays');
  });
  it.each([[], null, [display, display], [{ ...display, widthPx: 0 }], [{ ...display, isPrimary: 'yes' }], [{ ...display, xPx: NaN }]])('rejects malformed enumeration %j', (value) => {
    expect(() => parseDisplays(value)).toThrow();
  });
  it('checks PNG signature, dimensions and pixel limits', () => {
    expect(parsePngSize(pngHeader())).toEqual({ widthPx: 2, heightPx: 1 });
    for (const value of [new ArrayBuffer(0), new ArrayBuffer(33), [137, 80], pngHeader(0), pngHeader(4097, 4096)]) expect(() => parsePngSize(value)).toThrow();
  });
  it('decodes native binary before creating a preview URL and closes decoder resources', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ captureId: 'opaque', displayId: 'display-1', widthPx: 2, heightPx: 1, mode: 'fullscreen' }).mockResolvedValueOnce(pngHeader());
    const close = vi.fn();
    vi.stubGlobal('createImageBitmap', vi.fn().mockResolvedValue({ width: 2, height: 1, close }));
    vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:test-preview');
    expect(await captureFullscreen('display-1')).toMatchObject({ captureId: 'opaque', mode: 'fullscreen', url: 'blob:test-preview', displayId: 'display-1', widthPx: 2, heightPx: 1, blob: expect.any(Blob) });
    expect(invoke).toHaveBeenCalledWith('capture_fullscreen', { displayId: 'display-1' });
    expect(close).toHaveBeenCalledOnce();
  });
  it('does not create a URL for invalid or mismatched decoded pixels', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ captureId: 'opaque', displayId: 'display-1', widthPx: 2, heightPx: 1, mode: 'fullscreen' }).mockResolvedValueOnce(pngHeader());
    const close = vi.fn();
    vi.stubGlobal('createImageBitmap', vi.fn().mockResolvedValue({ width: 1, height: 1, close }));
    const create = vi.spyOn(URL, 'createObjectURL');
    await expect(captureFullscreen('display-1')).rejects.toThrow('不一致');
    expect(close).toHaveBeenCalledOnce();
    expect(create).not.toHaveBeenCalled();
  });
  it('propagates structured native errors without a fake result', async () => {
    const error = { code: 'PERMISSION_DENIED', message: '桌面访问被拒绝' };
    vi.mocked(invoke).mockRejectedValue(error);
    await expect(captureFullscreen('display-1')).rejects.toEqual(error);
  });
});
