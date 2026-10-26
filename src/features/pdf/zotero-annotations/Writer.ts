import {create} from 'zustand';
import {invoke} from '@tauri-apps/api/core';
import {sourceKey,annotationSource,type AnnotationSource,type ZoteroAnnotation} from './model';
import {errorMessage} from '../../../shared/api/errors';
export const writeKinds=['highlight','underline','note'] as const;
export type WriteKind=typeof writeKinds[number];
export interface WriteGeometry {pageIndex:number;view:number[];userUnit:number;rectangles:number[][]}
export type WriteInput={action:'create';annotationType:WriteKind;color:string;text:string;comment:string;geometry:WriteGeometry}|{action:'update';annotationKey:string;expectedVersion:number;annotationType:WriteKind;color:string;text:string;comment:string}|{action:'delete';annotationKey:string;expectedVersion:number};
export interface WriteCapabilities {productVersion:string|null;supportedTypes:string[];authorized:boolean;remembered:boolean;message:string}
export interface WriteOutcome {verified:boolean;annotationKey:string;annotation:ZoteroAnnotation|null;message:string}
export interface WriteResponse {capabilities:WriteCapabilities|null;outcome:WriteOutcome|null;pending:boolean}
export type WriteRequest={operation:'capabilities'|'authorize'|'forget'|'verify'}|{operation:'submit';input:WriteInput};
interface WriteState {source:string;busy:boolean;capabilities:WriteCapabilities|null;pending:boolean;message:string;error:string;draft:WriteInput|null}
export function parseWriteResponse(value:unknown):WriteResponse {
 const v=value as WriteResponse|null,c=v?.capabilities,o=v?.outcome;
 if(!v||typeof v.pending!=='boolean'||!(c===null||c&&typeof c.authorized==='boolean'&&typeof c.remembered==='boolean'&&typeof c.message==='string'&&(c.productVersion===null||typeof c.productVersion==='string')&&Array.isArray(c.supportedTypes)&&c.supportedTypes.every(k=>writeKinds.includes(k as WriteKind)))||!(o===null||o&&typeof o.verified==='boolean'&&/^[A-Z0-9]{8}$/.test(o.annotationKey)&&typeof o.message==='string'&&(o.annotation===null||typeof o.annotation==='object')))throw Error('Zotero写入响应无效，请保留草稿并复核。');
 return v;
}
/** Per PDF native authorization and draft owner; never stores an API key. */
export class ZoteroAnnotationWriter {
 readonly state=create<WriteState>(()=>({source:'',busy:false,capabilities:null,pending:false,message:'写入默认关闭；先核对来源，再检测能力并请求Zotero授权。',error:'',draft:null}));
 private disposed=false;private lookup:string|null=null;private drafts=new Map<string,WriteInput>();private pendingSources=new Set<string>();
 get hasDrafts(){return !!this.state.getState().draft||this.drafts.size>0;}
 get hasPending(){return this.state.getState().pending||this.pendingSources.size>0;}
 constructor(private token:string){}
 bind(source:AnnotationSource){const key=sourceKey(source),s=this.state.getState();if(s.source===key)return;if(s.busy)throw Error('批注操作未结束，请稍候切换来源。');if(s.draft)this.drafts.set(s.source,s.draft);this.state.setState({source:key,capabilities:null,pending:this.pendingSources.has(key),draft:this.drafts.get(key)??null,error:'',message:'来源已切换，请重新检测并为当前来源授权。'});}
 draft(value:WriteInput|null){if(this.state.getState().busy)return;this.state.setState({draft:value});if(value)this.drafts.set(this.state.getState().source,value);else this.drafts.delete(this.state.getState().source);}
 async request(source:AnnotationSource,request:WriteRequest){source=annotationSource(source);if(this.disposed||this.state.getState().busy)return null;this.bind(source);const id=crypto.randomUUID().replaceAll('-','');this.lookup=id;this.state.setState({busy:true,error:'',message:request.operation==='authorize'?'请在Zotero授权弹窗中选择；等待最多90秒。取消等待不会代替关闭该弹窗。':request.operation==='submit'?'正在校验、条件写入并重新读取；不要重复提交。':request.operation==='verify'?'仅复读待核实批注，不重新发送写入。':'正在核对当前来源与写入能力…'});
 try{const response=parseWriteResponse(await invoke('zotero_annotation_write',{token:this.token,source,lookupId:id,request}));if(this.disposed)return null;if(response.pending)this.pendingSources.add(this.state.getState().source);else this.pendingSources.delete(this.state.getState().source);this.state.setState({capabilities:response.capabilities,pending:response.pending,message:response.outcome?.message??response.capabilities?.message??'操作完成。'});if(response.outcome?.verified)this.draftAfterVerified();return response;}catch(e){if(!this.disposed){const code=e&&typeof e==='object'&&'code'in e?String(e.code):'';if(['ZOTERO_WRITE_UNKNOWN','ZOTERO_WRITE_PROTOCOL','ZOTERO_WRITE_SOURCE_MISMATCH'].includes(code))this.pendingSources.add(this.state.getState().source);this.state.setState({error:errorMessage(e,'写入未确认，草稿保留。'),message:'未声明保存成功。草稿保留；检查权限、版本或连接后再处理。',...(['ZOTERO_WRITE_UNKNOWN','ZOTERO_WRITE_PROTOCOL','ZOTERO_WRITE_SOURCE_MISMATCH'].includes(code)?{pending:true}:{}),...(['ZOTERO_WRITE_AUTH_REQUIRED','ZOTERO_WRITE_SOURCE_MISMATCH'].includes(code)?{capabilities:null}:{})});}return null;}finally{this.lookup=null;if(!this.disposed)this.state.setState({busy:false});}}
 private draftAfterVerified(){this.drafts.delete(this.state.getState().source);this.state.setState({draft:null});}
 async cancel(){if(this.lookup){await invoke('cancel_zotero_lookup',{lookupId:this.lookup});this.state.setState({message:'已请求取消；已发出的写入仍需复读确认，不保证撤销。'});}}
 dispose(){this.disposed=true;const id=this.lookup;if(id)void invoke('cancel_zotero_lookup',{lookupId:id}).catch(()=>{/* Native PDF close also cancels this owner. */});this.drafts.clear();this.pendingSources.clear();this.state.setState({draft:null,capabilities:null});}
}
