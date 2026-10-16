import {create} from 'zustand';
import {listTags} from '../../shared/api/library';
import type {Tag} from '../../shared/api/library';
import {isTauri} from '@tauri-apps/api/core';
import {errorMessage} from '../../shared/api/errors';
interface TagState {items:Tag[];loading:boolean;error:string|null;generation:number;refresh:()=>Promise<void>}
export const useTagStore=create<TagState>((set,get)=>({items:[],loading:false,error:null,generation:0,
  refresh:async()=>{if(!isTauri())return;const generation=get().generation+1;set({generation,loading:true,error:null});
    try{const items=await listTags();if(get().generation===generation)set({items,loading:false});}
    catch(error){if(get().generation===generation)set({loading:false,error:errorMessage(error,'无法读取标签，已有数据已保留。')});}
  },
}));
