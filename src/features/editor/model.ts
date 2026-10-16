import { clampPoint, validateImageSize } from './geometry';
import type { Annotation, AnnotationDocument, AnnotationStyle, EditorTool, Point } from './types';
export const MAX_OBJECTS = 500; export const MAX_POINTS = 5000;
export function createAnnotation(tool: EditorTool, point: Point, style: AnnotationStyle, id: string = crypto.randomUUID()): Annotation {
  const base = { id, x: point.x, y: point.y, color: style.color, strokeWidth: style.strokeWidth };
  if (tool === 'rectangle' || tool === 'ellipse') return { ...base, type: tool, width: 0, height: 0 };
  if (tool === 'arrow' || tool === 'pen') return { ...base, type: tool, points: [0,0,0,0] };
  if (tool === 'text') return { ...base, type: tool, text: '', fontSize: style.fontSize };
  throw new Error('该工具不创建标注对象。');
}
export function drawAnnotation(original: Annotation, start: Point, current: Point): Annotation {
  if (original.type === 'rectangle' || original.type === 'ellipse') return { ...original, x: Math.min(start.x,current.x), y: Math.min(start.y,current.y), width: Math.abs(current.x-start.x), height: Math.abs(current.y-start.y) };
  if (original.type === 'arrow') return { ...original, points: [0,0,current.x-start.x,current.y-start.y] };
  if (original.type === 'pen') {
    const x=current.x-original.x, y=current.y-original.y; const points=original.points;
    if (points.length >= MAX_POINTS*2 || Math.hypot(x-points[points.length-2],y-points[points.length-1]) < 0.5) return original;
    return { ...original, points: [...points,x,y] };
  } return original;
}
export function translateAnnotation(annotation: Annotation, delta: Point): Annotation { return { ...annotation, x: annotation.x+delta.x, y: annotation.y+delta.y }; }
export function resizeAnnotation(annotation: Annotation, x: number, y: number, scaleX: number, scaleY: number): Annotation {
  if (![x,y,scaleX,scaleY].every(Number.isFinite) || scaleX <= 0 || scaleY <= 0) throw new Error('标注缩放值无效。');
  if (annotation.type === 'rectangle' || annotation.type === 'ellipse') return { ...annotation,x,y,width:annotation.width*scaleX,height:annotation.height*scaleY };
  if (annotation.type === 'arrow' || annotation.type === 'pen') return { ...annotation,x,y,points:annotation.points.map((n,i)=>n*(i%2?scaleY:scaleX)) };
  return { ...annotation,x,y,fontSize:Math.min(200,Math.max(8,annotation.fontSize*scaleY)) };
}
export function moveArrowEndpoint(annotation: Annotation, index: 0 | 1, point: Point): Annotation {
  if (annotation.type !== 'arrow') throw new Error('该对象没有箭头端点。');
  if (index === 1) return { ...annotation, points: [annotation.points[0],annotation.points[1],point.x-annotation.x,point.y-annotation.y] };
  const end = { x:annotation.x+annotation.points[2],y:annotation.y+annotation.points[3] };
  return { ...annotation,x:point.x,y:point.y,points:[0,0,end.x-point.x,end.y-point.y] };
}
export function nonEmpty(annotation: Annotation): boolean {
  if (annotation.type === 'rectangle' || annotation.type === 'ellipse') return annotation.width >= 1 && annotation.height >= 1;
  if (annotation.type === 'text') return typeof annotation.text === 'string' && annotation.text.trim().length > 0;
  return (annotation.type==='arrow'||annotation.type==='pen') && Array.isArray(annotation.points) && annotation.points.length >= 4 && annotation.points.some((v,i)=>Math.abs(v-annotation.points[i%2])>=1);
}
export function validateAnnotation(value: Annotation) {
  if (!value || typeof value.id !== 'string' || !/^[\w-]{1,128}$/.test(value.id) || ![value.x,value.y,value.strokeWidth].every(Number.isFinite)
    || Math.abs(value.x)>1_000_000 || Math.abs(value.y)>1_000_000 || typeof value.color !== 'string' || !/^#[0-9a-f]{6}$/i.test(value.color) || value.strokeWidth < 1 || value.strokeWidth > 32 || !nonEmpty(value)) throw new Error('标注数据无效。');
  if (value.type === 'rectangle' || value.type === 'ellipse') { if (![value.width,value.height].every((v)=>Number.isFinite(v)&&v>=1&&v<=1_000_000)) throw new Error('标注尺寸无效。'); }
  else if (value.type === 'arrow' || value.type === 'pen') { if (!Array.isArray(value.points) || value.points.length % 2 !== 0 || value.points.length > MAX_POINTS*2 || (value.type === 'arrow' && value.points.length !== 4) || !value.points.every((v)=>Number.isFinite(v)&&Math.abs(v)<=1_000_000)) throw new Error('标注路径无效。'); }
  else if (value.type === 'text') { if (value.text.length>2000 || !Number.isFinite(value.fontSize) || value.fontSize<8 || value.fontSize>200) throw new Error('标注文字无效。'); }
  else throw new Error('标注类型不支持。');
}
export function validateDocument(value: AnnotationDocument) {
  if (value.schemaVersion !== 1) throw new Error('标注版本不支持。'); validateImageSize(value.image.widthPx,value.image.heightPx);
  if (!Array.isArray(value.objects) || value.objects.length > MAX_OBJECTS) throw new Error('标注对象超过当前上限。');
  const ids = new Set(); let points = 0;
  for (const item of value.objects) { validateAnnotation(item); if (ids.has(item.id)) throw new Error('标注 ID 重复。'); ids.add(item.id); if (item.type==='pen'||item.type==='arrow') points += item.points.length / 2; }
  if (points>50000) throw new Error('标注路径总量超过当前上限。');
}
export function inImage(point: Point, width: number, height: number) { return Number.isFinite(point.x) && Number.isFinite(point.y) && point.x>=0 && point.y>=0 && point.x<=width && point.y<=height; }
export { clampPoint };
