import Konva from 'konva';
import {renderEffectPatch,numberAttributes} from './effects';
import { parseDocument } from './serialization';
import type { Annotation, AnnotationDocument } from './types';

// Shared shape attributes keep text, arrows and line styles identical in editor/export.
export function shapeAttributes(a: Annotation): Record<string, unknown> {
  const stroke = { stroke: a.color, strokeWidth: a.strokeWidth };
  switch (a.type) {
    case 'rectangle': return { ...stroke, width: a.width, height: a.height, fill: 'rgba(0,0,0,0)' };
    case 'ellipse': return { ...stroke, x: a.width / 2, y: a.height / 2, radiusX: a.width / 2, radiusY: a.height / 2, fill: 'rgba(0,0,0,0)' };
    case 'arrow': return { ...stroke, points: a.points, fill: a.color, pointerLength: Math.max(10, a.strokeWidth * 3), pointerWidth: Math.max(10, a.strokeWidth * 3) };
    case 'pen': return { ...stroke, points: a.points, lineCap: 'round', lineJoin: 'round' };
    case 'number': return {};
    case 'mosaic': case 'blur': return {};
    case 'text': return { text: a.text, fontSize: a.fontSize, fill: a.color, fontFamily: 'Arial, Microsoft YaHei' };
  }
}
export async function renderComposite(source: Blob, input: AnnotationDocument): Promise<Blob> {
  const doc = parseDocument(input);
  await document.fonts.ready;
  const image = await createImageBitmap(source);
  let stage: Konva.Stage | undefined;const patches:HTMLCanvasElement[]=[];
  try {
    if (image.width !== doc.image.widthPx || image.height !== doc.image.heightPx) throw new Error('原图实际尺寸与标注不一致。');
    // No viewport, selection, Transformer or toolbar is passed into this renderer.
    stage = new Konva.Stage({ container: document.createElement('div'), width: image.width, height: image.height });
    const layer = new Konva.Layer({ listening: false });
    layer.getCanvas().setPixelRatio(1); layer.getHitCanvas().setPixelRatio(1);
    stage.add(layer); layer.add(new Konva.Image({ image, width: image.width, height: image.height }));
    // Effects modify original pixels first. Vector marks remain above all image effects.
    for(const a of doc.objects){if(a.type!=='mosaic'&&a.type!=='blur')continue;const patch=renderEffectPatch(image,doc.image,a);if(patch){patches.push(patch.canvas);const region=new Konva.Group({x:a.x+patch.x,y:a.y+patch.y,clipX:0,clipY:0,clipWidth:patch.width,clipHeight:patch.height});region.add(new Konva.Image({image:patch.canvas,width:patch.width,height:patch.height,globalCompositeOperation:'copy'}));layer.add(region);}}
    for (const a of doc.objects) {
      if(a.type==='mosaic'||a.type==='blur')continue;
      const group = new Konva.Group({ x: a.x, y: a.y });
      if(a.type==='number'){const n=numberAttributes(a);group.add(new Konva.Circle(n.circle),new Konva.Text(n.text));layer.add(group);continue;}
      const attrs = shapeAttributes(a);
      const shape = a.type === 'rectangle' ? new Konva.Rect(attrs) : a.type === 'ellipse' ? new Konva.Ellipse(attrs) : a.type === 'arrow' ? new Konva.Arrow(attrs) : a.type === 'pen' ? new Konva.Line(attrs) : new Konva.Text(attrs);
      group.add(shape); layer.add(group);
    }
    layer.draw();
    const canvas = stage.toCanvas({ pixelRatio: 1, x: 0, y: 0, width: image.width, height: image.height });
    return await new Promise<Blob>((resolve, reject) => canvas.toBlob((blob) => blob ? resolve(blob) : reject(new Error('PNG 合成失败，请重试。')), 'image/png'));
  } finally { stage?.destroy();for(const canvas of patches)canvas.width=canvas.height=0;image.close(); }
}
