import './style.css';
import {newId,publicImageUrl,requestFits,restorePending,clipFingerprint,type Pending,type ClipRequest,type Extracted,type Mode,type Reply} from './types';
const $=<T extends HTMLElement>(id:string)=>document.getElementById(id) as T;
const mode=$<HTMLSelectElement>('mode'),title=$<HTMLInputElement>('title'),project=$<HTMLSelectElement>('project'),copy=$<HTMLInputElement>('copy');
let captured:Extracted|null=null;let selected=new Set<string>();let busy=false;let controller:AbortController|null=null;let disconnect:(()=>void)|null=null;
let pending:Pending|null=null;
const status=(message:string,error=false)=>{const s=$('status');s.textContent=message;s.classList.toggle('error',error);};
function setBusy(value:boolean){busy=value;for(const e of document.querySelectorAll<HTMLInputElement|HTMLSelectElement|HTMLButtonElement>('button,input,select'))if(e.id!=='cancel')e.disabled=value;}
function native(request:ClipRequest):Promise<Reply>{
 return new Promise((resolve,reject)=>{
  const port=chrome.runtime.connectNative('io.researchclip.clipper');let done=false;
  const finish=(reply?:Reply,message?:string)=>{if(done)return;done=true;clearTimeout(timer);disconnect=null;port.disconnect();if(reply)resolve(reply);else reject(Error(message??'通信已取消；已接收的请求可能完成，请重试确认。'));};
  const timer=setTimeout(()=>finish(undefined,'桌面响应超时，请启动应用并启用网页剪藏。同一请求重试不会重复创建。'),20000);
  disconnect=()=>finish();
  port.onMessage.addListener((reply:Reply)=>{if(!reply||typeof reply.ok!=='boolean'||typeof reply.message!=='string'||typeof reply.code!=='string')finish(undefined,'Host响应结构无效。');else finish(reply);});
  port.onDisconnect.addListener(()=>{const error=chrome.runtime.lastError?.message;finish(undefined,error?`连接失败：${error}。请检查Host注册和桌面开关。`:'Host通信已中断，可重试同一请求。');});
  port.postMessage(request);
 });
}
async function projects(){
 const result=await native({version:1,requestId:newId(),operation:'projects',clip:null});if(!result.ok)throw Error(result.message);
 const old=project.value;project.replaceChildren(new Option('未分类',''));
 for(const p of result.data.projects??[]){if(/^[a-f0-9]{32}$/.test(p.id)&&typeof p.name==='string')project.add(new Option(p.name,p.id));}
 project.value=old;status('已连接桌面。选择项目后保存。');
}
function current(){
 if(!captured)throw Error('请先读取当前页。');
 const images=captured.mode==='images'?captured.images.filter(i=>selected.has(i.id)):captured.images;
 const markdown=captured.mode==='images'?images.map(i=>`![${i.alt.replace(/[\\[\]\n\r]/g,' ')}](clip-image://${i.id})`).join('\n\n'):captured.markdown;
 return {...captured,images,markdown};
}
function preview(){
 const c=current();$('preview').hidden=false;let text=c.markdown;
 for(const i of c.images)text=text.replaceAll(`clip-image://${i.id}`,i.url);
 $<HTMLTextAreaElement>('markdown').value=text||'(仅保存标题、网址及来源信息)';
}
function showImages(){
 const box=$('images');box.replaceChildren();if(!captured)return;
 if(captured.mode==='images')for(const img of captured.images){
  const label=document.createElement('label');label.className='image-choice';const input=document.createElement('input');input.type='checkbox';input.checked=selected.has(img.id);
  input.addEventListener('change',()=>{if(input.checked)selected.add(img.id);else selected.delete(img.id);preview();});label.append(input);
  if(img.thumbnail){const picture=document.createElement('img');picture.src=img.thumbnail;picture.alt='';label.append(picture);}
  const caption=document.createElement('span');caption.textContent=`${img.alt} · ${img.width}×${img.height}`;label.append(caption);box.append(label);
 }else{const p=document.createElement('p');p.className='hint';p.textContent=`正文含 ${captured.images.length} 张兼容图片引用。未勾选复制时只保留来源。`;box.append(p);}
}
async function prepare(){
 setBusy(true);status('正在读取当前页…');
 try{
  const [tab]=await chrome.tabs.query({active:true,currentWindow:true});if(!tab?.id)throw Error('未找到当前标签页。');
  await chrome.scripting.executeScript({target:{tabId:tab.id},world:'ISOLATED',files:['content.js']});
  const [result]=await chrome.scripting.executeScript({target:{tabId:tab.id},world:'ISOLATED',func:(value:Mode)=>window.__researchClipExtract?.(value),args:[mode.value as Mode]});
  if(!result?.result)throw Error('页面读取失败，浏览器可能不允许扩展访问此页。');
  captured=result.result;selected=new Set(captured.images.map(i=>i.id));title.value=captured.title;pending=null;await chrome.storage.session.remove('pendingClip');showImages();preview();status('请检查预览并修改标题，再点击保存。');
 }catch(e){status(e instanceof Error?e.message:'页面提取失败。',true);}finally{setBusy(false);}
}
async function copyImage(url:string,source:string,signal:AbortSignal):Promise<string>{
 if(!publicImageUrl(url)||new URL(url).protocol!=='https:'||new URL(url).origin!==new URL(source).origin)throw Error('跨域、非HTTPS或本地地址，仅保留来源');
 const response=await fetch(url,{credentials:'omit',redirect:'error',referrerPolicy:'no-referrer',cache:'no-store',signal:AbortSignal.any([signal,AbortSignal.timeout(5000)])});
 if(!response.ok||!/^image\/(png|jpeg|webp)(;|$)/i.test(response.headers.get('content-type')??''))throw Error('图片无法匿名读取或格式不支持');
 if(Number(response.headers.get('content-length')??0)>256*1024)throw Error('图片超过256 KiB');
 const reader=response.body?.getReader();if(!reader)throw Error('图片响应为空');const chunks:Uint8Array[]=[];let size=0;
 try{for(;;){const {value,done}=await reader.read();if(done)break;size+=value.length;if(size>256*1024)throw Error('图片超过256 KiB');chunks.push(value);}}finally{await reader.cancel().catch(()=>{});}
 const bytes=new Uint8Array(size);let offset=0;for(const c of chunks){bytes.set(c,offset);offset+=c.length;}
 const bitmap=await createImageBitmap(new Blob([bytes]));
 try{if(bitmap.width*bitmap.height>4_194_304)throw Error('图片像素超过419万');const canvas=document.createElement('canvas');canvas.width=bitmap.width;canvas.height=bitmap.height;canvas.getContext('2d')?.drawImage(bitmap,0,0);const blob=await new Promise<Blob>((resolve,reject)=>canvas.toBlob(b=>b?resolve(b):reject(Error('PNG转换失败')),'image/png'));if(blob.size>256*1024)throw Error('转换PNG超过256 KiB');const data=new Uint8Array(await blob.arrayBuffer());return btoa(Array.from(data,b=>String.fromCharCode(b)).join(''));}finally{bitmap.close();}
}
async function save(){
 if(!captured||busy)return;const c=current();if(!title.value.trim())return status('标题不能为空。',true);if(c.mode==='images'&&!c.images.length)return status('请至少选择一张图片。',true);
 const fingerprint=clipFingerprint(c,title.value,project.value,copy.checked);setBusy(true);controller=new AbortController();status('正在准备并保存…');
 try{
  if(!pending||pending.fingerprint!==fingerprint){
   const images:NonNullable<ClipRequest['clip']>['images']=[];let total=0;let copies=0;
   for(const image of c.images){let pngBase64:string|null=null;let warning:string|null=copy.checked?'当前图片未复制':'用户未允许复制';
    if(copy.checked&&copies<4){try{pngBase64=await copyImage(image.url,c.url,controller.signal);const size=atob(pngBase64).length;if(total+size>384*1024)throw Error('图片合计超过384 KiB');total+=size;copies++;warning=null;}catch(e){if(controller.signal.aborted)throw e;pngBase64=null;warning=e instanceof Error?e.message:'图片复制失败';}}
    images.push({id:image.id,url:image.url,alt:image.alt,pngBase64,warning});
   }
   const request:ClipRequest={version:1,requestId:newId(),operation:'save',clip:{title:title.value.trim(),url:c.url,siteName:c.siteName,byline:c.byline,mode:c.mode,markdown:c.markdown,projectId:project.value||null,images}};
   if(!requestFits(request))throw Error('消息超过700 KiB，请减少图片或选中文字剪藏。');
   pending={at:Date.now(),fingerprint,copyImages:copy.checked,request};await chrome.storage.session.set({pendingClip:pending});
  }
  if(controller.signal.aborted)throw Error('已取消准备。');
  const result=await native(pending.request);if(!result.ok)throw Error(result.message);
  status(`${result.data.duplicate?'已保存过，未重复创建':'保存成功'}。请在桌面提示中打开Markdown。${c.images.some(i=>!pending?.request.clip?.images.find(p=>p.id===i.id)?.pngBase64)?'部分图片仅保留来源，请在文档中查看资源状态。':''}`);
 }catch(e){status(e instanceof Error?e.message:'保存失败，可重试。',true);}finally{controller=null;setBusy(false);}
}
$('prepare').addEventListener('click',()=>void prepare());$('connect').addEventListener('click',()=>{if(busy)return;setBusy(true);void projects().catch(e=>status(String(e.message??e),true)).finally(()=>setBusy(false));});$('save').addEventListener('click',()=>void save());
$('cancel').addEventListener('click',()=>{controller?.abort();disconnect?.();status('已取消等待；已交给桌面的请求可能已保存。保持内容不变并重试可确认，不会重复创建。');});
void chrome.storage.session.get('pendingClip').then(({pendingClip})=>{const p=restorePending(pendingClip);
 if(p&&Date.now()-p.at<10*60*1000&&p.request?.clip){pending=p;const c=p.request.clip;captured={...c,images:c.images.map((i)=>({...i,width:0,height:0,thumbnail:null}))};mode.value=c.mode;title.value=c.title;project.add(new Option(c.projectId?'上次选择的项目（刷新可更新名称）':'未分类',c.projectId??''));project.value=c.projectId??'';copy.checked=Boolean(p.copyImages);selected=new Set(c.images.map((i:{id:string})=>i.id));showImages();preview();status('已恢复本次浏览器会话内的待确认剪藏；重试可确认保存结果。');}
 else void chrome.storage.session.remove('pendingClip');
}).catch(()=>status('无法读取扩展会话状态。请重新预览后保存。',true));
