import { create } from 'zustand';
import { captureFullscreen, captureRegion, captureWindow, copyCaptureImage, discardCapture, listDisplays, readCapturePreview, releasePreview, saveCapturePng } from '../../shared/api/capture';
import { errorMessage } from '../../shared/api/errors';
import type { CaptureInfo, CapturePreview, DisplayInfo } from '../../shared/types/capture';

interface CaptureState {
  displays: DisplayInfo[];
  selectedDisplayId: string | null;
  phase: 'idle' | 'listing' | 'capturing' | 'working';
  preview: CapturePreview | null;
  error: string | null;
  notice: string | null;
  refreshDisplays: () => Promise<void>;
  selectDisplay: (id: string) => void;
  capture: () => Promise<void>;
  captureRegion: () => Promise<void>;
  captureWindow: () => Promise<void>;
  clearPreview: () => void;
  previewFailed: (url: string) => void;
  copyImage: () => Promise<void>;
  savePng: () => Promise<void>;
  acceptNative: (info: CaptureInfo) => Promise<void>;
}

// One session preview URL, no pixel arrays, persistence, timers or request queue.
export const useCaptureStore = create<CaptureState>((set, get) => ({
  displays: [], selectedDisplayId: null, phase: 'idle', preview: null, error: null, notice: null,
  refreshDisplays: async () => {
    if (get().phase !== 'idle') return;
    set({ phase: 'listing', error: null });
    try {
      const displays = await listDisplays();
      const selected = displays.find((display) => display.id === get().selectedDisplayId)
        ?? displays.find((display) => display.isPrimary) ?? displays[0];
      set({ displays, selectedDisplayId: selected.id, phase: 'idle' });
    } catch (error) {
      set({ displays: [], selectedDisplayId: null, phase: 'idle', error: errorMessage(error, '无法读取显示器，请刷新后重试。') });
    }
  },
  selectDisplay: (id) => {
    if (get().phase === 'idle' && get().displays.some((display) => display.id === id)) set({ selectedDisplayId: id, error: null });
  },
  capture: async () => {
    if (get().phase !== 'idle') return;
    const id = get().selectedDisplayId;
    if (!id) { set({ error: '请先选择可用的显示器。' }); return; }
    set({ phase: 'capturing', error: null, notice: null });
    try {
      const preview = await captureFullscreen(id);
      releasePreview(get().preview);
      set({ preview, phase: 'idle' });
    } catch (error) {
      set({ phase: 'idle', error: errorMessage(error, '全屏截图失败，请刷新显示器后重试。') });
    }
  },
  captureRegion: async () => {
    if (get().phase !== 'idle') return;
    const id = get().selectedDisplayId;
    if (!id) { set({ error: '请先选择可用的显示器。' }); return; }
    set({ phase: 'capturing', error: null, notice: null });
    try {
      const preview = await captureRegion(id);
      if (preview) { releasePreview(get().preview); set({ preview }); }
      set({ phase: 'idle' });
    } catch (error) { set({ phase: 'idle', error: errorMessage(error, '区域截图失败，请重试。') }); }
  },
  captureWindow: async () => {
    if (get().phase !== 'idle') return;
    set({ phase: 'capturing', error: null, notice: null });
    try {
      const preview = await captureWindow();
      if (preview) { releasePreview(get().preview); set({ preview }); }
      set({ phase: 'idle' });
    } catch (error) { set({ phase: 'idle', error: errorMessage(error, '窗口截图失败，请选择普通桌面窗口后重试。') }); }
  },
  clearPreview: () => {
    if (get().phase !== 'idle') return;
    const id = get().preview?.captureId;
    releasePreview(get().preview);
    set({ preview: null, notice: null, error: null });
    if (id) void discardCapture(id).catch((error) => set({ error: errorMessage(error, '无法释放截图资源。') }));
  },
  previewFailed: (url) => {
    if (get().preview?.url !== url) return;
    releasePreview(get().preview);
    set({ preview: null, error: '截图预览无法显示，请重试。' });
  },
  copyImage: async () => {
    const preview = get().preview; if (get().phase !== 'idle' || !preview?.captureId) return;
    set({ phase: 'working', error: null, notice: null });
    try { await copyCaptureImage(preview.captureId); set({ phase: 'idle', notice: '图片已复制，可粘贴到画图、Word 等应用。' }); }
    catch (error) { set({ phase: 'idle', error: errorMessage(error, '复制图片失败，请重试。') }); }
  },
  savePng: async () => {
    const preview = get().preview; if (get().phase !== 'idle' || !preview?.captureId) return;
    set({ phase: 'working', error: null, notice: null });
    try {
      const savedPath = await saveCapturePng(preview.captureId);
      if (savedPath && get().preview?.url === preview.url) set({ preview: { ...preview, savedPath }, notice: 'PNG 已保存。' });
      set({ phase: 'idle' });
    } catch (error) { set({ phase: 'idle', error: errorMessage(error, 'PNG 保存失败，请重试。') }); }
  },
  acceptNative: async (info) => {
    if (get().preview?.captureId === info.captureId) { set({ phase: 'idle' }); return; }
    try { const preview = await readCapturePreview(info); releasePreview(get().preview); set({ preview, phase: 'idle', error: null, notice: null }); }
    catch (error) { set({ phase: 'idle', error: errorMessage(error, '无法读取快捷键截图。') }); }
  },
}));
