import {useEffect} from 'react';
import {isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {Globe,FileText,X} from 'lucide-react';
import {useClipperStore} from './store';
import {useDocumentStore} from '../document/store';
import {useProjectStore} from '../projects/store';
import {useUiStore} from '../../app/ui-store';
import {useLibraryStore} from '../library/store';
export function ClipperNotice(){
 const notice=useClipperStore(s=>s.notice),error=useClipperStore(s=>s.error);
 useEffect(()=>{
  if(!isTauri())return;let active=true,stop:(()=>void)|undefined;
  void listen('clipper-saved',event=>{if(active){useClipperStore.getState().receive(event.payload);const f=useLibraryStore.getState();void useProjectStore.getState().refresh(f.projectId??null,Boolean(f.unclassified));}}).then(unlisten=>{if(active)stop=unlisten;else unlisten();}).catch(()=>{if(active)useClipperStore.setState({error:'网页剪藏通知监听失败，请在项目文档中查看已保存结果。'});});
  return()=>{active=false;stop?.();};
 },[]);
 if(!notice)return error?<div className="error-banner" role="alert">{error}</div>:null;
 return <div className="clipper-notice" role="status"><Globe size={17}/><div><strong>{notice.duplicate?'剪藏已存在':'网页已保存'}</strong><span>{notice.title}</span></div><button className="secondary-button" onClick={()=>void useDocumentStore.getState().openProject(notice.documentId)}><FileText size={15}/>打开 Markdown</button><button className="icon-button" aria-label="关闭剪藏通知" onClick={()=>useClipperStore.getState().dismiss()}><X size={16}/></button></div>;
}
export function ClipperSettings(){
 const {status,busy,error,connect,refresh}=useClipperStore();
 useEffect(()=>{void refresh();},[refresh]);
 return <section className="settings-section clipper-settings" aria-labelledby="clipper-title"><div className="settings-row"><div><h3 id="clipper-title"><Globe size={17}/>浏览器网页剪藏</h3><p className="settings-description">只接收你在Chrome / Edge扩展中主动保存的内容。每次启动后按需启用。</p></div><span className="subtle-badge">{status?.enabled?'本次会话已启用':'未启用'}</span></div><ol className="clipper-steps"><li>下载并解压本版本扩展包，按附带说明注册Host、加载扩展。</li><li>启用下方连接；在浏览器选择方式、预览、项目，再保存。</li><li>保存成功后点击桌面通知“打开 Markdown”，或在项目文档中查看。</li></ol><p className="text-xs text-muted select-text break-all">本地加载扩展ID：{status?.extensionId??'请在桌面应用中查看'}</p><div className="mt-4 flex flex-wrap gap-2"><button className="secondary-button" disabled={!isTauri()||busy} onClick={()=>void connect(!status?.enabled)}>{busy?'正在处理…':status?.enabled?'停止网页剪藏连接':'启用网页剪藏连接'}</button><button className="secondary-button" onClick={()=>useUiStore.getState().openGuide()}>查看安装与使用指南</button></div><p className="mt-3 text-xs text-muted">不开放HTTP写入或数据库访问。图片需明确允许，无法匿名获取时保留来源。停止连接不会删除已保存文档；已经接收的请求可能继续完成。</p>{error&&<p className="error-banner mt-3" role="alert">{error}</p>}</section>;
}
