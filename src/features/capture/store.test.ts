import { beforeEach, describe, expect, it, vi } from 'vitest';
import { captureFullscreen, captureRegion, captureWindow, copyCaptureImage, discardCapture, listDisplays, readCapturePreview, releasePreview, saveCapturePng } from '../../shared/api/capture';
import { useCaptureStore } from './store';
import type { CapturePreview } from '../../shared/types/capture';

vi.mock('../../shared/api/capture', () => ({ captureFullscreen: vi.fn(), captureRegion: vi.fn(), captureWindow: vi.fn(), listDisplays: vi.fn(), releasePreview: vi.fn(), copyCaptureImage: vi.fn(), discardCapture: vi.fn(), readCapturePreview: vi.fn(), saveCapturePng: vi.fn() }));
const primary = { id: 'main', name: '主显示器', widthPx: 2560, heightPx: 1440, xPx: 0, yPx: 0, isPrimary: true };
const secondary = { ...primary, id: 'second', isPrimary: false, xPx: -2560 };
const preview: CapturePreview = { url: 'blob:first', displayId: 'main', widthPx: 2560, heightPx: 1440 };
beforeEach(() => { vi.resetAllMocks(); useCaptureStore.setState(useCaptureStore.getInitialState(), true); });

describe('single capture session', () => {
  it('copies real resource IDs, preserves preview on clipboard failure, and prevents duplicate actions', async () => {
    useCaptureStore.setState({ preview: { ...preview, captureId: 'one' } });
    let reject!: (error: unknown) => void; vi.mocked(copyCaptureImage).mockReturnValue(new Promise((_, fail) => { reject = fail; }));
    const pending = useCaptureStore.getState().copyImage(); await useCaptureStore.getState().copyImage(); await useCaptureStore.getState().savePng();
    expect(copyCaptureImage).toHaveBeenCalledExactlyOnceWith('one'); expect(saveCapturePng).not.toHaveBeenCalled(); reject({ message: '剪贴板正被占用' }); await pending;
    expect(useCaptureStore.getState()).toMatchObject({ phase: 'idle', error: '剪贴板正被占用', preview: { captureId: 'one' } });
  });
  it('save dialog cancellation preserves preview and success records only the returned path', async () => {
    useCaptureStore.setState({ preview: { ...preview, captureId: 'one' } }); vi.mocked(saveCapturePng).mockResolvedValueOnce(null).mockResolvedValueOnce('C:/chosen/image.png');
    await useCaptureStore.getState().savePng(); expect(useCaptureStore.getState().preview?.savedPath).toBeUndefined();
    await useCaptureStore.getState().savePng(); expect(useCaptureStore.getState().preview?.savedPath).toBe('C:/chosen/image.png');
    vi.mocked(discardCapture).mockResolvedValue(); useCaptureStore.getState().clearPreview(); expect(discardCapture).toHaveBeenCalledWith('one');
  });
  it('native shortcut results decode through the resource gateway and replace the prior URL', async () => {
    useCaptureStore.setState({ preview }); const next = { ...preview, captureId: 'native', url: 'blob:native', mode: 'region' as const };
    vi.mocked(readCapturePreview).mockResolvedValue(next);
    await useCaptureStore.getState().acceptNative({ captureId: 'native', displayId: 'main', widthPx: 2560, heightPx: 1440, mode: 'region' });
    expect(releasePreview).toHaveBeenCalledWith(preview); expect(useCaptureStore.getState()).toMatchObject({ preview: next, phase: 'idle' });
  });
  it('region cancellation preserves the previous image and permits another capture', async () => {
    useCaptureStore.setState({ selectedDisplayId: 'main', preview });
    vi.mocked(captureRegion).mockResolvedValue(null);
    await useCaptureStore.getState().captureRegion();
    expect(useCaptureStore.getState()).toMatchObject({ preview, phase: 'idle', error: null });
    expect(releasePreview).not.toHaveBeenCalled();
  });
  it('region failures preserve the previous preview and show the backend error', async () => {
    useCaptureStore.setState({ selectedDisplayId: 'main', preview });
    vi.mocked(captureRegion).mockRejectedValue({ code: 'SAVE_FAILED', message: '无法保存区域 PNG' });
    await useCaptureStore.getState().captureRegion();
    expect(useCaptureStore.getState()).toMatchObject({ preview, phase: 'idle', error: '无法保存区域 PNG' });
    expect(releasePreview).not.toHaveBeenCalled();
  });
  it('region success replaces one preview and excludes concurrent full capture', async () => {
    useCaptureStore.setState({ selectedDisplayId: 'main', preview });
    let resolve!: (value: CapturePreview) => void;
    vi.mocked(captureRegion).mockReturnValue(new Promise((done) => { resolve = done; }));
    const request = useCaptureStore.getState().captureRegion();
    await useCaptureStore.getState().capture();
    await useCaptureStore.getState().captureRegion();
    expect(captureRegion).toHaveBeenCalledOnce();
    expect(captureFullscreen).not.toHaveBeenCalled();
    const region = { ...preview, url: 'blob:region', mode: 'region' as const, savedPath: 'test.png' };
    resolve(region); await request;
    expect(releasePreview).toHaveBeenCalledWith(preview);
    expect(useCaptureStore.getState().preview).toEqual(region);
  });
  it('selects primary, preserves available selection, and falls back after unplug', async () => {
    vi.mocked(listDisplays).mockResolvedValue([secondary, primary]);
    await useCaptureStore.getState().refreshDisplays();
    expect(useCaptureStore.getState().selectedDisplayId).toBe('main');
    useCaptureStore.getState().selectDisplay('second');
    await useCaptureStore.getState().refreshDisplays();
    expect(useCaptureStore.getState().selectedDisplayId).toBe('second');
    vi.mocked(listDisplays).mockResolvedValue([primary]);
    await useCaptureStore.getState().refreshDisplays();
    expect(useCaptureStore.getState().selectedDisplayId).toBe('main');
  });
  it('deduplicates enumeration and does not queue capture while listing', async () => {
    let resolve!: (value: typeof primary[]) => void;
    vi.mocked(listDisplays).mockReturnValue(new Promise((done) => { resolve = done; }));
    const request = useCaptureStore.getState().refreshDisplays();
    await useCaptureStore.getState().refreshDisplays();
    await useCaptureStore.getState().capture();
    expect(listDisplays).toHaveBeenCalledOnce();
    expect(captureFullscreen).not.toHaveBeenCalled();
    resolve([primary]); await request;
  });
  it('deduplicates capture, keeps selection stable, and revokes the previous preview', async () => {
    useCaptureStore.setState({ displays: [primary, secondary], selectedDisplayId: 'main', preview });
    let resolve!: (value: CapturePreview) => void;
    vi.mocked(captureFullscreen).mockReturnValue(new Promise((done) => { resolve = done; }));
    const request = useCaptureStore.getState().capture();
    useCaptureStore.getState().selectDisplay('second');
    useCaptureStore.getState().clearPreview();
    await useCaptureStore.getState().capture();
    expect(captureFullscreen).toHaveBeenCalledOnce();
    expect(useCaptureStore.getState().selectedDisplayId).toBe('main');
    const next = { ...preview, url: 'blob:next' };
    resolve(next); await request;
    expect(releasePreview).toHaveBeenCalledWith(preview);
    expect(useCaptureStore.getState()).toMatchObject({ preview: next, phase: 'idle', error: null });
  });
  it('preserves the previous screenshot on failure and permits retry', async () => {
    useCaptureStore.setState({ selectedDisplayId: 'main', preview });
    vi.mocked(captureFullscreen).mockRejectedValueOnce({ code: 'DISPLAY_GONE', message: '显示器已断开' }).mockResolvedValueOnce(preview);
    await useCaptureStore.getState().capture();
    expect(useCaptureStore.getState()).toMatchObject({ preview, error: '显示器已断开', phase: 'idle' });
    expect(releasePreview).not.toHaveBeenCalled();
    await useCaptureStore.getState().capture();
    expect(useCaptureStore.getState().error).toBeNull();
  });
  it('clears stale display selection after enumeration failure without discarding the preview', async () => {
    useCaptureStore.setState({ selectedDisplayId: 'main', preview });
    vi.mocked(listDisplays).mockRejectedValue(new Error('无法访问桌面'));
    await useCaptureStore.getState().refreshDisplays();
    await useCaptureStore.getState().capture();
    expect(captureFullscreen).not.toHaveBeenCalled();
    expect(useCaptureStore.getState()).toMatchObject({ displays: [], selectedDisplayId: null, preview });
  });
  it('releases the blob on explicit close or image rendering failure', () => {
    useCaptureStore.setState({ preview });
    useCaptureStore.getState().clearPreview();
    expect(releasePreview).toHaveBeenCalledWith(preview);
    expect(useCaptureStore.getState().preview).toBeNull();
    useCaptureStore.setState({ preview });
    useCaptureStore.getState().previewFailed(preview.url);
    expect(useCaptureStore.getState().error).toContain('无法显示');
    expect(useCaptureStore.getState().preview).toBeNull();
  });
  it('ignores a late image error from a revoked previous preview', () => {
    const current = { ...preview, url: 'blob:current' };
    useCaptureStore.setState({ preview: current });
    useCaptureStore.getState().previewFailed(preview.url);
    expect(useCaptureStore.getState()).toMatchObject({ preview: current, error: null });
    expect(releasePreview).not.toHaveBeenCalled();
  });
});

