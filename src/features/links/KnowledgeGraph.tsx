import {useEffect,useMemo,useRef,useState} from 'react';
import {Network,RefreshCw,Focus,Plus,Minus} from 'lucide-react';
import {isTauri} from '@tauri-apps/api/core';
import {getGraph,type GraphFilter} from './api';
import {graphLayout,targetKey,type GraphData,type GraphNode} from './model';
import {indexDocumentLinks} from './indexing';
import {openInternalTarget} from './navigation';
import {useProjectStore} from '../projects/store';
import {listProjectDocuments} from '../projects/api';
import {useTagStore} from '../tags/store';
import {useDocumentStore} from '../document/store';
import {errorMessage} from '../../shared/api/errors';
const initial:GraphFilter={projectId:null,unclassified:false,tagId:null,focus:null};
export function KnowledgeGraph(){
 const [filter,setFilter]=useState<GraphFilter>(initial),[version,setVersion]=useState(0),[snapshot,setSnapshot]=useState<{key:string;data:GraphData}|null>(null),[error,setError]=useState<string|null>(null),[zoom,setZoom]=useState(1),[pan,setPan]=useState({x:0,y:0}),[indexing,setIndexing]=useState(false),[progress,setProgress]=useState(''),[failures,setFailures]=useState<string[]>([]);
 const projects=useProjectStore(s=>s.items),current=useProjectStore(s=>s.current),projectToken=useProjectStore(s=>s.token),docToken=useDocumentStore(s=>s.token),tags=useTagStore(s=>s.items),svg=useRef<SVGSVGElement>(null),controller=useRef<AbortController|null>(null),drag=useRef<{x:number;y:number;pan:{x:number;y:number}}|null>(null),key=JSON.stringify(filter)+version;
 const data=snapshot?.key===key?snapshot.data:null;
 useEffect(()=>{if(!isTauri())return;void useProjectStore.getState().refresh();void useTagStore.getState().refresh();return()=>controller.current?.abort();},[]);
 useEffect(()=>{let active=true;void getGraph(filter).then(data=>{if(active){setSnapshot({key,data});setError(null);}}).catch(e=>{if(active)setError(errorMessage(e,'知识图谱加载失败。'));});return()=>{active=false;};},[filter,key]);
 useEffect(()=>{const host=svg.current;if(!host)return;const wheel=(e:WheelEvent)=>{e.preventDefault();setZoom(z=>Math.max(.25,Math.min(4,z*(e.deltaY>0?.9:1.1))));};host.addEventListener('wheel',wheel,{passive:false});return()=>host.removeEventListener('wheel',wheel);},[]);
 const nodes=data?.nodes,positions=useMemo(()=>graphLayout(nodes??[],filter.focus),[nodes,filter.focus]);
 const baseScale=1/Math.max(1,...[...positions.values()].map(p=>Math.hypot(p.x-500,p.y-320)/280)),scale=zoom*baseScale;
 const change=(next:Partial<GraphFilter>)=>{setFilter(f=>({...f,...next}));setZoom(1);setPan({x:0,y:0});};
 const open=async(node:GraphNode)=>{setError(null);if(!node.available){setError('目标已删除或不可访问；引用仍保留供核对。');return;}try{await openInternalTarget(node);}catch(e){setError(errorMessage(e,'打开节点失败。'));}};
 const rebuild=async()=>{
  if(controller.current)return;const abort=new AbortController();controller.current=abort;setIndexing(true);setFailures([]);setError(null);let done=0;const failed:string[]=[];
  try{const documents=await listProjectDocuments(null,false);for(const d of documents){abort.signal.throwIfAborted();setProgress(`正在更新 ${done+1} / ${documents.length}：${d.name}`);try{await indexDocumentLinks(d.id,abort.signal);}catch(e){if(abort.signal.aborted)throw e;failed.push(`${d.name}：${errorMessage(e,'索引失败')}`);}done++;}setProgress(`索引更新完成：${done-failed.length} 篇成功，${failed.length} 篇未更新`);}
  catch(e){if(abort.signal.aborted)setProgress(`已取消；已完成的 ${done} 篇索引保留`);else setError(errorMessage(e,'索引更新失败，原有索引保留。'));}
  finally{controller.current=null;if(!abort.signal.aborted||svg.current){setIndexing(false);setFailures(failed);setVersion(v=>v+1);}}
 };
 return <section className="page-container graph-page" aria-label="知识图谱工作区"><header className="page-heading"><div><p className="guide-eyebrow"><Network size={14}/> LOCAL KNOWLEDGE</p><h2>知识图谱</h2><p>把研究文档与截图连接起来。滚轮缩放，拖动空白区域平移，点击节点打开内容。</p></div></header>
 <div className="graph-controls"><label>项目<select aria-label="图谱项目筛选" value={filter.unclassified?'none':filter.projectId??'all'} onChange={e=>change({projectId:['all','none'].includes(e.target.value)?null:e.target.value,unclassified:e.target.value==='none'})}><option value="all">全部项目</option><option value="none">未分类</option>{projects.map(p=><option key={p.id} value={p.id}>{p.name}</option>)}</select></label><label>标签<select aria-label="图谱标签筛选" value={filter.tagId??''} onChange={e=>change({tagId:e.target.value||null})}><option value="">全部标签</option>{tags.map(t=><option key={t.id} value={t.id}>{t.name}</option>)}</select></label>
 <button className="secondary-button" disabled={!filter.focus&&(!current||projectToken!==docToken||!docToken)} aria-pressed={Boolean(filter.focus)} onClick={()=>change({focus:filter.focus?null:`document:${current!.id}`})}><Focus size={14}/>{filter.focus?'查看全图':'聚焦当前文档'}</button><button className="secondary-button" onClick={()=>setVersion(n=>n+1)}><RefreshCw size={14}/>刷新图谱</button><button className="secondary-button" disabled={indexing||!isTauri()} onClick={()=>void rebuild()}>更新全部链接索引</button>{indexing&&<button className="secondary-button" onClick={()=>controller.current?.abort()}>取消更新索引</button>}</div>
 {(progress||failures.length>0)&&<div className="graph-index-status"><p role="status">{progress}</p>{failures.length>0&&<details><summary>{failures.length} 篇文档未更新</summary>{failures.map((f,i)=><p key={i}>{f}</p>)}</details>}</div>}
 {error&&<p role="alert" className="error-banner">{error}<button className="secondary-button" onClick={()=>setVersion(n=>n+1)}>重试读取图谱</button></p>}
 <div className="graph-legend"><span className="graph-document">■ 文档</span><span className="graph-screenshot">● 截图卡片</span><span className="graph-missing">◌ 失效引用</span><span>{data?`${data.nodes.length} 个节点 · ${data.edges.length} 条连线`:'正在读取图谱…'}</span></div>
 {data?.truncated&&<p role="status" className="link-hint">图谱最多展示1000个节点、5000条连线。请使用项目、标签或邻居筛选缩小范围。</p>}
 <div className="graph-canvas"><div className="graph-zoom"><button className="icon-button" aria-label="缩小图谱" onClick={()=>setZoom(z=>Math.max(.25,z/1.2))}><Minus size={16}/></button><span>{Math.round(zoom*100)}%</span><button className="icon-button" aria-label="放大图谱" onClick={()=>setZoom(z=>Math.min(4,z*1.2))}><Plus size={16}/></button><button className="secondary-button" onClick={()=>{setZoom(1);setPan({x:0,y:0});}}>适应图谱</button></div>
 <svg ref={svg} viewBox="0 0 1000 640" role="application" aria-label="可缩放和平移的知识图谱" onPointerDown={e=>{if((e.target as Element).closest('[data-graph-node]'))return;drag.current={x:e.clientX,y:e.clientY,pan};e.currentTarget.setPointerCapture(e.pointerId);}} onPointerMove={e=>{if(!drag.current)return;const rect=e.currentTarget.getBoundingClientRect();setPan({x:drag.current.pan.x+(e.clientX-drag.current.x)*1000/rect.width,y:drag.current.pan.y+(e.clientY-drag.current.y)*640/rect.height});}} onPointerUp={()=>{drag.current=null;}} onPointerCancel={()=>{drag.current=null;}}>
 <defs><marker id="graph-arrow" viewBox="0 0 10 10" refX="26" refY="5" markerWidth="5" markerHeight="5" orient="auto"><path d="M 0 0 L 10 5 L 0 10 z"/></marker></defs>
 <g transform={`translate(${500+pan.x} ${320+pan.y}) scale(${scale}) translate(-500 -320)`}>
 {data?.edges.map(e=>{const a=positions.get(e.source),b=positions.get(e.target);return a&&b?(e.source===e.target?<path className="graph-edge" key={`${e.source}:${e.target}`} d={`M ${a.x-10} ${a.y-10} c -50 -60 70 -60 25 0`} markerEnd="url(#graph-arrow)"/>:<line className="graph-edge" key={`${e.source}:${e.target}`} x1={a.x} y1={a.y} x2={b.x} y2={b.y} markerEnd="url(#graph-arrow)"/>):null;})}
 {data?.nodes.map(n=>{const p=positions.get(targetKey(n))!;return <g key={targetKey(n)} data-graph-node={targetKey(n)} className={`graph-node graph-${n.kind} ${n.available?'':'is-missing'}`} role="button" tabIndex={0} aria-label={`打开${n.kind==='document'?'文档':'截图'} ${n.title}${n.available?'':'（目标失效）'}`} onClick={()=>void open(n)} onKeyDown={e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();void open(n);}}} transform={`translate(${p.x} ${p.y})`}><title>{n.title} · {n.id}</title>{n.kind==='document'?<rect x="-17" y="-17" width="34" height="34" rx="9"/>:<circle r="17"/>}<text y="34" textAnchor="middle">{Array.from(n.title).slice(0,18).join('')}{n.title.length>18?'…':''}</text></g>;})}</g></svg>
 {data&&!data.nodes.length&&<div className="graph-empty"><Network size={32}/><strong>此范围还没有节点</strong><p>试试清除筛选，或打开文档并更新链接索引。</p></div>}</div>
 <details className="graph-node-list"><summary>节点列表 · 键盘快速打开</summary>{data?.nodes.map(n=><button className="link-row" key={targetKey(n)} onClick={()=>void open(n)}><span>{n.title}</span><small>{n.kind==='document'?'文档':'截图'} · {n.available?'可打开':'目标失效'}</small></button>)}</details>
 </section>;
}
