import { useEffect, useRef } from 'react';
import { Arrow, Circle, Ellipse, Group, Line, Rect, Text, Transformer } from 'react-konva';
import type Konva from 'konva';
import { resizeAnnotation } from './model';
import { useEditorStore } from './store';
import type { Annotation, Point } from './types';
import { shapeAttributes } from './export';

export function AnnotationNode({ annotation: a, selected, scale, selectable, point, editText, registerTransform }: {
  annotation: Annotation; selected: boolean; scale: number; selectable: boolean; point: () => Point | null;
  editText: (a: Annotation) => void; registerTransform: (active: boolean) => void;
}) {
  const group = useRef<Konva.Group>(null); const transformer = useRef<Konva.Transformer>(null);
  const cancelVersion = useEditorStore((s)=>s.cancelVersion); const canceled = useRef(false);
  useEffect(()=>{ if(selected && selectable && transformer.current && group.current) { transformer.current.nodes([group.current]); transformer.current.getLayer()?.batchDraw(); } },[selected,selectable,a.type]);
  useEffect(()=>{
    canceled.current=true;
    if(transformer.current?.isTransforming()) { transformer.current.stopTransform(); group.current?.position({x:a.x,y:a.y}); group.current?.scale({x:1,y:1}); registerTransform(false); group.current?.getLayer()?.batchDraw(); }
  },[cancelVersion,a.x,a.y,registerTransform]);
  const attrs = shapeAttributes(a), hit = Math.max(a.strokeWidth,12/scale);
  const shape = a.type==='rectangle'?<Rect {...attrs} hitStrokeWidth={hit} />
    :a.type==='ellipse'?<Ellipse {...attrs} radiusX={a.width/2} radiusY={a.height/2} hitStrokeWidth={hit} />
    :a.type==='arrow'?<Arrow {...attrs} points={a.points} hitStrokeWidth={hit} />
    :a.type==='pen'?<Line {...attrs} points={a.points} hitStrokeWidth={hit} />
    :<Text {...attrs} />;
  return <>
    <Group ref={group} id={a.id} x={a.x} y={a.y} listening={selectable}
      onPointerDown={(event)=>{if(event.evt.button!==0)return;event.cancelBubble=true;const at=point();if(at)useEditorStore.getState().beginMove(a.id,at);}}
      onDblClick={()=>{ if(a.type==='text') { useEditorStore.getState().cancelGesture();editText(a); } }}
      onDblTap={()=>{if(a.type==='text')editText(a);}}
      onTransformStart={()=>{canceled.current=false;registerTransform(true);}}
      onTransformEnd={()=>{
        const node=group.current;if(!node)return;registerTransform(false);
        const restore=()=>{node.position({x:a.x,y:a.y});node.scale({x:1,y:1});transformer.current?.forceUpdate();node.getLayer()?.batchDraw();};
        if(canceled.current){restore();return;}
        try {
          const changed=resizeAnnotation(a,node.x(),node.y(),node.scaleX(),node.scaleY());node.scale({x:1,y:1});
          if(!useEditorStore.getState().update(changed))restore();
        } catch(error) {
          restore();useEditorStore.setState({error:error instanceof Error?error.message:'无法调整标注尺寸。'});
        }
      }}>{shape}</Group>
    {selected && selectable && a.type!=='arrow' && <Transformer ref={transformer} rotateEnabled={false} flipEnabled={false} ignoreStroke keepRatio={a.type==='text'} enabledAnchors={a.type==='text'?['bottom-right']:undefined} anchorSize={8/scale} borderStroke="#aaa6e8" borderStrokeWidth={1/scale} anchorStroke="#aaa6e8" anchorStrokeWidth={1/scale} boundBoxFunc={(oldBox,newBox)=>Math.abs(newBox.width)<2||Math.abs(newBox.height)<2?oldBox:newBox} />}
    {selected && selectable && a.type==='arrow' && ([0,1] as const).map((index)=><Circle key={index} x={a.x+a.points[index*2]} y={a.y+a.points[index*2+1]} radius={5/scale} fill="#ffffff" stroke="#aaa6e8" strokeWidth={1.5/scale} onPointerDown={(event)=>{if(event.evt.button!==0)return;event.cancelBubble=true;const at=point();if(at)useEditorStore.getState().beginMove(a.id,at,index);}} />)}
  </>;
}
