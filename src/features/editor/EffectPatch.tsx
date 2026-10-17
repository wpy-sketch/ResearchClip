import {useLayoutEffect,useRef} from 'react';
import {Image,Group} from 'react-konva';
import type Konva from 'konva';
import {renderEffectPatch} from './effects';
import type {EffectAnnotation} from './types';
import {useEditorStore} from './store';
export function EffectPatch({source,image,annotation}:{source:HTMLImageElement;image:{widthPx:number;heightPx:number};annotation:EffectAnnotation}){
  const group=useRef<Konva.Group>(null),node=useRef<Konva.Image>(null);
  useLayoutEffect(()=>{
    let next:ReturnType<typeof renderEffectPatch>=null;const shape=node.current,container=group.current;
    try{
      next=renderEffectPatch(source,{widthPx:image.widthPx,heightPx:image.heightPx},annotation);
      if(shape&&container&&next){container.setAttrs({x:annotation.x+next.x,y:annotation.y+next.y,clipX:0,clipY:0,clipWidth:next.width,clipHeight:next.height});shape.setAttrs({image:next.canvas,width:next.width,height:next.height,visible:true});}
      else shape?.hide();
      shape?.getLayer()?.batchDraw();
    }catch{shape?.hide();useEditorStore.setState({error:'滤镜预览失败，请缩小区域后重试。导出将重新校验并应用滤镜。'});}
    return()=>{shape?.image(undefined);if(next)next.canvas.width=next.canvas.height=0;};
  },[source,image.widthPx,image.heightPx,annotation]);
  return <Group ref={group} listening={false}><Image ref={node} image={undefined} globalCompositeOperation="copy" listening={false}/></Group>;
}
