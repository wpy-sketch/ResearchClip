import {create} from 'zustand';
import type {SelectionItem} from './api';
export function chooseItems(selected:SelectionItem[],ordered:SelectionItem[],id:string,anchor:string|null,shift:boolean):SelectionItem[]{
 const index=ordered.findIndex(i=>i.id===id);if(index<0)return selected;
 if(shift&&anchor){const start=ordered.findIndex(i=>i.id===anchor);if(start>=0){const next=new Map(selected.map(i=>[i.id,i]));for(const i of ordered.slice(Math.min(start,index),Math.max(start,index)+1))next.set(i.id,{id:i.id,revision:i.revision});return [...next.values()];}}
 return selected.some(i=>i.id===id)?selected.filter(i=>i.id!==id):[...selected,{id:ordered[index].id,revision:ordered[index].revision}];
}
interface State {items:SelectionItem[];anchor:string|null;choose:(ordered:SelectionItem[],id:string,shift?:boolean)=>void;add:(items:SelectionItem[])=>void;clear:()=>void}
export const useSelectionStore=create<State>((set,get)=>({items:[],anchor:null,
 choose:(ordered,id,shift=false)=>{const s=get();set({items:chooseItems(s.items,ordered,id,s.anchor,shift),anchor:shift&&s.anchor?s.anchor:id});},
 add:(items)=>{const map=new Map(get().items.map(i=>[i.id,i]));for(const item of items)map.set(item.id,{id:item.id,revision:item.revision});if(map.size>5000)throw new Error('最多选择5000张图片，请缩小范围。');set({items:[...map.values()]});},
 clear:()=>set({items:[],anchor:null}),
}));
