import {useEffect,useState,type RefObject} from 'react';
import {linkTargets} from './api';
import {linkMarkdown,type GraphNode,type WikiTrigger} from './model';
import {useDocumentStore} from '../document/store';
import {errorMessage} from '../../shared/api/errors';
export function LinkPicker({trigger,input}:{trigger:WikiTrigger;input:RefObject<HTMLTextAreaElement|null>}){
 const [attempt,setAttempt]=useState(0),[page,setPage]=useState(0),[selected,setSelected]=useState(0),[dismissed,setDismissed]=useState(false),[result,setResult]=useState<{key:string;items:GraphNode[];more:boolean;error?:string}|null>(null);
 const query=trigger.query,key=`${query}:${page}:${attempt}`;
 const choose=(node:GraphNode)=>{if(!node.available)return;const s=useDocumentStore.getState();s.setCaret(trigger.start,trigger.end);s.insertText(linkMarkdown(node,node.title));};
 const current=result?.key===key?result:null;
 useEffect(()=>{let active=true;const timer=setTimeout(()=>{void linkTargets(query,page).then(r=>{if(active)setResult({key,...r});}).catch(e=>{if(active)setResult({key,items:[],more:false,error:errorMessage(e,'内部引用搜索失败。')});});},150);return()=>{active=false;clearTimeout(timer);};},[query,page,key]);
 useEffect(()=>{const node=input.current;if(!node||dismissed)return;const keydown=(e:KeyboardEvent)=>{if(e.isComposing||e.ctrlKey||e.metaKey||e.altKey)return;if(e.key==='Escape'){e.preventDefault();e.stopPropagation();setDismissed(true);}if(current?.items.length){if(e.key==='ArrowDown'||e.key==='ArrowUp'){e.preventDefault();setSelected(i=>(i+(e.key==='ArrowDown'?1:-1)+current.items.length)%current.items.length);}if(e.key==='Enter'){e.preventDefault();choose(current.items[Math.min(selected,current.items.length-1)]);}}};node.addEventListener('keydown',keydown);return()=>node.removeEventListener('keydown',keydown);});
 if(dismissed)return null;
 return <section className="internal-link-picker" aria-label="内部引用候选"><header><strong>链接到文档或截图</strong><button onClick={()=>setDismissed(true)} aria-label="取消内部引用搜索">取消 · Esc</button></header>{!current?<p role="status">正在搜索…</p>:current.error?<p role="alert">{current.error}<button onClick={()=>setAttempt(n=>n+1)}>重试</button></p>:<><div role="listbox" aria-label="内部引用搜索结果">{current.items.map((node,i)=><button role="option" disabled={!node.available} aria-selected={i===Math.min(selected,current.items.length-1)} key={`${node.kind}:${node.id}`} onMouseDown={e=>e.preventDefault()} onClick={()=>choose(node)}><span>{node.kind==='document'?'文档':'截图'}</span><strong>{node.title}</strong><small>{node.available?node.id.slice(0,8):'目标失效'}</small></button>)}</div>{!current.items.length&&<p>没有匹配内容。先在应用中打开或保存目标文档；相同名称可通过类型和ID区分。</p>}<footer><span>↑↓ 选择 · Enter 插入</span><button disabled={!page} onClick={()=>{setPage(page-1);setSelected(0);}}>上一页</button><button disabled={!current.more} onClick={()=>{setPage(page+1);setSelected(0);}}>下一页</button></footer></>}</section>;
}
