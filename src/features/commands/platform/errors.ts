import {commandNames,stableId,type CommandName} from './contracts';
const messages={INVALID_INPUT:'命令参数或资源标识无效。',UNKNOWN_COMMAND:'此应用命令未注册。',PERMISSION_DENIED:'此模块未获准执行该操作。',MODULE_UNAVAILABLE:'对应模块尚未安装或启动。',NOT_FOUND:'资源不存在或已删除，请检查关联。',UNSUPPORTED_VERSION:'命令或资源版本不受支持。',INVALID_RESPONSE:'后端响应不符合当前契约。',REQUEST_REUSED:'请求标识已用于另一项操作。',LEDGER_FULL:'本会话命令去重记录已满，请保存后重启。',BUSY:'目标资源已有操作或任务数量已达上限，请稍后再试。',CANCELLED:'操作已取消。',OPERATION_FAILED:'操作未能完成，现有草稿和资料已保留；请检查当前面板提示。',EVENT_LOOP:'已阻止循环事件。',LISTENER_LIMIT:'事件监听数量已达上限，请先释放原监听。',LISTENER_FAILED:'一个事件订阅者异常，其他订阅者继续处理。',TASK_COMMITTING:'当前操作已进入提交阶段，请等待完成。'} as const;
const sourceMessages={PDF_SOURCE_MISMATCH:'PDF来源文件不可用或版本不一致，未跳转；请保留笔记并核对原件。'} as const;
export type ErrorCode=keyof typeof messages|keyof typeof sourceMessages;
export class CommandFailure extends Error {constructor(readonly code:ErrorCode){super(Object.hasOwn(sourceMessages,code)?sourceMessages[code as keyof typeof sourceMessages]:messages[code as keyof typeof messages]);this.name='CommandFailure';}}
export function failure(value:unknown):CommandFailure{
 try{if(value instanceof DOMException&&value.name==='AbortError')return new CommandFailure('CANCELLED');
 const candidate=value&&typeof value==='object'&&'code'in value?value.code:value instanceof Error?value.message:'',code=typeof candidate==='string'?candidate:'';
 return new CommandFailure(code==='WINDOW_DENIED'?'PERMISSION_DENIED':Object.hasOwn(messages,code)||Object.hasOwn(sourceMessages,code)?code as ErrorCode:'OPERATION_FAILED');
 }catch{return new CommandFailure('OPERATION_FAILED');}
}

export interface Diagnostic {readonly id:number;readonly time:number;readonly code:ErrorCode;readonly message:string;readonly command?:CommandName;readonly requestId?:string}
/** Diagnostics deliberately exclude exception text, paths, document bodies and credentials. */
export class ErrorReporter {
 private items:readonly Diagnostic[]=[];private serial=0;private listeners=new Set<()=>void>();
 getSnapshot=()=>this.items;
 subscribe=(listener:()=>void)=>{this.listeners.add(listener);return()=>{this.listeners.delete(listener);};};
 private notify(){for(const listener of [...this.listeners])try{listener();}catch{this.listeners.delete(listener);this.items=Object.freeze([...this.items.slice(-19),Object.freeze({id:++this.serial,time:Date.now(),code:'LISTENER_FAILED' as const,message:messages.LISTENER_FAILED})]);}}
 report(error:unknown,context:{command?:CommandName;requestId?:string}={}){const e=failure(error),item=Object.freeze({id:++this.serial,time:Date.now(),code:e.code,message:e.message,...(context.command&&commandNames.includes(context.command)?{command:context.command}:{}),...(stableId(context.requestId)?{requestId:context.requestId}:{})});this.items=Object.freeze([...this.items.slice(-19),item]);this.notify();return e;}
 dismiss(id:number){this.items=Object.freeze(this.items.filter(x=>x.id!==id));this.notify();}
}
