import type {ComponentType} from 'react';
import type {EditorKind,PanelRecord} from './layout';
export interface PanelLifecycle {dirty:()=>boolean;busy:()=>boolean;save:()=>Promise<boolean>;dispose:()=>Promise<void>;resize?:(width:number,height:number)=>void}
export interface EditorSession {panel:PanelRecord;workspaceId:string;lifecycle:PanelLifecycle}
export interface EditorPanelProps {session:EditorSession}
export interface EditorDefinition {kind:EditorKind;label:string;available:boolean;component:ComponentType<EditorPanelProps>}
export class EditorRegistry {
 private entries=new Map<EditorKind,EditorDefinition>();
 register(entry:EditorDefinition){if(this.entries.has(entry.kind))throw Error('面板类型已注册。');this.entries.set(entry.kind,entry);}
 get(kind:EditorKind){const entry=this.entries.get(kind);if(!entry)throw Error('未注册的编辑器类型。');return entry;}
 list(){return [...this.entries.values()];}
}

export const editorRegistry=new EditorRegistry();
