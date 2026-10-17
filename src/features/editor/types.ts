export interface Point { x: number; y: number }
export interface Viewport extends Point { scale: number; mode: 'fit' | 'manual' }
export type EditorTool = 'select' | 'pan' | 'rectangle' | 'ellipse' | 'arrow' | 'pen' | 'text' | 'mosaic' | 'blur' | 'number';
export interface AnnotationStyle { color: string; strokeWidth: number; fontSize: number; mosaicSize?:number; blurRadius?:number; numberSize?:number }
interface BaseAnnotation { id: string; x: number; y: number; color: string; strokeWidth: number }
export type EffectAnnotation = BaseAnnotation & { width:number; height:number; strength:number } & ({type:'mosaic'}|{type:'blur'});
export type Annotation =
  | EffectAnnotation
  | (BaseAnnotation & {type:'number';value:number;size:number})
  | (BaseAnnotation & { type: 'rectangle'; width: number; height: number })
  | (BaseAnnotation & { type: 'ellipse'; width: number; height: number })
  | (BaseAnnotation & { type: 'arrow'; points: number[] })
  | (BaseAnnotation & { type: 'pen'; points: number[] })
  | (BaseAnnotation & { type: 'text'; text: string; fontSize: number });
// Only this document is serializable. URLs, Blob, Konva nodes, selection and view are separate.
export interface AnnotationDocument { schemaVersion: 1; image: { widthPx: number; heightPx: number }; objects: Annotation[] }
export interface EditorImage { id: string; name: string; widthPx: number; heightPx: number; blob: Blob; url: string; libraryId?:string; libraryRevision?:number }
export interface EditHistory { past: Annotation[][]; future: Annotation[][] }
export interface EditorSession { image: EditorImage; document: AnnotationDocument; viewport: Viewport; selectedId: string | null; loadError: string | null; history: EditHistory; savedDocument:string }
