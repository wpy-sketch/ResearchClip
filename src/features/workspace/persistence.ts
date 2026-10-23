import {validateLayout,type WorkspaceLayout,validWorkspaceId} from './layout';
export class WorkspacePersistence {
 constructor(private storage:Pick<Storage,'getItem'|'setItem'>){}
 private key(id:string){if(!validWorkspaceId(id))throw Error('工作区标识无效。');return 'researchclip.workspace.layout:'+id;}
 read(id:string):WorkspaceLayout|null{const raw=this.storage.getItem(this.key(id));if(raw===null)return null;if(raw.length>65536)throw Error('工作区布局超过64KiB，原记录已保留。');let value:unknown;try{value=JSON.parse(raw);}catch{throw Error('布局记录损坏，原记录已保留。');}const layout=validateLayout(value);if(layout.workspaceId!==id)throw Error('布局归属不匹配。');return layout;}
 save(layout:WorkspaceLayout){const existing=this.storage.getItem(this.key(layout.workspaceId));if(existing!==null)this.read(layout.workspaceId);const checked=validateLayout(layout),raw=JSON.stringify(checked);if(raw.length>65536)throw Error('布局超过64KiB，未覆盖原记录。');try{this.storage.setItem(this.key(layout.workspaceId),raw);}catch{throw Error('布局保存失败，当前编辑内容与原布局已保留；请检查本机存储空间。');}}
}
