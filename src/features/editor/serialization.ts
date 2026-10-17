import { validateDocument } from './model';
import type { Annotation, AnnotationDocument } from './types';
export const MAX_DOCUMENT_BYTES = 4 * 1024 * 1024;
export interface EditorProject { format: 'ResearchClip'; schemaVersion: 1; originalImage: string; document: AnnotationDocument }
const object = (value: unknown): value is Record<string, unknown> => Boolean(value) && typeof value === 'object' && !Array.isArray(value);
export function parseDocument(value: unknown): AnnotationDocument {
  if (!object(value) || value.schemaVersion !== 1) throw new Error('标注版本不支持，现有图片和项目未改动。');
  if (!object(value.image) || !Array.isArray(value.objects)) throw new Error('标注文件结构无效。');
  if (value.objects.length > 500) throw new Error('标注对象超过当前上限。');
  const objects = value.objects.map((item) => {
    if (!object(item)) throw new Error('标注对象无效。');
    const base = { id: item.id, x: item.x, y: item.y, color: item.color, strokeWidth: item.strokeWidth };
    if (item.type === 'rectangle' || item.type === 'ellipse') return { ...base, type: item.type, width: item.width, height: item.height };
    if(item.type==='mosaic'||item.type==='blur')return {...base,type:item.type,width:item.width,height:item.height,strength:item.strength};
    if(item.type==='number')return {...base,type:item.type,value:item.value,size:item.size};
    if (item.type === 'arrow' || item.type === 'pen') return { ...base, type: item.type, points: Array.isArray(item.points) ? [...item.points] : item.points };
    if (item.type === 'text') return { ...base, type: item.type, text: item.text, fontSize: item.fontSize };
    throw new Error('标注类型不支持。');
  }) as Annotation[];
  const result = { schemaVersion: 1 as const, image: { widthPx: value.image.widthPx, heightPx: value.image.heightPx }, objects } as AnnotationDocument;
  validateDocument(result);
  if (new TextEncoder().encode(JSON.stringify(result)).length > MAX_DOCUMENT_BYTES) throw new Error('标注文件超过大小上限。');
  return result;
}
export function parseProject(value: unknown): EditorProject {
  if (!object(value) || value.format !== 'ResearchClip' || value.schemaVersion !== 1) throw new Error('项目版本不支持，文件未改动。');
  if (typeof value.originalImage !== 'string' || !/^original-[a-zA-Z0-9-]{1,100}\.png$/.test(value.originalImage)) throw new Error('项目原图引用无效。');
  return { format: 'ResearchClip', schemaVersion: 1, originalImage: value.originalImage, document: parseDocument(value.document) };
}
// IPC envelope: u32 little endian JSON length, UTF-8 JSON, then original/composite PNG.
export async function packImage(blob: Blob, document: AnnotationDocument): Promise<ArrayBuffer> {
  const json = new TextEncoder().encode(JSON.stringify({ document: parseDocument(document) }));
  const png = await blob.arrayBuffer();
  if (!png.byteLength || png.byteLength > 16_777_216 * 4 + 1_048_576) throw new Error('PNG 超过大小上限。');
  const result = new ArrayBuffer(4 + json.length + png.byteLength);
  new DataView(result).setUint32(0, json.length, true);
  new Uint8Array(result, 4, json.length).set(json);
  new Uint8Array(result, 4 + json.length).set(new Uint8Array(png));
  return result;
}
export function unpackProject(buffer: ArrayBuffer): { project: EditorProject; blob: Blob } | null {
  if (buffer.byteLength === 0) return null;
  if (buffer.byteLength < 4 || buffer.byteLength > MAX_DOCUMENT_BYTES + 16_777_216 * 4 + 1_048_580) throw new Error('项目数据大小无效。');
  const length = new DataView(buffer).getUint32(0, true);
  if (length < 1 || length > MAX_DOCUMENT_BYTES || length + 4 >= buffer.byteLength) throw new Error('项目数据不完整。');
  const project = parseProject(JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(new Uint8Array(buffer, 4, length))));
  return { project, blob: new Blob([buffer.slice(4 + length)], { type: 'image/png' }) };
}
