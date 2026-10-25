import {create} from 'zustand';
import {Actions,DockLocation,Model,GroupAction,type Action} from 'flexlayout-react';
import {allDocumentStores,createDocumentStore,documentDirty,focusDocumentStore,retainDocumentStore,releaseDocumentStore,type DocumentStore,type DocumentAction} from '../document/store';
import {useProjectStore} from '../projects/store';
import {useEditorStore} from '../editor/store';
import {useUiStore} from '../../app/ui-store';
import {openLibraryEditor,saveEditorToLibrary} from '../../app/library-actions';
import {WorkspacePersistence} from './persistence';
import {defaultDock,projectDock,validWorkspaceId,MAX_MARKDOWN,MAX_PANELS,type PanelRecord,type WorkspaceLayout,validateLayout} from './layout';
import type {EditorSession,PanelLifecycle} from './registry';
interface Snapshot {ready:boolean;revision:number;workspaceId:string;explorer:boolean;busy:boolean;error:string|null;notice:string|null;closingImage:string|null}
interface DocumentBinding {store:DocumentStore;request:ReturnType<DocumentStore['getState']>['request'];resolve:ReturnType<DocumentStore['getState']>['resolvePending'];stop:()=>void}
const basePanel=():PanelRecord=>({id:'workbench',kind:'workbench',title:'资料工作台',resourceId:null,hidden:false});
/** Owns UI layout and lifecycle only. Text/PNG content stays in existing stores and native services. */
export class WorkspaceManager {
 readonly state=create<Snapshot>(()=>({ready:false,revision:0,workspaceId:'default',explorer:true,busy:false,error:null,notice:null,closingImage:null}));
 readonly sessions=new Map<string,EditorSession>();
 readonly documents=new Map<string,DocumentBinding>();
 private bindings=new Map<DocumentStore,DocumentBinding>();
 private models=new Map<string,Model>();private saved=new Map<string,WorkspaceLayout>();private explorers=new Map<string,boolean>();private blocked=new Set<string>();private timers=new Map<string,ReturnType<typeof setTimeout>>();
 private persistence:WorkspacePersistence|null=null;private started=false;private routing=false;private selecting=false;private stops:Array<()=>void>=[];
 get workspaceId(){return this.state.getState().workspaceId;}
 get model(){return this.ensure(this.workspaceId);}
 private notify(patch:Partial<Snapshot>={}){this.state.setState(s=>({...patch,revision:s.revision+1}));}
 records(id=this.workspaceId){return [basePanel(),...[...this.sessions.values()].filter(s=>s.workspaceId===id).map(s=>s.panel)];}
 initialize(storage:Pick<Storage,'getItem'|'setItem'>){if(this.started)return;this.started=true;this.persistence=new WorkspacePersistence(storage);this.ensure('default');for(const store of allDocumentStores())this.bind(store);if(!allDocumentStores().length){const entry=createDocumentStore();this.bind(entry);focusDocumentStore(entry);}
  this.stops.push(useEditorStore.subscribe((now,old)=>{for(const [id,session] of this.sessions)if(session.panel.kind==='image'&&!now.sessions[id.slice(6)])this.remove(id);if(now.activeId&&(now.activeId!==old.activeId||!this.sessions.has('image:'+now.activeId))&&useUiStore.getState().activeView==='editor')this.showImage(now.activeId);}));
  this.stops.push(useUiStore.subscribe((now,old)=>{if(now.activeView!==old.activeView||now.navigationSerial!==old.navigationSerial){if(now.activeView==='editor'){const id=useEditorStore.getState().activeId;if(id)this.showImage(id);}else this.select('workbench');}}));
  this.notify({ready:true,explorer:this.explorers.get('default')??true});
 }
 private ensure(id:string){let model=this.models.get(id);if(model)return model;let saved:WorkspaceLayout|null=null;
  try{saved=this.persistence?.read(id)??null;}catch(e){this.blocked.add(id);this.notify({error:this.message(e)});}
  if(saved){this.saved.set(id,saved);this.explorers.set(id,saved.explorer);for(const panel of saved.panels)if(panel.kind==='terminal')this.addTerminal(id,panel.hidden,false);}
  const records=this.records(id);model=Model.fromJson(saved?projectDock(saved,records):defaultDock(records));this.models.set(id,model);return model;
 }
 private message(e:unknown){return e instanceof Error?e.message:'工作区操作失败；内容已保留。';}
 private fail(e:unknown){this.notify({error:this.message(e)});}
 private bind(store:DocumentStore){const known=this.bindings.get(store);if(known)return known;
  const original=store.getState(),binding:DocumentBinding={store,request:original.request,resolve:original.resolvePending,stop:()=>{}};
  this.bindings.set(store,binding);retainDocumentStore(store);
  store.setState({request:action=>this.route(store,action),resolvePending:async choice=>{await binding.resolve(choice);const row=[...this.documents].find(([,b])=>b===binding);if(row&&!store.getState().document)this.finishDocument(row[0]);}});
  binding.stop=store.subscribe(()=>this.updateDocument(store));return binding;
 }
 private updateDocument(store:DocumentStore){const found=[...this.documents].find(([,v])=>v.store===store);if(!found)return;const [id]=found,s=store.getState(),record=this.sessions.get(id);if(!record)return;
  const current=s.token?useProjectStore.getState().byToken?.[s.token]:null,title=(s.fileName??'untitled.md').slice(0,155)+(documentDirty(s)?' ●':'');
  if(record.panel.title!==title||record.panel.resourceId!==(current?.id??null)){record.panel={...record.panel,title,resourceId:current?.id??null};const model=this.models.get(record.workspaceId);if(model?.getNodeById(id))model.doAction(Actions.renameTab(id,title));this.notify();this.schedule(record.workspaceId);}
 }
 private async route(source:DocumentStore,action:DocumentAction){const binding=this.bind(source);
  if(action==='close'){const row=[...this.documents].find(([,b])=>b===binding);if(row)this.focus(row[0]);await binding.request(action);if(row&&!source.getState().document)this.finishDocument(row[0]);return;}
  if(this.routing){source.setState({error:'已有文档正在打开，请等待或取消系统对话框。'});return;}
  const sourceState=source.getState();if(sourceState.phase!=='idle'||sourceState.pending){source.setState({error:'当前文档正在处理，请先完成操作。'});return;}
  if(action==='project'&&sourceState.projectOpenId){const existing=[...this.sessions.values()].find(s=>s.panel.kind==='markdown'&&s.panel.resourceId===sourceState.projectOpenId);if(existing){this.focus(existing.panel.id);if(sourceState.projectLinkNavigation)this.documents.get(existing.panel.id)?.store.getState().jumpToSource(sourceState.projectOpenOffset);return;}}
  if(this.documents.size>=MAX_MARKDOWN||this.records().length>=MAX_PANELS){source.setState({error:'最多同时打开3份 Markdown（包含隐藏和其他工作区），为另存/重连预留原生文件授权。请先关闭一份。'});this.notify({error:source.getState().error});return;}
  this.routing=true;this.notify({busy:true,error:null});const workspaceId=this.workspaceId,store=sourceState.document?createDocumentStore():source,target=this.bind(store);
  try{store.setState({projectOpenId:sourceState.projectOpenId,projectOpenOffset:sourceState.projectOpenOffset,projectLinkNavigation:sourceState.projectLinkNavigation,templateSeed:sourceState.templateSeed});await target.request(action);const s=store.getState();
   if(!s.document){if(s.error)this.fail(Error(s.error));if(store!==source)this.unbind(store);return;}
   const resource=s.token?useProjectStore.getState().byToken?.[s.token]?.id:null;
   const duplicate=[...this.documents].find(([id,b])=>this.sessions.get(id)?.panel.resourceId&&this.sessions.get(id)?.panel.resourceId===resource||s.path&&b.store.getState().path?.toLocaleLowerCase()===s.path.toLocaleLowerCase());
   if(duplicate){await target.request('close');if(store!==source)this.unbind(store);this.focus(duplicate[0]);return;}
   const id='markdown:'+s.document.id,panel:PanelRecord={id,kind:'markdown',title:s.fileName??'untitled.md',resourceId:resource??null,hidden:false};
   const lifecycle={dirty:()=>documentDirty(store.getState()),busy:()=>store.getState().phase!=='idle',save:()=>store.getState().save(),dispose:async()=>{await target.request('close');}};
   this.sessions.set(id,{panel,workspaceId,lifecycle});this.documents.set(id,target);this.add(id,'right');this.updateDocument(store);this.focus(id);this.notify({notice:'文档在独立标签页打开；其他草稿保留。'});
  }catch(e){this.fail(e);if(!store.getState().document&&store!==source)this.unbind(store);}finally{this.routing=false;this.notify({busy:false});}
 }
 private unbind(store:DocumentStore){const b=this.bindings.get(store);if(!b)return;b.stop();store.setState({request:b.request,resolvePending:b.resolve});this.bindings.delete(store);releaseDocumentStore(store);}
 private finishDocument(id:string){const b=this.documents.get(id);if(!b)return;const error=b.store.getState().error;this.documents.delete(id);this.unbind(b.store);this.remove(id);if(!allDocumentStores().length){const entry=createDocumentStore();this.bind(entry);focusDocumentStore(entry);}if(error)this.fail(Error(error));}
 private add(id:string,direction:'right'|'bottom'|'center'='center'){const session=this.sessions.get(id);if(!session)return;const model=this.ensure(session.workspaceId);if(model.getNodeById(id)){model.doAction(Actions.selectTab(id));return;}
  const target=model.getActiveTabset()??model.getNodeById('workbench')?.getParent();if(!target)return;
  // New documents default beside the workbench; subsequent documents join the active tab group.
  const onlyWorkbench=model.getNodeById('workbench')&&this.records(session.workspaceId).filter(p=>!p.hidden&&model.getNodeById(p.id)).length===1;
  model.doAction(Actions.addTab({type:'tab',id,name:session.panel.title,component:'workspace'},target.getId(),session.panel.kind==='terminal'?DockLocation.BOTTOM:onlyWorkbench?direction==='bottom'?DockLocation.BOTTOM:DockLocation.RIGHT:DockLocation.CENTER,-1));this.notify();this.schedule(session.workspaceId);
 }
 openEditor(panel:PanelRecord,lifecycle:PanelLifecycle){if(this.sessions.has(panel.id)||this.records().length>=MAX_PANELS)throw Error('面板已打开或工作区已达12个面板上限。');this.sessions.set(panel.id,{panel,workspaceId:this.workspaceId,lifecycle});this.add(panel.id,'right');this.focus(panel.id);this.notify();}
 updateEditor(id:string,patch:Partial<Pick<PanelRecord,'title'|'resourceId'>>){const session=this.sessions.get(id);if(!session)return;session.panel={...session.panel,...patch};const model=this.models.get(session.workspaceId);if(patch.title&&model?.getNodeById(id))model.doAction(Actions.renameTab(id,patch.title));this.notify();this.schedule(session.workspaceId);}
 async flushReadOnlyEditors(){for(const session of this.sessions.values())if(session.panel.kind==='pdf'&&!await session.lifecycle.save())throw Error('PDF阅读位置未保存，退出已取消。');}
 showImage(imageId:string){const image=useEditorStore.getState().sessions[imageId];if(!image)return;const id='image:'+imageId;
  if(this.sessions.has(id)){this.focus(id);return;}if(this.records().length>=MAX_PANELS){this.fail(Error('工作区面板已达12个上限，请先关闭面板。'));return;}
  const panel:PanelRecord={id,kind:'image',title:image.image.name.slice(0,160),resourceId:image.image.libraryId??null,hidden:false};
  this.sessions.set(id,{panel,workspaceId:this.workspaceId,lifecycle:{dirty:()=>{const s=useEditorStore.getState().sessions[imageId];return !!s&&JSON.stringify(s.document)!==s.savedDocument;},busy:()=>useEditorStore.getState().pendingIo>0||useEditorStore.getState().gesture?.imageId===imageId,save:async()=>{const s=useEditorStore.getState().sessions[imageId];if(!s)return false;const finish=useEditorStore.getState().beginIo();try{await saveEditorToLibrary(s);return true;}finally{finish();}},dispose:async()=>{useEditorStore.getState().close(imageId,true);}}});this.add(id);this.focus(id);
 }
 focus(id:string){const session=this.sessions.get(id);if(session&&session.workspaceId!==this.workspaceId)this.switchWorkspace(session.workspaceId);if(session?.panel.hidden){session.panel.hidden=false;this.add(id);}
  if(session?.panel.kind==='markdown'){const b=this.documents.get(id);if(b)focusDocumentStore(b.store);}if(session?.panel.kind==='image'){const imageId=id.slice(6);if(useEditorStore.getState().activeId!==imageId)useEditorStore.getState().activate(imageId);}
  this.select(id);
 }
 private select(id:string){if(this.selecting)return;const model=this.models.get(this.workspaceId);if(model?.getNodeById(id)){this.selecting=true;try{model.doAction(Actions.selectTab(id));}finally{this.selecting=false;}}}
 onAction(action:Action){if(action.type===Actions.DELETE_TABSET||action instanceof GroupAction){this.notify({notice:'请逐个关闭面板，以检查每份草稿。'});return undefined;}if(action.type===Actions.DELETE_TAB){void this.close(String(action.data.node));return undefined;}if(action.type===Actions.SELECT_TAB)this.focus(String(action.data.tabNode));return action;}
 changed(){this.schedule(this.workspaceId);}
 split(vertical:boolean){const model=this.model,active=model.getActiveTabset()?.getSelectedNode();if(!active)return;const other=this.records().find(p=>p.id!==active.getId()&&!p.hidden&&model.getNodeById(p.id));if(!other){this.notify({notice:'请先打开另一个标签页，再进行拆分。'});return;}const target=model.getNodeById(other.id)!.getParent();if(!target)return;model.doAction(Actions.moveNode(active.getId(),target.getId(),vertical?DockLocation.BOTTOM:DockLocation.RIGHT,-1));this.notify();this.schedule(this.workspaceId);}
 async close(id:string){const s=this.sessions.get(id);if(!s)return;if(s.lifecycle.busy()){this.fail(Error('面板正在处理或绘制，请完成后关闭。'));return;}this.focus(id);
  if(s.panel.kind==='markdown'){await this.documents.get(id)!.store.getState().request('close');return;}
  if(s.lifecycle.dirty()){this.notify({closingImage:id});return;}try{await s.lifecycle.dispose();this.remove(id);}catch(e){this.fail(e);if(s.panel.kind==='pdf')(await import('../pdf/PdfViewerSession')).pdfSessions.get(id)?.report(e);}
 }
 async resolveImage(choice:'save'|'discard'|'cancel'){const id=this.state.getState().closingImage,s=id?this.sessions.get(id):null;if(!id||!s)return;if(choice==='cancel'){this.notify({closingImage:null});return;}this.notify({busy:true,error:null});try{if(choice==='save'&&!await s.lifecycle.save())return;await s.lifecycle.dispose();this.remove(id);this.notify({closingImage:null});}catch(e){this.fail(e);}finally{this.notify({busy:false});}}
 hide(id:string){const s=this.sessions.get(id);if(!s)return;if(s.lifecycle.busy()){this.fail(Error('面板正在处理，暂不能隐藏。'));return;}s.panel.hidden=true;const model=this.models.get(s.workspaceId);if(model?.getNodeById(id))model.doAction(Actions.deleteTab(id));this.notify({notice:'面板已隐藏，草稿与撤销历史仍保留。'});this.schedule(s.workspaceId);}
 private remove(id:string){const s=this.sessions.get(id);if(!s)return;this.sessions.delete(id);const model=this.models.get(s.workspaceId);if(model?.getNodeById(id))model.doAction(Actions.deleteTab(id));this.notify();this.schedule(s.workspaceId);}
 private addTerminal(workspaceId:string,hidden=false,insert=true){const id='terminal:'+workspaceId;if(this.sessions.has(id))return;this.sessions.set(id,{panel:{id,kind:'terminal',title:'终端区域 · 尚未连接',resourceId:null,hidden},workspaceId,lifecycle:{dirty:()=>false,busy:()=>false,save:async()=>true,dispose:async()=>{}}});if(insert&&!hidden)this.add(id,'bottom');}
 toggleTerminal(){const s=[...this.sessions.values()].find(s=>s.workspaceId===this.workspaceId&&s.panel.kind==='terminal');if(!s){if(this.records().length>=MAX_PANELS){this.fail(Error('工作区面板已达12个上限，请先关闭面板。'));return;}this.addTerminal(this.workspaceId);return;}if(s.panel.hidden)this.focus(s.panel.id);else this.hide(s.panel.id);}
 toggleExplorer(){const explorer=!this.state.getState().explorer;this.explorers.set(this.workspaceId,explorer);this.notify({explorer});this.schedule(this.workspaceId);}
 switchWorkspace(id:string){if(!validWorkspaceId(id)||this.routing||[...this.sessions.values()].some(s=>s.lifecycle.busy())){this.fail(Error('工作区标识无效或仍有在途操作，请稍后切换。'));return;}this.save(this.workspaceId,false);this.ensure(id);this.notify({workspaceId:id,explorer:this.explorers.get(id)??true,notice:'已切换布局；其他工作区草稿仍在内存中。'});}
 private snapshot(id:string):WorkspaceLayout{return {version:1,workspaceId:id,explorer:this.explorers.get(id)??true,panels:this.records(id),dock:this.ensure(id).toJson()};}
 save(id=this.workspaceId,announce=true){if(!this.persistence||!announce&&this.pendingRestore(id).length)return;try{if(this.blocked.has(id))throw Error('此工作区原布局不可识别，已保留；本次不覆盖。可继续编辑，但暂不持久化新布局。');const snapshot=this.snapshot(id);this.persistence.save(snapshot);this.saved.set(id,validateLayout(snapshot));if(announce)this.notify({notice:'布局已保存；不包含正文、路径、文件授权或终端命令。',error:null});}catch(e){this.fail(e);}}
 private schedule(id:string){clearTimeout(this.timers.get(id));this.timers.set(id,setTimeout(()=>{this.timers.delete(id);this.save(id,false);},450));}
 pendingRestore(id=this.workspaceId){const saved=this.saved.get(id);return saved?.panels.filter(p=>['markdown','image','pdf'].includes(p.kind)&&p.resourceId&&!this.records(id).some(s=>s.kind===p.kind&&s.resourceId===p.resourceId))??[];}
 async restore(){const saved=this.saved.get(this.workspaceId);if(!saved){this.notify({notice:'尚无已保存布局。'});return;}const id=this.workspaceId;
  for(const p of saved.panels.filter(p=>!p.hidden&&p.resourceId)){
   if(this.records(id).some(s=>s.kind===p.kind&&s.resourceId===p.resourceId))continue;
   if(p.kind==='markdown'){const store=allDocumentStores().find(s=>!s.getState().document)??createDocumentStore();this.bind(store);await store.getState().openProject(p.resourceId!);}else if(p.kind==='pdf'){try{await (await import('../pdf/PdfViewerSession')).openPdf(p.resourceId!);}catch(e){this.fail(e);}}else if(p.kind==='image'){try{await openLibraryEditor(p.resourceId!);}catch(e){this.fail(e);}}
   if(this.workspaceId!==id){this.notify({notice:'该资源已在另一工作区打开，已切换到其独立会话。'});return;}
  }
  const mapping=new Map<string,string>();for(const p of saved.panels){const current=this.records().find(s=>s.id===p.id||s.kind===p.kind&&s.resourceId&&s.resourceId===p.resourceId);if(current){mapping.set(p.id,current.id);const session=this.sessions.get(current.id);if(session)session.panel={...session.panel,hidden:p.hidden};}}
  this.models.set(id,Model.fromJson(projectDock(saved,this.records(),mapping)));this.notify({notice:'已恢复可打开资源的位置；未保存草稿、缺失或未授权资源不会生成空白 Markdown 面板。'});this.schedule(id);
 }
 focusDocument(store:DocumentStore){const item=[...this.documents].find(([,b])=>b.store===store);if(item)this.focus(item[0]);else focusDocumentStore(store);}
 disconnect(){for(const stop of this.stops)stop();this.stops=[];for(const timer of this.timers.values())clearTimeout(timer);this.timers.clear();for(const store of [...this.bindings.keys()])this.unbind(store);this.started=false;}
 activePanel(){return this.model.getActiveTabset()?.getSelectedNode()?.getId()??'workbench';}
}
export const workspaceManager=new WorkspaceManager();
