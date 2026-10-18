import { useEffect } from 'react';
import { Camera, Crop, RefreshCw } from 'lucide-react';
import { useAppStore } from '../../app/store';
import { useCaptureStore } from './store';

export function CaptureControls({ onCaptured,compact=false }: { onCaptured?: () => void; compact?:boolean }) {
  const desktop = useAppStore((state) => state.info?.mode === 'desktop');
  const { displays, selectedDisplayId, phase, error, refreshDisplays, selectDisplay, capture, captureRegion } = useCaptureStore();
  useEffect(() => {
    if (desktop && useCaptureStore.getState().displays.length === 0) void refreshDisplays();
  }, [desktop, refreshDisplays]);
  const busy = phase !== 'idle';

  return <div>
    <div className="capture-controls">
      <label className="min-w-0 flex-1">
        <span className={compact?'sr-only':'mb-2 block text-xs text-muted'}>目标显示器</span>
        <select aria-label="目标显示器" value={selectedDisplayId ?? ''} disabled={!desktop || busy || displays.length === 0} onChange={(event) => selectDisplay(event.target.value)} className="display-select">
          {displays.length === 0 && <option value="">{phase === 'listing' ? '正在读取显示器…' : desktop ? '暂无可用显示器' : '请启动 Windows 桌面应用'}</option>}
          {displays.map((display, index) => <option key={display.id} value={display.id}>显示器 {index + 1}{display.isPrimary ? ' · 主显示器' : ''} · {display.widthPx} × {display.heightPx}</option>)}
        </select>
      </label>
      <button className="icon-button" aria-label="刷新显示器" title="刷新显示器" disabled={!desktop || busy} onClick={() => void refreshDisplays()}><RefreshCw size={16} /></button>
      <button className="secondary-button h-9 flex items-center gap-2" aria-label="区域截图" disabled={!desktop || busy || !selectedDisplayId} onClick={() => {
        const previousUrl = useCaptureStore.getState().preview?.url;
        void captureRegion().then(() => {
          const state = useCaptureStore.getState();
          if (!state.error && state.preview?.mode === 'region' && state.preview.url !== previousUrl) onCaptured?.();
        });
      }}><Crop size={15} />区域截图</button>
      <button className="capture-button" aria-label="全屏截图" disabled={!desktop || busy || !selectedDisplayId} onClick={() => {
        const previousUrl = useCaptureStore.getState().preview?.url;
        void capture().then(() => {
          const state = useCaptureStore.getState();
          if (!state.error && state.preview && state.preview.url !== previousUrl) onCaptured?.();
        });
      }}><Camera size={15} /><span>{phase === 'capturing' ? '正在截图…' : '全屏截图'}</span></button>
    </div>
    {(!compact||busy)&&<p className="mt-3 text-xs leading-5 text-muted" role="status">{phase === 'capturing' ? '正在截图，主窗口会短暂隐藏。区域选择时按 Esc 取消。' : phase === 'working' ? '正在处理图片操作…' : '截图仅保留为当前预览；选择“保存 PNG”才写入所选目录。'}</p>}
    {!desktop && <p className="mt-2 text-xs text-muted">浏览器预览不具备屏幕捕获能力。</p>}
    {error && <div className="error-banner mt-4" role="alert">{error}</div>}
  </div>;
}
