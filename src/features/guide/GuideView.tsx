import syncSettingsImage from '../../../docs/images/v300-sync-settings.png';
import syncFlowImage from '../../../docs/images/v300-sync-flow.svg';
import guideDarkImage from '../../../docs/images/v200-guide-dark.png';
import guideLightImage from '../../../docs/images/v200-guide-light.png';
import revisionsImage from '../../../docs/images/v105-revisions.svg';
import mathOcrInputImage from '../../../docs/images/v104-math-ocr-input.png';
import mathOcrWorkflowImage from '../../../docs/images/v104-math-ocr-workflow.svg';
import ocrIndexImage from '../../../docs/images/v103-ocr-index.svg';
import {useEffect,useMemo,useRef,useState} from 'react';
import {ChevronRight,Search,BookOpen} from 'lucide-react';
import text from '../../../docs/USER_GUIDE.md?raw';
import {MarkdownPreview} from '../document/MarkdownPreview';
import {Modal} from '../../shared/ui/Modal';
import {guideSections,searchGuide,guideOffsets,visibleGuideLocation,type GuideSection} from './navigation';
import projectsImage from '../../../docs/images/v021-projects.png';
import mathImage from '../../../docs/images/v021-math.png';
import scrollingImage from '../../../docs/images/v101-scrolling.png';
import clipperSettingsImage from '../../../docs/images/v102-clipper-settings.png';
import clipperExtensionImage from '../../../docs/images/v102-clipper-extension.png';
import captureImage from '../../../docs/images/v100-capture.png';
import library_gridImage from '../../../docs/images/v100-library-grid.png';
import library_listImage from '../../../docs/images/v100-library-list.png';
import editorImage from '../../../docs/images/v100-editor.png';
import cardImage from '../../../docs/images/v100-card.png';
import compareImage from '../../../docs/images/v100-compare.png';
import markdownImage from '../../../docs/images/v100-markdown.png';
import settingsImage from '../../../docs/images/v100-settings.png';
const guideImages={'images/v300-sync-settings.png':syncSettingsImage,'images/v300-sync-flow.svg':syncFlowImage,'images/v200-guide-dark.png':guideDarkImage,'images/v200-guide-light.png':guideLightImage,'images/v105-revisions.svg':revisionsImage,'images/v104-math-ocr-input.png':mathOcrInputImage,'images/v104-math-ocr-workflow.svg':mathOcrWorkflowImage,'images/v103-ocr-index.svg':ocrIndexImage,'images/v102-clipper-settings.png':clipperSettingsImage,'images/v102-clipper-extension.png':clipperExtensionImage,'images/v101-scrolling.png':scrollingImage,'images/v021-projects.png':projectsImage,'images/v021-math.png':mathImage,'images/v100-capture.png':captureImage,'images/v100-library-grid.png':library_gridImage,'images/v100-library-list.png':library_listImage,'images/v100-editor.png':editorImage,'images/v100-card.png':cardImage,'images/v100-compare.png':compareImage,'images/v100-markdown.png':markdownImage,'images/v100-settings.png':settingsImage};
const groups=[{name:'开始与采集',sections:[1,2,3]},{name:'资料与项目',sections:[4,5,7]},{name:'图片工具',sections:[6,8,12]},{name:'Markdown 写作',sections:[9,10,11]},{name:'设置与数据安全',sections:[13,14]}];

