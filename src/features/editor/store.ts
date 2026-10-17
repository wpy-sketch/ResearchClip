import { create } from 'zustand';
import { boundHistory, recordEdit } from './history';
import { parseDocument } from './serialization';
import { fitViewport, validateImageSize, zoomViewport } from './geometry';
import type { Annotation, AnnotationDocument, AnnotationStyle, EditorImage, EditorSession, EditorTool, Point, Viewport } from './types';
import { clampPoint, createAnnotation, drawAnnotation, inImage, moveArrowEndpoint, nonEmpty, translateAnnotation, validateDocument, nextNumber } from './model';

interface Gesture { kind: 'draw' | 'move' | 'endpoint'; imageId: string; start: Point; original: Annotation; draft: Annotation; endpoint?: 0 | 1 }

interface EditorState {
  defaultStyle:Pick<AnnotationStyle,'color'|'strokeWidth'>;
  setDefaults:(style:Pick<AnnotationStyle,'color'|'strokeWidth'>)=>void;
  markSaved:(id:string,document:AnnotationDocument)=>void;
  sessions: Record<string, EditorSession>; activeId: string | null; tool: EditorTool; error: string | null;
  style: AnnotationStyle; gesture: Gesture | null; cancelVersion: number;
  open: (source: Omit<EditorImage, 'url'>, document?: unknown) => void; undo: () => void; redo: () => void; close: (id: string, discard?:boolean) => boolean; activate: (id: string) => void;
  setTool: (tool: EditorTool) => void; fit: (width: number, height: number) => void;
  setViewport: (viewport: Viewport) => void; zoom: (scale: number, anchor: Point) => void;
  imageFailed: (id: string, message: string | null) => void; serialize: (id: string) => AnnotationDocument;
  bindLibrary:(id:string,libraryId:string,libraryRevision:number,name:string)=>void;
  unlinkLibrary:(libraryId:string)=>void;
  select: (id: string | null) => void; add: (annotation: Annotation) => boolean; update: (annotation: Annotation) => boolean; removeSelected: () => void;
  setStyle: (patch: Partial<AnnotationStyle>) => void;
  beginDraw: (point: Point) => void; beginMove: (id: string, point: Point, endpoint?: 0 | 1) => void;
  moveGesture: (point: Point) => void; finishGesture: () => void; cancelGesture: () => void;
}
export const useEditorStore = create<EditorState>((set, get) => ({
  defaultStyle:{color:'#ef6262',strokeWidth:3},
  setDefaults:style=>{if(!/^#[0-9a-f]{6}$/i.test(style.color)||!Number.isInteger(style.strokeWidth)||style.strokeWidth<1||style.strokeWidth>32)return;get().cancelGesture();set({defaultStyle:{...style},style:{...get().style,...style}});},
  markSaved:(id,document)=>{const session=get().sessions[id];if(session)set({sessions:{...get().sessions,[id]:{...session,savedDocument:JSON.stringify(document)}}});},
  sessions: {}, activeId: null, tool: 'select', error: null, style: { color: '#ef6262', strokeWidth: 3, fontSize: 24 }, gesture: null, cancelVersion: 0,
  open: (source, imported) => {
    try {
      validateImageSize(source.widthPx, source.heightPx);
      if(!/^[\w-]{1,128}$/.test(source.id)||['__proto__','constructor','prototype'].includes(source.id)||!(source.blob instanceof Blob)||source.blob.size===0)throw new Error('图片资源标识或数据无效。');
      const document = imported === undefined ? { schemaVersion: 1 as const, image: { widthPx: source.widthPx, heightPx: source.heightPx }, objects: [] } : parseDocument(imported);
      if (document.image.widthPx !== source.widthPx || document.image.heightPx !== source.heightPx) throw new Error('原图尺寸与项目标注不一致。');
      get().cancelGesture();
      const existing = Object.hasOwn(get().sessions,source.id)?get().sessions[source.id]:null; if (existing) { set({ activeId: source.id, error: null }); return; }
      if (Object.keys(get().sessions).length >= 8 || Object.values(get().sessions).reduce((sum, s) => sum + s.image.blob.size, source.blob.size) > 128 * 1024 * 1024) throw new Error('已达到编辑会话容量上限，请先关闭不需要的图片。');
      const image = { ...source, url: URL.createObjectURL(source.blob) };
      set({ sessions: { ...get().sessions, [source.id]: { image, document, savedDocument:JSON.stringify(document),history: { past: [], future: [] }, selectedId: null, viewport: { scale: 1, x: 0, y: 0, mode: 'fit' }, loadError: null } }, activeId: source.id, style:{...get().style,...get().defaultStyle},error: null });
    } catch (error) { set({ error: error instanceof Error ? error.message : '图片无法打开。' }); }
  },
  close: (id, discard=false) => {
    const session = get().sessions[id]; if (!session) return true;
    if(!discard&&(get().gesture?.imageId===id||JSON.stringify(session.document)!==session.savedDocument))return false;
    if (get().activeId === id) get().cancelGesture();
    URL.revokeObjectURL(session.image.url);
    const sessions = { ...get().sessions }; delete sessions[id]; set({ sessions, activeId: get().activeId === id ? Object.keys(sessions)[0] ?? null : get().activeId });
    return true;
  },
  activate: (id) => { if (get().sessions[id]) { get().cancelGesture(); set({ activeId: id, error: null }); } },
  setTool: (tool) => { get().cancelGesture(); set({ tool }); },
  fit: (width, height) => {
    const session = get().activeId ? get().sessions[get().activeId!] : null; if (!session) return;
    get().setViewport(fitViewport(session.image.widthPx, session.image.heightPx, width, height));
  },
  setViewport: (viewport) => {
    const id = get().activeId; if (!id || !get().sessions[id] || ![viewport.x, viewport.y, viewport.scale].every(Number.isFinite) || viewport.scale <= 0) return;
    set({ sessions: { ...get().sessions, [id]: { ...get().sessions[id], viewport } } });
  },
  zoom: (scale, anchor) => { const id = get().activeId; if (id) get().setViewport(zoomViewport(get().sessions[id].viewport, scale, anchor)); },
  imageFailed: (id, loadError) => { const session = get().sessions[id]; if (session) set({ sessions: { ...get().sessions, [id]: { ...session, loadError } } }); },
  bindLibrary:(id,libraryId,libraryRevision,name)=>{const session=get().sessions[id];if(session)set({sessions:{...get().sessions,[id]:{...session,image:{...session.image,libraryId,libraryRevision,name}}}});},
  unlinkLibrary:(libraryId)=>{const sessions={...get().sessions};for(const [id,session]of Object.entries(sessions)){if(session.image.libraryId===libraryId){const image={...session.image};delete image.libraryId;delete image.libraryRevision;sessions[id]={...session,image};}}set({sessions});},
  serialize: (id) => { const session = get().sessions[id]; if (!session) throw new Error('编辑会话不存在。'); validateDocument(session.document); return structuredClone(session.document); },
  select: (selectedId) => { const id=get().activeId; if(id && (selectedId===null||get().sessions[id].document.objects.some((o)=>o.id===selectedId))) set({sessions:{...get().sessions,[id]:{...get().sessions[id],selectedId}}}); },
  add: (annotation) => {
    const id=get().activeId; if(!id)return false; const session=get().sessions[id]; const document={...session.document,objects:[...session.document.objects,structuredClone(annotation)]};
    try { validateDocument(document); set({sessions:{...get().sessions,[id]:{...session,document,history:recordEdit(session.history,session.document.objects,document.objects),selectedId:annotation.id}},error:null}); return true; }
    catch(error){set({error:error instanceof Error?error.message:'无法添加标注。'});return false;}
  },
  update: (annotation) => {
    const id=get().activeId; if(!id)return false; const session=get().sessions[id]; if(!session.document.objects.some((o)=>o.id===annotation.id))return false;
    const document={...session.document,objects:session.document.objects.map((o)=>o.id===annotation.id?structuredClone(annotation):o)};
    try{validateDocument(document);set({sessions:{...get().sessions,[id]:{...session,document,history:recordEdit(session.history,session.document.objects,document.objects)}},error:null});return true;}catch(error){set({error:error instanceof Error?error.message:'无法更新标注。'});return false;}
  },
  removeSelected: () => {
    const id=get().activeId;if(!id)return;get().cancelGesture();const session=get().sessions[id];if(!session.selectedId)return;
    const objects=session.document.objects.filter((o)=>o.id!==session.selectedId);
    set({sessions:{...get().sessions,[id]:{...session,document:{...session.document,objects},history:recordEdit(session.history,session.document.objects,objects),selectedId:null}}});
  },
  undo: () => {
    get().cancelGesture(); const id=get().activeId;if(!id)return;const s=get().sessions[id];const objects=s.history.past.at(-1);if(!objects)return;
    set({sessions:{...get().sessions,[id]:{...s,document:{...s.document,objects},selectedId:null,history:boundHistory({past:s.history.past.slice(0,-1),future:[...s.history.future,s.document.objects]})}},error:null});
  },
  redo: () => {
    get().cancelGesture(); const id=get().activeId;if(!id)return;const s=get().sessions[id];const objects=s.history.future.at(-1);if(!objects)return;
    set({sessions:{...get().sessions,[id]:{...s,document:{...s.document,objects},selectedId:null,history:boundHistory({past:[...s.history.past,s.document.objects],future:s.history.future.slice(0,-1)})}},error:null});
  },
  setStyle: (patch) => {
    const style={...get().style,...patch}; if(!/^#[0-9a-f]{6}$/i.test(style.color)||!Number.isFinite(style.strokeWidth)||style.strokeWidth<1||style.strokeWidth>32||!Number.isFinite(style.fontSize)||style.fontSize<8||style.fontSize>200)return;
    if((style.mosaicSize!==undefined&&(!Number.isInteger(style.mosaicSize)||style.mosaicSize<2||style.mosaicSize>64))||(style.blurRadius!==undefined&&(!Number.isInteger(style.blurRadius)||style.blurRadius<1||style.blurRadius>64))||(style.numberSize!==undefined&&(!Number.isFinite(style.numberSize)||style.numberSize<16||style.numberSize>200)))return;
    set({style});const id=get().activeId;const session=id?get().sessions[id]:null; const selected=session?.document.objects.find((o)=>o.id===session.selectedId);
    if(selected) get().update({...selected,...(patch.color?{color:style.color}:{}),...(patch.strokeWidth!==undefined?{strokeWidth:style.strokeWidth}:{}),...(selected.type==='mosaic'&&patch.mosaicSize!==undefined?{strength:style.mosaicSize!}:{}),...(selected.type==='blur'&&patch.blurRadius!==undefined?{strength:style.blurRadius!}:{}),...(selected.type==='number'&&patch.numberSize!==undefined?{size:style.numberSize!}:{}),...(selected.type==='text'&&patch.fontSize!==undefined?{fontSize:style.fontSize}:{})});
  },
  beginDraw: (point) => {
    const id=get().activeId;if(id&&!get().gesture&&get().tool==='number'){const image=get().sessions[id].image;if(inImage(point,image.widthPx,image.heightPx)){const size=get().style.numberSize??36;get().add({...createAnnotation('number',point,get().style),type:'number',value:nextNumber(get().sessions[id].document.objects),size,x:point.x-size/2,y:point.y-size/2});}return;}if(!id||get().gesture||!['rectangle','ellipse','arrow','pen','mosaic','blur'].includes(get().tool))return;const image=get().sessions[id].image;if(!inImage(point,image.widthPx,image.heightPx))return;
    const annotation=createAnnotation(get().tool,point,get().style);get().select(null);set({gesture:{kind:'draw',imageId:id,start:point,original:annotation,draft:annotation}});
  },
  beginMove: (id,point,endpoint) => {
    const active=get().activeId;if(!active||get().tool!=='select'||get().gesture)return;const original=get().sessions[active].document.objects.find((o)=>o.id===id);if(!original)return;
    get().select(id);set({gesture:{kind:endpoint===undefined?'move':'endpoint',imageId:active,start:point,original,draft:original,endpoint}});
  },
  moveGesture: (point) => {
    const gesture=get().gesture;if(!gesture||get().activeId!==gesture.imageId||![point.x,point.y].every(Number.isFinite))return;let draft:Annotation;
    if(gesture.kind==='draw'){const image=get().sessions[gesture.imageId].image;const bounded=clampPoint(point,image.widthPx,image.heightPx);draft=drawAnnotation(gesture.original.type==='pen'?gesture.draft:gesture.original,gesture.start,bounded);}
    else if(gesture.kind==='endpoint')draft=moveArrowEndpoint(gesture.original,gesture.endpoint!,point);
    else draft=translateAnnotation(gesture.original,{x:point.x-gesture.start.x,y:point.y-gesture.start.y});
    set({gesture:{...gesture,draft}});
  },
  finishGesture: () => { const g=get().gesture;set({gesture:null});if(!g||g.imageId!==get().activeId||!nonEmpty(g.draft))return;if(g.kind==='draw')get().add(g.draft);else get().update(g.draft); },
  cancelGesture: () => set({gesture:null,cancelVersion:get().cancelVersion+1}),
}));
