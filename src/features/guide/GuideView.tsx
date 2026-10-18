import {useMemo,useRef,useState} from 'react';
import text from '../../../docs/USER_GUIDE.md?raw';
import {MarkdownPreview} from '../document/MarkdownPreview';
import {documentOutline} from '../document/outline';
import projectsImage from '../../../docs/images/v021-projects.png';
import mathImage from '../../../docs/images/v021-math.png';
const guideImages={'images/v021-projects.png':projectsImage,'images/v021-math.png':mathImage};
const groups=[{name:'开始与采集',sections:[1,2,3]},{name:'资料与项目',sections:[4,5,7]},{name:'图片工具',sections:[6,8,12]},{name:'Markdown 写作',sections:[9,10,11]},{name:'设置与数据安全',sections:[13,14]}];
export function GuideView(){
 const headings=useMemo(()=>documentOutline(text).filter(h=>h.depth===2),[]),body=useRef<HTMLDivElement>(null),[query,setQuery]=useState('');
 const visible=headings.filter(h=>h.text.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()));
 return <section className="guide-workspace" aria-label="完整使用指南"><header className="page-heading"><div><p className="text-xs text-accent">RESEARCHCLIP · 本地使用手册</p><h2>从图片到研究记录</h2><p>按任务查找步骤，先保存资料，再组织与写作。</p></div></header><div className="guide-layout"><nav aria-label="使用指南目录"><input className="guide-search" aria-label="搜索指南章节" value={query} onChange={e=>setQuery(e.target.value)} placeholder="搜索章节名称…"/>{groups.map(group=><div className="guide-topic-group" key={group.name}><h3>{group.name}</h3>{visible.filter(h=>group.sections.includes(Number(h.text.split('.')[0]))).map(h=><button key={h.offset} onClick={()=>body.current?.querySelector<HTMLElement>(`[data-heading-offset="${h.offset}"]`)?.scrollIntoView({block:'start'})}>{h.text}</button>)}</div>)}{!visible.length&&<p className="text-xs text-muted">没有匹配的章节，试试“项目”或“保存”。</p>}</nav><div ref={body} className="guide-content"><MarkdownPreview markdown={text} assets={{}} guideImages={guideImages}/></div></div></section>;
}
