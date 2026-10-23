import {useState} from 'react';
import {ExternalLink} from 'lucide-react';
import {invoke} from '@tauri-apps/api/core';
import {documentDirty,useDocumentApi,useDocumentStore} from './store';
import {errorMessage} from '../../shared/api/errors';
export function ExternalOpenButton(){
  const documentApi=useDocumentApi();
 const [opening,setOpening]=useState(false),s=useDocumentStore();
 const open=async()=>{if(opening||s.phase!=='idle')return;setOpening(true);try{
   if((!s.token||documentDirty(s))&&!await s.save())return;
   const token=documentApi.getState().token;if(!token)return;
   await invoke('open_markdown_external',{token});
 }catch(e){documentApi.setState({error:errorMessage(e,'无法使用其他应用打开，请检查 Windows 文件关联。')});}finally{setOpening(false);}};
 return <button className="icon-button" disabled={opening||s.phase!=='idle'} title="使用其他应用打开（先保存，再由 Windows 选择应用）" aria-label="使用其他应用打开" onClick={()=>void open()}><ExternalLink size={15}/></button>;
}
