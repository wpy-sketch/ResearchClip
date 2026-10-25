import {createContext,lazy,Suspense,useCallback,useContext,useEffect,useRef,useState,type CSSProperties,type ReactNode} from 'react';
import {Layout,I18nLabel,type TabNode} from 'flexlayout-react';
import {PanelLeft,PanelBottom,Columns2,Rows2,EyeOff,Save,RotateCcw,PanelTop,Minus,Plus} from 'lucide-react';
import 'flexlayout-react/style/rounded.css';
import './workspace.css';
import {workspaceManager as manager} from './manager';
import {editorRegistry,type EditorPanelProps} from './registry';
import {DocumentSessionContext,useDocumentApi,useDocumentStore} from '../document/store';
import {useEditorStore} from '../editor/store';
import {DocumentConfirmation} from '../document/DocumentConfirmation';
import {TemplateDialog} from '../document/TemplateDialog';
import {ExternalPdfOpen} from '../pdf/ExternalPdfOpen';
import {PdfOpenButton} from '../pdf/PdfOpenButton';
import {Modal} from '../../shared/ui/Modal';
import {useProjectStore} from '../projects/store';
import {useSettingsStore} from '../settings/store';
const PdfViewer=lazy(()=>import('../pdf/PdfViewer').then(m=>({default:m.PdfViewer})));
const MarkdownPanel=lazy(()=>import('../document/MarkdownPanel').then(m=>({default:m.MarkdownPanel})));
const EditorView=lazy(()=>import('../editor/EditorView').then(m=>({default:m.EditorView})));
const dockLabels:Record<string,string>={[I18nLabel.Close_Tab]:'关闭面板',[I18nLabel.Maximize]:'最大化面板',[I18nLabel.Restore]:'还原面板',[I18nLabel.Splitter]:'调整面板大小',[I18nLabel.Overflow_Menu_Tooltip]:'更多标签',[I18nLabel.Error_rendering_component]:'面板加载失败',[I18nLabel.Error_rendering_component_retry]:'重试'};
const Content=createContext<ReactNode>(null);
function Workbench(){return <>{useContext(Content)}</>;}
function DocumentPanel({session}:EditorPanelProps){const entry=manager.documents.get(session.panel.id);return entry?<DocumentSessionContext.Provider value={entry.store}><MarkdownPanel/></DocumentSessionContext.Provider>:null;}
function ImagePanel({session}:EditorPanelProps){return <EditorView sessionId={session.panel.id.slice(6)}/>;}
function TerminalPanel(){return <section className="workspace-terminal" aria-label="终端区域"><h3>终端区域</h3><p>尚未连接终端运行时。本阶段仅提供面板布局，没有启动命令、Shell、LSP 或 Agent 进程。</p><p className="text-muted">代码与终端能力将在相应阶段单独授权接入。</p></section>;}
function Unavailable({session}:EditorPanelProps){return <p role="status">{session.panel.kind==='pdf'?'PDF 阅读器':'代码编辑器'}尚未实现，本阶段仅注册扩展类型。</p>;}
if(!editorRegistry.list().length)for(const definition of [{kind:'workbench',label:'资料工作台',available:true,component:Workbench},{kind:'markdown',label:'Markdown',available:true,component:DocumentPanel},{kind:'image',label:'图片编辑器',available:true,component:ImagePanel},{kind:'pdf',label:'PDF阅读器',available:true,component:PdfViewer},{kind:'code',label:'代码 · 待后续接入',available:false,component:Unavailable},{kind:'terminal',label:'终端布局区域',available:true,component:TerminalPanel}] as const)editorRegistry.register(definition);
function Panel({id}:{id:string}){const host=useRef<HTMLDivElement>(null),session=manager.sessions.get(id),Definition=editorRegistry.get(session?.panel.kind??'workbench').component;
 useEffect(()=>{const node=host.current;if(!node)return;const observer=new ResizeObserver(entries=>{const {width,height}=entries[0].contentRect;if(width>0&&height>0){node.dataset.editorWidth=String(Math.round(width));node.dataset.editorHeight=String(Math.round(height));session?.lifecycle.resize?.(width,height);}});observer.observe(node);return()=>observer.disconnect();},[session]);
 const focus=()=>{if(session?.panel.kind==='markdown'){const store=manager.documents.get(id)?.store;if(store)manager.focusDocument(store);}else if(session?.panel.kind==='image'){const image=id.slice(6);if(useEditorStore.getState().activeId!==image)useEditorStore.getState().activate(image);}};
 return <div ref={host} className="research-panel" data-editor-kind={session?.panel.kind??'workbench'} data-panel-id={id} onPointerDownCapture={focus} onFocusCapture={focus} onDropCapture={focus}><Suspense fallback={<p role="status">正在加载编辑器…</p>}><Definition session={session??{panel:{id:'workbench',kind:'workbench',title:'资料工作台',resourceId:null,hidden:false},workspaceId:manager.workspaceId,lifecycle:{dirty:()=>false,busy:()=>false,save:async()=>true,dispose:async()=>{}}}}/></Suspense></div>;
}
export function ResearchWorkspace({children}:{children:ReactNode}){
 const [cssScale,setCssScale]=useState(1);
 useEffect(()=>{const shell=document.querySelector<HTMLElement>('.app-shell'),previous=shell?.style.height??'';let applied=previous;const update=()=>{const zoom=(node:Element)=>Number.parseFloat(getComputedStyle(node).zoom)||1;const measured=zoom(document.documentElement)*zoom(document.body),scale=Number.isFinite(measured)&&measured>0?measured:1;setCssScale(scale);if(shell){applied=scale===1?previous:`calc(100dvh / ${scale})`;if(shell.style.height!==applied)shell.style.height=applied;}};update();const observer=new MutationObserver(update);for(const node of [document.documentElement,document.body])observer.observe(node,{attributes:true,attributeFilter:['style','class']});window.addEventListener('resize',update);return()=>{observer.disconnect();window.removeEventListener('resize',update);if(shell&&shell.style.height===applied)shell.style.height=previous;};},[]);

 const state=manager.state(),projects=useProjectStore(s=>s.items),api=useDocumentApi(),template=useDocumentStore(s=>s.templateDialog),docError=useDocumentStore(s=>s.error),docOpen=useDocumentStore(s=>!!s.document),fontSize=useSettingsStore(s=>s.snapshot.preferences.fontSize);
 useEffect(()=>{manager.initialize(window.localStorage);const flush=()=>manager.save(manager.workspaceId,false);window.addEventListener('pagehide',flush);return()=>window.removeEventListener('pagehide',flush);},[]);
 const factory=useCallback((node:TabNode)=><Panel id={node.getId()}/>,[]),model=state.ready?manager.model:null;
 const hidden=[...manager.sessions.values()].filter(s=>s.workspaceId===state.workspaceId&&s.panel.hidden),terminal=[...manager.sessions.values()].find(s=>s.workspaceId===state.workspaceId&&s.panel.kind==='terminal'),pending=manager.pendingRestore();
 const zoom=(delta:number)=>void useSettingsStore.getState().change({fontSize:Math.max(12,Math.min(20,fontSize+delta))});
 return <><div className="research-workspace" data-css-scaled={cssScale!==1} style={{'--workspace-layout-scale':cssScale} as CSSProperties} data-workspace-id={state.workspaceId} data-document-open={docOpen}>
 <div className="workspace-layout-tools" role="toolbar" aria-label="工作区布局"><label className="workspace-picker">工作区<select aria-label="研究工作区" value={state.workspaceId} disabled={state.busy} onChange={e=>manager.switchWorkspace(e.target.value)}><option value="default">默认工作区</option>{projects.map(p=><option key={p.id} value={'project:'+p.id}>{p.name}</option>)}</select></label><PdfOpenButton/><span className="workspace-tool-divider"/>
 <button className="icon-button" aria-label={state.explorer?'隐藏资源管理器':'显示资源管理器'} title="显示或隐藏左侧资源管理器" aria-pressed={state.explorer} onClick={()=>manager.toggleExplorer()}><PanelLeft size={16}/></button><button className="icon-button" aria-label={terminal&&!terminal.panel.hidden?'隐藏终端区域':'显示终端区域'} title="底部终端布局区域（尚未接入运行时）" aria-pressed={!!terminal&&!terminal.panel.hidden} onClick={()=>manager.toggleTerminal()}><PanelBottom size={16}/></button>
 <button className="icon-button" aria-label="向右拆分当前面板" title="水平拆分：将当前标签放到右侧" onClick={()=>manager.split(false)}><Columns2 size={16}/></button><button className="icon-button" aria-label="向下拆分当前面板" title="垂直拆分：将当前标签放到下方" onClick={()=>manager.split(true)}><Rows2 size={16}/></button><button className="icon-button" aria-label="隐藏当前面板" title="隐藏当前编辑面板，保留草稿" onClick={()=>manager.hide(manager.activePanel())}><EyeOff size={16}/></button>
 <button className="secondary-button" onClick={()=>manager.save()}><Save size={14}/><span>保存布局</span></button><button className="secondary-button" disabled={state.busy} onClick={()=>void manager.restore()}><RotateCcw size={14}/><span>恢复布局</span></button>
 {hidden.length>0&&<label className="reopen-panels"><PanelTop size={14}/><select aria-label="重新打开隐藏面板" value="" onChange={e=>{if(e.target.value)manager.focus(e.target.value);}}><option value="">隐藏面板 · {hidden.length}</option>{hidden.map(s=><option key={s.panel.id} value={s.panel.id}>{s.panel.title}</option>)}</select></label>}
 <span className="workspace-zoom"><button className="icon-button" aria-label="缩小界面字号" title="缩小界面字号" disabled={fontSize<=12} onClick={()=>zoom(-1)}><Minus size={14}/></button><span title="界面基础字号">{fontSize}px</span><button className="icon-button" aria-label="放大界面字号" title="放大界面字号" disabled={fontSize>=20} onClick={()=>zoom(1)}><Plus size={14}/></button></span></div>
 {state.error&&<p role="alert" className="error-banner workspace-layout-message">{state.error}<button onClick={()=>manager.state.setState({error:null})}>关闭提示</button></p>}{state.notice&&<p role="status" className="workspace-layout-message">{state.notice}<button aria-label="关闭布局提示" onClick={()=>manager.state.setState({notice:null})}>×</button></p>}{!docOpen&&docError&&<p role="alert" className="error-banner">{docError}</p>}
 {pending.length>0&&<p className="workspace-layout-message">上次有 {pending.length} 个资源待重新打开。布局不会自动授予文件权限或生成空白 Markdown 面板。<button disabled={state.busy} onClick={()=>void manager.restore()}>重新打开上次资源</button></p>}
 <Content.Provider value={children}><div className="dock-layout" aria-label="科研工作区面板">{model?<Layout key={state.workspaceId} model={model} factory={factory} i18nTranslator={label=>dockLabels[label]??label} onContextMenu={(_,event)=>event.preventDefault()} onAction={action=>manager.onAction(action)} onModelChange={()=>manager.changed()}/>:<p role="status">正在恢复工作区布局…</p>}</div></Content.Provider>
 </div><ExternalPdfOpen/><DocumentConfirmation/>{template&&<DocumentSessionContext.Provider value={api}><TemplateDialog key={template} mode={template}/></DocumentSessionContext.Provider>}
 {state.closingImage&&<Modal title="关闭未保存图片面板" onClose={()=>void manager.resolveImage('cancel')}><div className="p-5"><p>此图片有未保存标注。隐藏面板可以保留草稿；关闭前请选择保存、放弃或取消。</p>{state.error&&<p role="alert">{state.error}</p>}<div className="workspace-confirm-actions"><button disabled={state.busy} onClick={()=>void manager.resolveImage('cancel')}>取消</button><button disabled={state.busy} onClick={()=>void manager.resolveImage('discard')}>放弃标注并关闭</button><button disabled={state.busy} onClick={()=>void manager.resolveImage('save')}>保存到素材库后关闭</button></div></div></Modal>}</>;
}
