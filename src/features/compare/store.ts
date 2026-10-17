import {create} from 'zustand';
import {comparisonFromLibrary,chooseComparisonFile,type LoadedComparison} from './api';
import {comparisonExtent,fitComparison,type CompareImage,type CompareMode} from './model';
import {zoomViewport} from '../editor/geometry';
import type {Viewport} from '../editor/types';
import type {NavigationView} from '../../app/ui-store';
import {errorMessage} from '../../shared/api/errors';
interface State {a:CompareImage|null;b:CompareImage|null;mode:CompareMode;split:number;opacity:number;viewport:Viewport;host:{width:number;height:number};busy:boolean;error:string|null;generation:number;returnView:NavigationView;enter:(view:NavigationView)=>void;close:()=>void;choose:(slot:'a'|'b')=>Promise<boolean>;loadPair:(ids:string[])=>Promise<boolean>;swap:()=>void;setMode:(mode:CompareMode)=>void;setSplit:(n:number)=>void;setOpacity:(n:number)=>void;resize:(width:number,height:number)=>void;fit:()=>void;zoom:(scale:number,x:number,y:number)=>void;pan:(x:number,y:number)=>void;failed:(slot:'a'|'b',id:string)=>void}
const initialViewport:Viewport={x:0,y:0,scale:1,mode:'fit'};
export function createCompareStore(){return create<State>((set,get)=>{
  const release=(image:CompareImage|null)=>{if(image)URL.revokeObjectURL(image.url);};
  const loaded=(data:LoadedComparison):CompareImage=>({...data,id:crypto.randomUUID(),url:URL.createObjectURL(data.blob)});
  const task=async(work:()=>Promise<{a?:LoadedComparison;b?:LoadedComparison}|null>)=>{
    if(get().busy)return false;const generation=get().generation;set({busy:true,error:null});const created:CompareImage[]=[];
    try{const result=await work();if(!result||get().generation!==generation)return false;
      const patch:Partial<State>={};for(const slot of ['a','b']as const)if(result[slot]){patch[slot]=loaded(result[slot]);created.push(patch[slot]);}
      for(const slot of ['a','b']as const)if(patch[slot])release(get()[slot]);set({...patch,viewport:{...get().viewport,mode:'fit'}});const host=get().host;if(host.width&&host.height)get().resize(host.width,host.height);return true;
    }catch(e){for(const image of created)release(image);if(get().generation===generation)set({error:errorMessage(e,'图片加载失败，已有对比图片已保留。')});return false;}finally{if(get().generation===generation)set({busy:false});}
  };
  return {a:null,b:null,mode:'side',split:50,opacity:0.5,viewport:initialViewport,host:{width:0,height:0},busy:false,error:null,generation:0,returnView:'screenshots',
    enter:(returnView)=>set({returnView:returnView==='compare'?'screenshots':returnView}),
    close:()=>{release(get().a);release(get().b);set({a:null,b:null,busy:false,error:null,generation:get().generation+1,host:{width:0,height:0},viewport:initialViewport});},
    choose:(slot)=>task(async()=>{const image=await chooseComparisonFile();return image?{[slot]:image}:null;}),
    loadPair:(ids)=>task(async()=>{if(ids.length!==2)throw new Error('请选择两张图片，按选择顺序放入 A / B。');const a=await comparisonFromLibrary(ids[0]),b=await comparisonFromLibrary(ids[1]);return {a,b};}),
    swap:()=>{if(!get().busy)set({a:get().b,b:get().a});},
    setMode:(mode)=>{if(['side','slider','overlay'].includes(mode))set({mode});},
    setSplit:(split)=>{if(Number.isFinite(split))set({split:Math.max(0,Math.min(100,split))});},
    setOpacity:(opacity)=>{if(Number.isFinite(opacity))set({opacity:Math.max(0,Math.min(1,opacity))});},
    resize:(width,height)=>{if(width<=0||height<=0)return;const s=get(),extent=comparisonExtent(s.a,s.b);set({host:{width,height},viewport:s.viewport.mode==='fit'||!s.host.width?fitComparison(extent.width,extent.height,width,height):{...s.viewport,x:s.viewport.x+(width-s.host.width)/2,y:s.viewport.y+(height-s.host.height)/2}});},
    fit:()=>{const s=get();set({viewport:{...s.viewport,mode:'fit'}});if(s.host.width)get().resize(s.host.width,s.host.height);},
    zoom:(scale,x,y)=>set({viewport:zoomViewport(get().viewport,scale,{x,y})}),
    pan:(x,y)=>{if(Number.isFinite(x)&&Number.isFinite(y))set({viewport:{...get().viewport,x,y,mode:'manual'}});},
    failed:(slot,id)=>{const image=get()[slot];if(image?.id===id)set({[slot]:{...image,broken:true},error:`图片 ${slot.toUpperCase()} 无法显示，请重新选择；另一张图片不受影响。`});},
  };
});}
export const useCompareStore=createCompareStore();
