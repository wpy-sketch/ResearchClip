import {create} from 'zustand';
import {useCaptureStore} from '../capture/store';
import {useEditorStore} from '../editor/store';
import {useDocumentStore} from '../document/store';
import {useLibraryStore} from '../library/store';
import {useUiStore} from '../../app/ui-store';
import {useCardStore} from '../notes/store';
import {createScreenshot,getScreenshot,screenshotImage} from '../../shared/api/library';
import {ocrStatus,recognizeOcr} from '../ocr/api';
import {validateText} from '../ocr/model';
import {mathStatus,recognizeMath,cancelMath,validateLatex} from '../math-ocr/api';
import {formulaMarkdown} from '../math-ocr/store';
import {actionSpecs,type WorkflowConfig} from './model';
import {createRunner,type ActionRegistry,type Payload,type Context} from './runner';
import {errorMessage} from '../../shared/api/errors';
type Image=Extract<Payload,{type:'image'}>;
type Draft=Extract<Payload,{type:'draft'|'confirmed'}>;
interface Interaction {kind:'select'|'confirm';draft?:Draft;resolve:(value:string|null)=>void}
export const useCommandUi=create<{open:boolean;tab:'commands'|'workflows';error:string|null;interaction:Interaction|null;show:()=>void}>((set)=>({open:false,tab:'commands',error:null,interaction:null,show:()=>set({open:true,error:null})}));
function ask(signal:AbortSignal,kind:Interaction['kind'],draft?:Draft):Promise<string>{return new Promise((resolve,reject)=>{
 const finish=(value:string|null)=>{signal.removeEventListener('abort',abort);useCommandUi.setState({interaction:null});if(value===null)reject(new DOMException('用户已取消','AbortError'));else resolve(value);};
 const abort=()=>finish(null);if(signal.aborted){abort();return;}
 signal.addEventListener('abort',abort,{once:true});useCommandUi.setState({interaction:{kind,draft,resolve:finish}});
});}
export async function libraryImage(id:string):Promise<Image>{const item=await getScreenshot(id),blob=await screenshotImage(id);return {type:'image',blob,name:item.title,widthPx:item.widthPx,heightPx:item.heightPx,screenshotId:item.id};}
export async function currentImage():Promise<Image|null>{const view=useUiStore.getState().activeView;if(view==='card'){const id=useCardStore.getState().activeId;return id?libraryImage(id):null;}const editor=useEditorStore.getState(),session=editor.activeId?editor.sessions[editor.activeId]:null;if(view==='editor')return session&&!session.loadError?{type:'image',blob:session.image.blob,name:session.image.name,widthPx:session.image.widthPx,heightPx:session.image.heightPx,screenshotId:session.image.libraryId}:null;const preview=useCaptureStore.getState().preview;return preview?.blob?{type:'image',blob:preview.blob,name:'当前截图',widthPx:preview.widthPx,heightPx:preview.heightPx,screenshotId:preview.library?.id}:null;}
let targetDocument:string|null=null;
const image=(p:Payload)=>{if(p.type!=='image')throw Error('请提供图片。');return p;};
const draft=(p:Payload)=>{if(p.type!=='draft'&&p.type!=='confirmed')throw Error('请先识别并校对结果。');return p;};
const execute:Record<string,(p:Payload,c:Context)=>Promise<Payload>>={
 capture:async(_p,c)=>{const s=useCaptureStore.getState();if(s.phase!=='idle')throw Error('截图或文件操作正在进行，请等待。');if(!s.selectedDisplayId)await s.refreshDisplays();c.signal.throwIfAborted();const previous=useCaptureStore.getState().preview?.captureId;await useCaptureStore.getState().captureRegion();const latest=useCaptureStore.getState();if(latest.error)throw Error(latest.error);if(!latest.preview?.blob||latest.preview.captureId===previous)throw new DOMException('截图已取消','AbortError');const p=latest.preview;return {type:'image',blob:p.blob!,name:`工作流截图 ${new Date().toLocaleString('zh-CN')}`,widthPx:p.widthPx,heightPx:p.heightPx,screenshotId:p.library?.id};},
 select:async(_p,c)=>c.select(),
 card:async(p,c)=>{const source=image(p);c.signal.throwIfAborted();const item=source.screenshotId?await getScreenshot(source.screenshotId):await createScreenshot(source.blob,source.name,{schemaVersion:1,image:{widthPx:source.widthPx,heightPx:source.heightPx},objects:[]});void useLibraryStore.getState().refresh();return {type:'result',message:source.screenshotId?'已有素材卡片已保留，未重复创建。':'截图已存为本地素材卡片。',screenshotId:item.id};},
 ocr:async(p,c)=>{const source=image(p),status=await ocrStatus();c.signal.throwIfAborted();const language=status.languages.find(l=>l.tag.startsWith('zh'))?.tag??status.languages[0]?.tag;if(!language)throw Error('没有Windows OCR语言包，请在系统语言选项安装后重试本步骤。');const result=await recognizeOcr(source.blob,language,null);c.signal.throwIfAborted();if(!result.text.trim())throw Error('未识别到文字，请核对图片。已完成的截图步骤仍保留。');return {type:'draft',text:result.text,format:'text',image:source};},
 formula:async(p,c)=>{const source=image(p),status=await mathStatus();if(status.job.phase==='running')throw Error('已有公式任务运行，请等待完成。');c.signal.throwIfAborted();const job=await recognizeMath(source.blob,null);let cancelSent=false;const started=Date.now();while(Date.now()-started<90000){if(c.signal.aborted&&!cancelSent){await cancelMath();cancelSent=true;}await new Promise(r=>setTimeout(r,350));const next=await mathStatus();if(next.job.id!==job.id)throw Error('公式任务已改变，请核对现有结果。');if(next.job.phase==='completed'){c.signal.throwIfAborted();const text=next.job.result?.latex??'';validateLatex(text);return {type:'draft',text,format:'latex',image:source};}if(['error','cancelled'].includes(next.job.phase))throw Error(next.job.error??'公式识别已取消。');}await cancelMath();throw Error('公式识别等待超时，已请求取消。请在设置检查任务状态后重试。');},
 confirm:async(p,c)=>{const source=draft(p),text=await c.confirm(source);if(source.format==='latex')validateLatex(text);else {validateText(text);if(!text.trim())throw Error('校对结果不能为空。');}c.signal.throwIfAborted();return {...source,type:'confirmed',text};},
 insert:async(p,c)=>{const result=draft(p),s=useDocumentStore.getState();c.signal.throwIfAborted();if(p.type!=='confirmed'||!targetDocument||s.document?.id!==targetDocument||s.phase!=='idle'||s.pending)throw Error('原Markdown文档已关闭、切换或正忙，结果保留；请恢复原文档后重试，或手动复制结果。');if(!s.insertText(result.format==='latex'?formulaMarkdown(result.text):result.text))throw Error(s.error??'Markdown插入失败，结果保留。');return {type:'result',message:'已插入当前Markdown草稿，可撤销；请另行保存文档。',documentId:targetDocument};},
};
export const registry:ActionRegistry=new Map(actionSpecs.map(spec=>[spec.actionId,{...spec,execute:execute[spec.actionId]}]));
export const runner=createRunner(registry,{select:async signal=>libraryImage(await ask(signal,'select')),confirm:(draft,signal)=>ask(signal,'confirm',draft)});
export async function startWorkflow(workflow:WorkflowConfig){
 const state=runner.getSnapshot();if(['running','waiting','cancelling'].includes(state.phase))return;
 const doc=useDocumentStore.getState();if(workflow.actions.includes('insert')&&(!doc.document||doc.phase!=='idle'||doc.pending)){useCommandUi.setState({error:'请先新建或打开可编辑的 Markdown 文档，再开始此工作流。尚未截图或创建文件。'});return;}
 targetDocument=doc.document?.id??null;useCommandUi.setState({open:false,error:null});try{await runner.start(workflow);}catch(e){useCommandUi.setState({error:errorMessage(e,'工作流无法启动。')});}
}
export function cancelWorkflow(){runner.cancel();useCommandUi.getState().interaction?.resolve(null);}
