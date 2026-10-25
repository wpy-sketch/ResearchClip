import {create} from 'zustand';
import {isTauri} from '@tauri-apps/api/core';
import {listProjects,listProjectDocuments,syncProjectDocument,type Project,type ProjectDocument} from './api';
import {errorMessage} from '../../shared/api/errors';
interface State {byToken:Record<string,ProjectDocument>;items:Project[];documents:ProjectDocument[];current:ProjectDocument|null;token:string|null;error:string|null;indexError:string|null;generation:number;refresh:(projectId?:string|null,unclassified?:boolean)=>Promise<void>;sync:(token:string,opened?:boolean,restoredFrom?:string|null,strict?:boolean)=>Promise<void>}
export const useProjectStore=create<State>((set,get)=>({byToken:{},items:[],documents:[],current:null,token:null,error:null,indexError:null,generation:0,
 refresh:async(projectId=null,unclassified=false)=>{if(!isTauri())return;const generation=get().generation+1;set({generation});try{const [items,documents]=await Promise.all([listProjects(),listProjectDocuments(projectId,unclassified)]);if(get().generation===generation)set({items,documents,error:null});}catch(e){if(get().generation===generation)set({error:errorMessage(e,'无法读取项目；已有数据未重置。')});}},
 sync:async(token,opened=false,restoredFrom=null,strict=false)=>{if(!isTauri())return;try{const current=await syncProjectDocument(token,opened,restoredFrom);set({current,token,byToken:{...get().byToken,[token]:current},indexError:null});}catch(e){set({indexError:errorMessage(e,'Markdown 文件已处理，但项目索引/历史保存失败，请重试登记。')});if(strict)throw e;}},
}));
