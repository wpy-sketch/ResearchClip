import { useEffect, useRef, useState } from 'react';
import type { PointerEvent as ReactPointerEvent } from 'react';
import type { CapturePreview } from '../../shared/types/capture';
import { cancelSelection, commitSelection, loadOverlay, readyOverlay } from '../../shared/api/region-overlay';
import { releasePreview } from '../../shared/api/capture';
import { errorMessage } from '../../shared/api/errors';
import { selectionToPixels } from './geometry';
import type { Point, Size } from './geometry';

export function CaptureOverlay({ sessionId }: { sessionId: string }) {
  const [preview, setPreview] = useState<CapturePreview | null>(null);
  const [viewport, setViewport] = useState<Size | null>(null);
  const [drag, setDrag] = useState<{ start: Point; end: Point } | null>(null);
  const [message, setMessage] = useState('正在准备冻结屏幕…');
  const [ready, setReady] = useState(false);
  const [pending, setPending] = useState(false);
  const dragRef = useRef<{ start: Point; end: Point } | null>(null);
  const readyRef = useRef(false);
  const starting = useRef(false);
  const pendingRef = useRef(false);

  useEffect(() => {
    let disposed = false;
    let owned: CapturePreview | null = null;
    void loadOverlay(sessionId).then((data) => {
      owned = data.preview;
      if (disposed) releasePreview(owned); else setPreview(owned);
    }).catch((error) => {
      if (!disposed) {
        setMessage(errorMessage(error, '无法读取冻结屏幕。'));
        void cancelSelection(sessionId, 'error').catch((error) => setMessage(errorMessage(error, '取消未完成，选择层将自动结束。')));
      }
    });
    const cancel = () => {
      if (pendingRef.current) return;
      pendingRef.current = true; setPending(true);
      void cancelSelection(sessionId).catch((error) => { setMessage(errorMessage(error, '无法取消截图，请关闭选择窗口。')); });
    };
    const key = (event: KeyboardEvent) => { if (event.key === 'Escape') { event.preventDefault(); cancel(); } };
    const resize = () => {
      if (readyRef.current && !pendingRef.current) {
        pendingRef.current = true; setPending(true);
        void cancelSelection(sessionId, 'viewport_changed').catch((error) => setMessage(errorMessage(error, '窗口尺寸已变化，请关闭选择窗口。')));
      }
    };
    window.addEventListener('keydown', key);
    window.addEventListener('resize', resize);
    return () => { disposed = true; releasePreview(owned); window.removeEventListener('keydown', key); window.removeEventListener('resize', resize); };
  }, [sessionId]);

  const show = async () => {
    if (starting.current) return;
    starting.current = true;
    // Hidden WebViews may suspend requestAnimationFrame; layout must be readable before show().
    await new Promise<void>((resolve) => setTimeout(resolve, 50));
    const size = { width: innerWidth, height: innerHeight };
    try {
      await readyOverlay(sessionId, size);
      setViewport(size); readyRef.current = true; setReady(true); setMessage('拖动选择区域 · Esc 取消');
    } catch (error) {
      setMessage(errorMessage(error, '无法显示截图选择层。'));
      void cancelSelection(sessionId, 'error').catch((error) => setMessage(errorMessage(error, '取消未完成，选择层将自动结束。')));
    }
  };
  const point = (event: ReactPointerEvent<HTMLDivElement>): Point => ({ x: Math.max(0, Math.min(innerWidth, event.clientX)), y: Math.max(0, Math.min(innerHeight, event.clientY)) });
  const down = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!ready || pendingRef.current || event.button !== 0) return;
    event.preventDefault(); event.currentTarget.setPointerCapture(event.pointerId);
    const p = point(event); dragRef.current = { start: p, end: p }; setDrag(dragRef.current); setMessage('松开鼠标完成截图 · Esc 取消');
  };
  const move = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!dragRef.current || pendingRef.current) return;
    dragRef.current = { ...dragRef.current, end: point(event) }; setDrag(dragRef.current);
  };
  const up = (event: ReactPointerEvent<HTMLDivElement>) => {
    const current = dragRef.current;
    if (!current || !preview || !viewport || pendingRef.current) return;
    const end = point(event);
    dragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    const rect = selectionToPixels(current.start, end, viewport, { width: preview.widthPx, height: preview.heightPx });
    if (!rect) { setDrag(null); setMessage('选区没有面积，请重新拖动 · Esc 取消'); return; }
    setDrag({ start: current.start, end }); pendingRef.current = true; setPending(true); setMessage('正在生成区域 PNG…');
    void commitSelection(sessionId, current.start, end, viewport).catch((error) => {
      pendingRef.current = false; setPending(false); setDrag(null); setMessage(errorMessage(error, '选区提交失败，请重试。'));
    });
  };
  const pixelRect = drag && viewport && preview ? selectionToPixels(drag.start, drag.end, viewport, { width: preview.widthPx, height: preview.heightPx }) : null;
  const box = drag ? { x: Math.min(drag.start.x, drag.end.x), y: Math.min(drag.start.y, drag.end.y), width: Math.abs(drag.end.x - drag.start.x), height: Math.abs(drag.end.y - drag.start.y) } : null;

  return <div className="capture-overlay" aria-label="截图选择层" data-ready={ready} data-pending={pending} onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={() => { if (!pendingRef.current) { dragRef.current = null; setDrag(null); setMessage('拖动选择区域 · Esc 取消'); } }}>
    {preview && <img className="frozen-screen" src={preview.url} alt="原生冻结屏幕" draggable={false} onLoad={() => void show()} onError={() => { setMessage('冻结屏幕显示失败。'); void cancelSelection(sessionId, 'error').catch((error) => setMessage(errorMessage(error, '取消未完成，选择层将自动结束。'))); }} />}
    {box ? <>
      <div className="selection-dim" style={{ left: 0, top: 0, right: 0, height: box.y }} />
      <div className="selection-dim" style={{ left: 0, top: box.y + box.height, right: 0, bottom: 0 }} />
      <div className="selection-dim" style={{ left: 0, top: box.y, width: box.x, height: box.height }} />
      <div className="selection-dim" style={{ left: box.x + box.width, top: box.y, right: 0, height: box.height }} />
      <div className="selection-border" style={{ left: box.x, top: box.y, width: box.width, height: box.height }} />
      <output className="selection-size" style={{ left: Math.max(8, Math.min(innerWidth - 150, box.x)), top: Math.max(8, Math.min(innerHeight - 36, box.y > 40 ? box.y - 34 : box.y + box.height + 8)) }}>{pixelRect ? `${pixelRect.width} × ${pixelRect.height} px` : '0 × 0 px'}</output>
    </> : <div className="selection-dim" style={{ inset: 0 }} />}
    <div className="selection-help" role="status">{message}</div>
  </div>;
}