function GuideBranch({nodes,depth,collapsed,active,searching,onToggle,onJump}:{nodes:GuideSection[];depth:number;collapsed:Set<number>;active:number|null;searching:boolean;onToggle:(offset:number)=>void;onJump:(offset:number)=>void}){
 return <ol className="guide-tree">{nodes.filter(n=>n.heading.depth<=depth).map(node=>{const h=node.heading,children=node.children.length>0&&h.depth<depth,expanded=searching||!collapsed.has(h.offset);return <li key={h.offset} data-guide-depth={h.depth}><div className="guide-tree-row">{children?<button className="guide-tree-toggle" aria-label={`${expanded?'收起':'展开'} ${h.text}`} aria-expanded={expanded} onClick={()=>onToggle(h.offset)}><ChevronRight size={13} style={{transform:expanded?'rotate(90deg)':undefined}}/></button>:<span className="guide-tree-spacer"/>}<button className="guide-tree-link" aria-current={active===h.offset?'location':undefined} onClick={()=>onJump(h.offset)}>{h.text}</button></div>{children&&expanded&&<GuideBranch nodes={node.children} depth={depth} collapsed={collapsed} active={active} searching={searching} onToggle={onToggle} onJump={onJump}/>}</li>;})}</ol>;
}
export function GuideView(){
 const sections=useMemo(()=>guideSections(text),[]),body=useRef<HTMLDivElement>(null),[query,setQuery]=useState(''),[active,setActive]=useState<number|null>(null),[figure,setFigure]=useState<{url:string;alt:string}|null>(null),[depth,setDepth]=useState(3),[collapsed,setCollapsed]=useState<Set<number>>(new Set());
 const visible=useMemo(()=>searchGuide(sections,query),[sections,query]);
 const jump=(offset:number)=>{setActive(offset);const node=body.current?.querySelector<HTMLElement>(`[data-heading-offset="${offset}"]`);if(node){node.tabIndex=-1;node.scrollIntoView({block:'start'});node.focus({preventScroll:true});}};
 useEffect(()=>{
  const host=body.current?.closest<HTMLElement>('.workspace-scroll'),offsets=new Set(guideOffsets(sections,4));let frame=0;
  const update=()=>{frame=0;if(!host)return;const top=host.getBoundingClientRect().top+80;const nodes=Array.from(body.current?.querySelectorAll<HTMLElement>('[data-heading-offset]')??[]).filter(n=>offsets.has(Number(n.dataset.headingOffset)));const current=nodes.filter(n=>n.getBoundingClientRect().top<=top).at(-1)??nodes[0];if(current)setActive(Number(current.dataset.headingOffset));};
  const scroll=()=>{if(!frame)frame=requestAnimationFrame(update);};host?.addEventListener('scroll',scroll,{passive:true});scroll();return()=>{host?.removeEventListener('scroll',scroll);cancelAnimationFrame(frame);};
 },[sections]);
 const shownDepth=query.trim()?4:depth,shownActive=visibleGuideLocation(visible,active,shownDepth,query.trim()?new Set():collapsed);
 const toggle=(offset:number)=>setCollapsed(previous=>{const next=new Set(previous);if(next.has(offset))next.delete(offset);else next.add(offset);return next;});
 return <section className="guide-workspace" aria-label="完整使用指南"><header className="page-heading"><div><p className="guide-eyebrow"><BookOpen size={14}/> RESEARCHCLIP · 使用手册</p><h2>从截图到研究成果</h2><p>先选任务，再按步骤操作。正文保留快捷键、完成检查和失败处理。</p></div><span className="guide-edition">v3.0 · Windows</span></header>
 <div className="guide-quick-start" aria-label="推荐阅读路线">{[{section:1,title:'开始使用',text:'工作区 · 采集 · 三种保存',number:'01'},{section:5,title:'整理科研素材',text:'项目分类 · 标签 · 批量操作',number:'02'},{section:9,title:'编写图文笔记',text:'Markdown · 公式 · 导出',number:'03'}].map(item=><button key={item.section} onClick={()=>{setQuery('');const section=sections.find(s=>s.chapter===item.section);if(section)jump(section.heading.offset);}}><span className="guide-route-number">{item.number}</span><div><strong>{item.title}</strong><span>{item.text}</span></div><ChevronRight size={16}/></button>)}</div>
 <div className="guide-layout"><nav aria-label="使用指南目录"><div className="guide-directory-heading"><strong>使用指南目录</strong><span>{sections.length} 章</span></div><label className="guide-search-box"><Search size={15}/><input className="guide-search" aria-label="搜索指南章节" value={query} onChange={e=>setQuery(e.target.value)} placeholder="搜索标题或操作关键词…"/></label><div className="guide-directory-tools"><label>目录层级<select aria-label="指南目录层级" value={depth} onChange={e=>setDepth(Number(e.target.value))}><option value={2}>章节</option><option value={3}>章节与小节</option><option value={4}>详细操作步骤</option></select></label><div><button onClick={()=>setCollapsed(new Set())}>展开全部</button><button onClick={()=>setCollapsed(new Set(sections.map(s=>s.heading.offset)))}>收起全部</button></div></div>
 {query&&<div className="guide-search-feedback" role="status"><span>{visible.length?`找到 ${visible.length} 个相关章节`:'没有匹配结果'}</span><button onClick={()=>setQuery('')}>清除</button></div>}
 {groups.map(group=>{const nodes=visible.filter(s=>group.sections.includes(s.chapter));return nodes.length?<div className="guide-topic-group" key={group.name}><h3>{group.name}</h3><GuideBranch nodes={nodes} depth={shownDepth} collapsed={collapsed} active={shownActive} searching={Boolean(query.trim())} onToggle={toggle} onJump={jump}/></div>:null;})}{!visible.length&&<p className="guide-no-result">试试“版本”“插图”或“快捷键”。搜索会保留匹配小节的上级目录。</p>}</nav>
 <div ref={body} className="guide-content"><div className="guide-reading-bar"><span>{sections.find(s=>s.heading.offset===visibleGuideLocation(sections,active,2,new Set()))?.heading.text??"阅读使用指南"}</span><button className="secondary-button" onClick={()=>{body.current?.closest<HTMLElement>('.workspace-scroll')?.scrollTo({top:0});document.querySelector<HTMLInputElement>('.guide-search')?.focus({preventScroll:true});}}>回到目录</button></div><p className="guide-figure-hint">阅读提示：目录可展开到操作步骤；图示可点击或按 Enter 放大，Esc 关闭。历史界面图与示意图均有注释。</p><MarkdownPreview markdown={text} assets={{}} guideImages={guideImages} onImageOpen={(url,alt)=>setFigure({url,alt})}/></div></div>
 {figure&&<Modal title="查看指南图片" onClose={()=>setFigure(null)}><div className="guide-image-full"><p>{figure.alt}</p><img src={figure.url} alt={figure.alt}/></div></Modal>}</section>;
}
