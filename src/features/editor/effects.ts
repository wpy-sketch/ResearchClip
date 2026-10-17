import Konva from 'konva';
import {effectBounds} from './model';
import type {EffectAnnotation,Annotation} from './types';

// One original-pixel implementation for live editor, PNG, clipboard, pins and Markdown.
export function renderEffectPatch(source:CanvasImageSource,image:{widthPx:number;heightPx:number},a:EffectAnnotation){
  const b=effectBounds(a,image);if(!b)return null;
  const canvas=document.createElement('canvas');canvas.width=b.width;canvas.height=b.height;
  const processor=new Konva.Image({image:canvas});
  try{
    const context=canvas.getContext('2d',{willReadFrequently:true});if(!context)throw new Error('无法创建滤镜画布。');
    context.drawImage(source,b.x,b.y,b.width,b.height,0,0,b.width,b.height);
    const pixels=context.getImageData(0,0,b.width,b.height);
    const filter=a.type==='mosaic'?Konva.Filters.Pixelate:Konva.Filters.Blur;
    if(typeof filter!=='function')throw new Error('本机滤镜 API 不可用。');
    processor.pixelSize(a.strength);processor.blurRadius(a.strength);filter.call(processor,pixels);
    context.putImageData(pixels,0,0);
    return {canvas,x:b.x-a.x,y:b.y-a.y,width:b.width,height:b.height};
  }catch(error){canvas.width=canvas.height=0;throw error;}finally{processor.destroy();}
}
export function numberAttributes(a:Extract<Annotation,{type:'number'}>){
  const rgb=[1,3,5].map(i=>parseInt(a.color.slice(i,i+2),16));
  const fill=rgb[0]*0.299+rgb[1]*0.587+rgb[2]*0.114>150?'#000000':'#ffffff';
  return {circle:{x:a.size/2,y:a.size/2,radius:a.size/2,fill:a.color},text:{width:a.size,height:a.size,text:String(a.value),fontSize:a.size*(a.value>999?0.32:a.value>99?0.4:0.5),fill,fontFamily:'Arial, Microsoft YaHei',align:'center',verticalAlign:'middle'}};
}
