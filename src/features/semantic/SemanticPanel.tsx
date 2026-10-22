import {useEffect,useRef,useState} from 'react';
import {isTauri} from '@tauri-apps/api/core';
import {Search,Sparkles,StopCircle} from 'lucide-react';
import {semanticCall,semanticStatus,validateSemanticQuery,mergeSemanticStatus} from './api';
import type {SemanticStatus} from './api';
import {chooseMathRuntime} from '../math-ocr/api';
import {errorMessage} from '../../shared/api/errors';
import {useLibraryStore} from '../library/store';
import {screenshotImage} from '../../shared/api/library';
import {openKnowledgeCard} from '../../app/card-actions';
import {Modal} from '../../shared/ui/Modal';

function Thumbnail({id,title}:{id:string;title:string}){
  const [url,setUrl]=useState<string|null>(null);
  useEffect(()=>{let active=true,owned:string|null=null;void screenshotImage(id,true).then(blob=>{if(active){owned=URL.createObjectURL(blob);setUrl(owned);}}).catch(()=>{if(active)setUrl(null);});return()=>{active=false;if(owned)URL.revokeObjectURL(owned);};},[id]);
  return url?<img src={url} alt={title} loading="lazy"/>:<span className="semantic-placeholder">缩略图暂不可用</span>;
}
export function SemanticPanel({favorites}:{favorites:boolean}){
  const [status,setStatus]=useState<SemanticStatus|null>(null),[query,setQuery]=useState(''),[error,setError]=useState<string|null>(null),[pending,setPending]=useState(false),[confirm,setConfirm]=useState<'download'|'clear'|'rebuild'|null>(null);
  const alive=useRef(true),request=useRef(false);const {projectId,unclassified}=useLibraryStore();
  const busy=pending||status?.job.phase==='running'||status?.job.phase==='cancelling';
  useEffect(()=>{alive.current=true;let active=true;let timer:ReturnType<typeof setTimeout>|undefined;async function poll(){try{const next=await semanticStatus();if(active){setStatus(current=>mergeSemanticStatus(current,next));timer=setTimeout(()=>void poll(),['running','cancelling'].includes(next.job.phase)?700:3000);}}catch(e){if(active){setError(errorMessage(e,'语义状态读取失败。'));timer=setTimeout(()=>void poll(),5000);}}}if(isTauri())void poll();return()=>{active=false;alive.current=false;if(timer)clearTimeout(timer);};},[]);
  async function run(action:()=>Promise<unknown>){if(request.current)return;request.current=true;setPending(true);setError(null);try{await action();const next=await semanticStatus();if(alive.current)setStatus(current=>mergeSemanticStatus(current,next));}catch(e){if(alive.current)setError(errorMessage(e,'语义操作失败，原图与普通搜索保留。'));}finally{request.current=false;if(alive.current)setPending(false);}}
  const start=(kind:'index'|'query'|'download',rebuild=false)=>run(async()=>{if(kind==='query')validateSemanticQuery(query);await semanticCall('semantic_start',{kind,query:kind==='query'?query:null,rebuild,scope:{projectId,unclassified,favorites},confirmed:kind!=='query'});});
  const job=status?.job,scopeMatches=job?.scope?.projectId===(projectId??null)&&job?.scope?.unclassified===unclassified&&job?.scope?.favorites===favorites,results=!busy&&job?.kind==='query'&&job.phase==='completed'&&scopeMatches?job.results:null;
  return <section className="semantic-panel" aria-label="离线图文语义检索" aria-busy={busy}>
    <div className="semantic-intro"><span className="semantic-icon"><Sparkles size={21}/></span><div><h3>描述画面，寻找相关截图 <span className="subtle-badge">实验性 · 本地 CPU</span></h3><p>用中文或英文描述图片内容。结果按视觉语义相似度排序，可能误匹配；需要人工核对。</p></div></div>
    {!isTauri()?<p role="status">请在Windows桌面应用中选择运行时与模型。普通搜索仍可使用。</p>:<>
      <form className="semantic-query" onSubmit={e=>{e.preventDefault();void start('query');}}><label htmlFor="semantic-query">画面描述<input id="semantic-query" value={query} onChange={e=>setQuery(e.target.value)} placeholder="例如：神经网络结构图 / a natural landscape" maxLength={1000} disabled={busy}/></label><button className="secondary-button text-accent" disabled={busy||!status?.modelsPresent||!status.configured||!status.stats.ready}><Search size={16}/>语义搜索</button></form>
      <p className="semantic-scope">当前结果范围：{projectId?'所选项目':unclassified?'未分类素材':'全部项目'}{favorites?' · 仅收藏':''}。最多显示前60项；标题与 OCR 文字不参与向量排序。</p>
      <details className="semantic-setup"><summary>模型与索引管理 · {status?.stats.ready??0} / {status?.stats.total??0} 张已索引</summary><div className="semantic-setup-body"><p>默认不下载、不索引、不上传图片。首次选择与公式OCR共用的 Python 3.12+ 运行时，再导入配套模型或明确确认下载。模型约228 MB；依赖和离线步骤见使用指南 6.4。</p><div className="semantic-actions">
        <button className="secondary-button" disabled={busy} onClick={()=>void run(()=>chooseMathRuntime())}>选择本地 Python</button><button className="secondary-button" disabled={busy} onClick={()=>void run(()=>semanticCall('semantic_import'))}>导入离线模型</button><button className="secondary-button" disabled={busy||!status?.configured} onClick={()=>setConfirm('download')}>下载模型…</button>
        <button className="secondary-button text-accent" disabled={busy||!status?.configured||!status.modelsPresent} onClick={()=>void start('index')}>开启 / 更新语义索引</button><button className="secondary-button" disabled={busy||!status?.configured||!status.modelsPresent} onClick={()=>setConfirm('rebuild')}>重建索引…</button><button className="secondary-button" disabled={busy||!status?.stats.ready} onClick={()=>setConfirm('clear')}>删除索引…</button>
      </div><p>固定配对：CLIP ViT-B/32 图像编码器 + 官方对齐的 multilingual-v1 文本编码器。单任务、2个推理线程、进程内存上限2 GiB，首版最多2000张。普通搜索不依赖模型。</p></div></details>
      {(busy||job?.phase)&&<div className="semantic-progress" role="status"><div><strong>{busy?'正在执行':job?.phase==='failed'?'任务失败':job?.phase==='cancelled'?'已取消':'任务完成'}</strong><span>{job?.kind==='index'?`已扫描 ${job.scanned} / ${job.total} · 新建 ${job.computed} · 跳过 ${job.skipped}`:job?.kind==='query'?'文本编码与本地向量检索':job?.kind==='import'?'离线文件校验与导入':'模型下载与校验'}</span></div>{busy&&<button className="secondary-button" disabled={pending||job?.phase==='cancelling'} onClick={()=>void run(()=>semanticCall('semantic_cancel'))}><StopCircle size={15}/>{job?.phase==='cancelling'?'正在取消…':'取消任务'}</button>}</div>}
      {(error||job?.error)&&<p role="alert" className="error-banner">{error??job?.error}</p>}
      {job?.kind==='query'&&job.phase==='completed'&&!scopeMatches&&<p role="status" className="semantic-scope">项目或收藏范围已改变，请重新搜索。</p>}
      {results&&<div className="semantic-results"><p role="status">查询：{job?.query} · 已比较 {results.searched} 张 · {results.items.length} 个候选 · {results.stale} 条过期索引已排除。余弦分数不是准确率；更新索引可加入新图和修复过期记录。</p><div className="semantic-grid">{results.items.map((item,index)=><button className="semantic-card" key={item.id} onClick={()=>openKnowledgeCard(item.id)}><Thumbnail id={item.id} title={item.title}/><span className="semantic-card-title">{index+1}. {item.title}</span><span className="semantic-score">相似度 {item.score.toFixed(3)}</span></button>)}</div>{!results.items.length&&<p>没有可用的已索引截图。请更新索引或调整项目和收藏范围。</p>}</div>}
    </>}
    {confirm&&<Modal title={confirm==='download'?'确认模型下载':confirm==='clear'?'删除语义索引':'重建语义索引'} onClose={()=>setConfirm(null)}><div className="p-5"><p>{confirm==='download'?'将从Hugging Face下载约228 MB固定版本模型（CLIP MIT / 文本模型 Apache-2.0）。仅下载模型，不发送截图。下载可取消，已有不同文件不会覆盖。':confirm==='clear'?'只删除可重建的向量索引，原图、标注、OCR与文档保留。模型文件保留。':'重新编码所有原图，逐项替换向量。取消或失败时保留已完成记录，其他记录仍可使用。'}</p><div className="semantic-actions"><button className="secondary-button" onClick={()=>setConfirm(null)}>取消</button><button className="secondary-button text-accent" onClick={()=>{const choice=confirm;setConfirm(null);if(choice==='clear')void run(()=>semanticCall('semantic_clear',{confirmed:true}));else void start(choice==='download'?'download':'index',choice==='rebuild');}}>确认{confirm==='download'?'下载':confirm==='clear'?'删除索引':'重建'}</button></div></div></Modal>}
  </section>;
}
