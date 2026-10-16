import {create} from 'zustand';
import {exportKnowledgeMarkdown,validateExportIds} from '../../shared/api/markdown';
import type {MarkdownExportReport} from '../../shared/api/markdown';
import {useCardStore,isDirty} from '../notes/store';
import {errorMessage} from '../../shared/api/errors';
interface State {busy:boolean;result:MarkdownExportReport|null;error:string|null;cancelled:boolean;run:(ids:string[])=>Promise<void>;dismiss:()=>void}
export const useMarkdownExportStore=create<State>((set,get)=>({busy:false,result:null,error:null,cancelled:false,
  run:async(input)=>{
    if(get().busy)return;
    const ids=[...input];set({busy:true,result:null,error:null,cancelled:false});
    try{
      validateExportIds(ids);
      const pending=ids.filter(id=>{const session=useCardStore.getState().sessions[id];return session&&isDirty(session);});
      const saved=await Promise.all(pending.map(id=>useCardStore.getState().flush(id)));
      if(saved.some(success=>!success))throw new Error('所选卡片尚有保存失败的草稿，请先修正并保存，再导出。草稿已保留。');
      const result=await exportKnowledgeMarkdown(ids);set({result,cancelled:result===null});
    }catch(error){set({error:errorMessage(error,'Markdown导出失败，资料库未修改。')});}
    finally{set({busy:false});}
  },
  dismiss:()=>{if(!get().busy)set({result:null,error:null,cancelled:false});},
}));