describe('window capture workflow',()=>{
  beforeEach(()=>{vi.clearAllMocks();useCaptureStore.setState({phase:'idle',preview:null,error:null,notice:null});});
  it('uses the native window capture without needing a display selection',async()=>{const preview={captureId:'window-1',mode:'window',displayId:'window',widthPx:640,heightPx:480,url:'blob:window',blob:new Blob()} as CapturePreview;vi.mocked(captureWindow).mockResolvedValue(preview);useCaptureStore.setState({selectedDisplayId:null});await useCaptureStore.getState().captureWindow();expect(useCaptureStore.getState().preview).toBe(preview);expect(useCaptureStore.getState().phase).toBe('idle');});
  it('retains the previous image when the picker is cancelled',async()=>{const previous={url:'blob:previous'} as CapturePreview;useCaptureStore.setState({preview:previous});vi.mocked(captureWindow).mockResolvedValue(null);await useCaptureStore.getState().captureWindow();expect(useCaptureStore.getState().preview).toBe(previous);expect(releasePreview).not.toHaveBeenCalled();});
  it('shows a native failure and permits a retry without losing the old image',async()=>{const previous={url:'blob:previous'} as CapturePreview;useCaptureStore.setState({preview:previous});vi.mocked(captureWindow).mockRejectedValue({code:'PERMISSION_DENIED',message:'Windows 拒绝访问'});await useCaptureStore.getState().captureWindow();expect(useCaptureStore.getState().preview).toBe(previous);expect(useCaptureStore.getState().error).toContain('拒绝访问');expect(useCaptureStore.getState().phase).toBe('idle');});
  it('does not queue a second capture while another operation owns the gate',async()=>{useCaptureStore.setState({phase:'capturing'});await useCaptureStore.getState().captureWindow();expect(captureWindow).not.toHaveBeenCalled();});
});
