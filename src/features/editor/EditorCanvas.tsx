import { useCallback, useEffect, useRef, useState } from 'react';
import { Group, Image as CanvasImage, Layer, Stage } from 'react-konva';
import type Konva from 'konva';
import { screenToImage, validateImageSize } from './geometry';
import { useEditorStore } from './store';
import type { EditorSession, Point, Viewport } from './types';
import { AnnotationNode } from './AnnotationNode';
import { inImage } from './model';

export function EditorCanvas({ session, onSize, onText }: { session: EditorSession; onSize: (size: { width: number; height: number }) => void; onText: (point: Point, id?: string) => void }) {
  const host = useRef<HTMLDivElement>(null); const stage = useRef<Konva.Stage>(null);
  const [size, setSize] = useState({ width: 1, height: 1 }); const [loaded, setLoaded] = useState<HTMLImageElement | null>(null);
  const [position, setPosition] = useState<Point | null>(null);
  const pan = useRef<{ start: Point; viewport: Viewport; cancelVersion: number } | null>(null);
  const tool = useEditorStore((s) => s.tool);
  const gesture = useEditorStore((s) => s.gesture);
  const cancelVersion = useEditorStore((s)=>s.cancelVersion);
  const space = useRef(false); const [spaceHeld,setSpaceHeld]=useState(false);
  const transforming = useRef(false);
  const registerTransform = useCallback((active:boolean)=>{transforming.current=active;},[]);
  const { image, viewport } = session;
  useEffect(()=>{if(pan.current?.cancelVersion!==cancelVersion)pan.current=null;},[cancelVersion]);
  useEffect(()=>{
    const editable=(target:EventTarget|null)=>target instanceof HTMLElement&&(target.isContentEditable||['INPUT','TEXTAREA','SELECT'].includes(target.tagName));
    const down=(e:KeyboardEvent)=>{if(editable(e.target))return;if(e.code==='Space'&&e.target instanceof Node&&host.current?.contains(e.target)){e.preventDefault();space.current=true;setSpaceHeld(true);useEditorStore.getState().cancelGesture();}if(e.key==='Escape'){pan.current=null;useEditorStore.getState().cancelGesture();}};
    const up=(e:KeyboardEvent)=>{if(e.code==='Space'){space.current=false;setSpaceHeld(false);}};
    const blur=()=>{space.current=false;setSpaceHeld(false);pan.current=null;useEditorStore.getState().cancelGesture();};
    window.addEventListener('keydown',down);window.addEventListener('keyup',up);window.addEventListener('blur',blur);
    return()=>{window.removeEventListener('keydown',down);window.removeEventListener('keyup',up);window.removeEventListener('blur',blur);};
  },[]);
  const point = useCallback(()=>{const screen=stage.current?.getPointerPosition();const state=useEditorStore.getState();const current=state.sessions[image.id];return screen&&current?screenToImage(screen,current.viewport):null;},[image.id]);
  useEffect(() => {
    const img = new Image(); let active = true;
    img.onload = () => { if (!active) return; try { validateImageSize(img.naturalWidth, img.naturalHeight); if (img.naturalWidth !== image.widthPx || img.naturalHeight !== image.heightPx) throw new Error('实际图片尺寸与编辑会话不一致。'); setLoaded(img); useEditorStore.getState().imageFailed(image.id, null); } catch (error) { useEditorStore.getState().imageFailed(image.id, error instanceof Error ? error.message : '图片无法加载。'); } };
    img.onerror = () => { if (active) useEditorStore.getState().imageFailed(image.id, '图片加载失败，请返回截图工作流重新打开图片。'); };
    img.src = image.url; return () => { active = false; img.onload = null; img.onerror = null; img.src = ''; };
  }, [image.id, image.url, image.widthPx, image.heightPx]);
  useEffect(() => {
    const element = host.current; if (!element) return;
    const observer = new ResizeObserver(() => {
      const bounds = element.getBoundingClientRect(); const next = { width: Math.max(1, Math.floor(bounds.width)), height: Math.max(1, Math.floor(bounds.height)) };
      setSize(next); onSize(next);
      const state = useEditorStore.getState(); const current = state.sessions[image.id];
      if (current?.viewport.mode === 'fit' && state.activeId === image.id) state.fit(next.width, next.height);
    }); observer.observe(element); return () => observer.disconnect();
  }, [image.id, onSize]);
  const move = () => {
    const point = stage.current?.getPointerPosition(); if (!point) return;
    const view = useEditorStore.getState().sessions[image.id].viewport;
    if (pan.current) { const start = pan.current; const next: Viewport = { ...start.viewport, x: start.viewport.x + point.x - start.start.x, y: start.viewport.y + point.y - start.start.y, mode: 'manual' }; useEditorStore.getState().setViewport(next); setPosition(screenToImage(point,next)); }
    else { setPosition(screenToImage(point, view)); if(!transforming.current) useEditorStore.getState().moveGesture(screenToImage(point,view)); }
  };
  return <div className="editor-canvas-frame">
    {session.loadError && <p className="error-banner m-3" role="alert">{session.loadError}</p>}
    <div className="editor-canvas-host" ref={host} tabIndex={0} role="application" aria-label="图片编辑画布" style={{ cursor: tool === 'pan'||spaceHeld ? 'grab' : tool==='select'?'default':'crosshair' }}
      onPointerDown={(event)=>{host.current?.focus();(event.target as HTMLElement).setPointerCapture?.(event.pointerId);}}>
      <Stage ref={stage} width={size.width} height={size.height}
        onPointerDown={(event) => {
          if(session.loadError||!loaded)return;
          const screen=stage.current?.getPointerPosition();if(!screen)return;
          if(tool==='pan'||event.evt.button===1||space.current){event.evt.preventDefault();useEditorStore.getState().cancelGesture();pan.current={start:screen,viewport:useEditorStore.getState().sessions[image.id].viewport,cancelVersion:useEditorStore.getState().cancelVersion};return;}
          if(event.evt.button!==0)return;const raw=point();if(!raw)return;
          if(tool==='select'){if(event.target===stage.current)useEditorStore.getState().select(null);}
          else if(tool==='text'){if(inImage(raw,image.widthPx,image.heightPx))onText(raw);}
          else useEditorStore.getState().beginDraw(raw);
        }}
        onPointerMove={move} onPointerUp={() => { pan.current = null;if(!transforming.current)useEditorStore.getState().finishGesture(); }} onPointerCancel={() => { pan.current = null;useEditorStore.getState().cancelGesture(); }}
        onWheel={(event) => { event.evt.preventDefault();useEditorStore.getState().cancelGesture(); const at = stage.current?.getPointerPosition(); const view=useEditorStore.getState().sessions[image.id].viewport;if(at)useEditorStore.getState().zoom(view.scale * Math.exp(-event.evt.deltaY * 0.001),at); }}>
        <Layer listening={false}><Group x={viewport.x} y={viewport.y} scaleX={viewport.scale} scaleY={viewport.scale}>{loaded && !session.loadError && <CanvasImage image={loaded} width={image.widthPx} height={image.heightPx} />}</Group></Layer>
        <Layer><Group x={viewport.x} y={viewport.y} scaleX={viewport.scale} scaleY={viewport.scale}>
          {session.document.objects.map((original)=>{const a=gesture?.imageId===image.id&&gesture.draft.id===original.id?gesture.draft:original;return <AnnotationNode key={a.id} annotation={a} selected={session.selectedId===a.id} scale={viewport.scale} selectable={tool==='select'&&!spaceHeld&&!session.loadError} point={point} editText={(a)=>onText({x:a.x,y:a.y},a.id)} registerTransform={registerTransform} />;})}
          {gesture?.imageId===image.id&&gesture.kind==='draw'&&<AnnotationNode annotation={gesture.draft} selected={false} scale={viewport.scale} selectable={false} point={point} editText={()=>{}} registerTransform={registerTransform} />}
        </Group></Layer>
      </Stage>
    </div><div className="editor-coordinate-status">原图 {image.widthPx} × {image.heightPx} 像素<span>{position ? `X ${position.x.toFixed(1)} · Y ${position.y.toFixed(1)}` : '鼠标位置：原图像素坐标'}</span></div>
  </div>;
}
