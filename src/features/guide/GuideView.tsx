import pdfSpreadImage from '../../../docs/images/v419-pdf-spread.png';
import pdfSearchImage from '../../../docs/images/v419-pdf-search.png';
import pdfMarkdownImage from '../../../docs/images/v419-pdf-markdown.png';
import citationLibraryImage from '../../../docs/images/v418-citation-library.png';
import citationCompletionImage from '../../../docs/images/v418-citation-completion.png';
import citationBibliographyImage from '../../../docs/images/v418-citation-bibliography.png';
import documentDirectoryImage from '../../../docs/images/v417-document-directory.png';
import documentHistoryImage from '../../../docs/images/v417-document-history.png';
import markdownPaperImage from '../../../docs/images/v416-markdown-paper.png';
import markdownTableImage from '../../../docs/images/v415-markdown-table.png';
import markdownVisualImage from '../../../docs/images/v414-markdown-visual.png';
import markdownToolbarImage from '../../../docs/images/v413-markdown-toolbar.png';
import commandTasksImage from '../../../docs/images/v412-command-tasks.png';
import commandPaletteImage from '../../../docs/images/v318-command-palette.png';
import anchorLinkImage from '../../../docs/images/v317-anchor-link.png';
import graphImage from '../../../docs/images/v315-graph.png';
import backlinksImage from '../../../docs/images/v315-backlinks.png';
import referencesImage from '../../../docs/images/v313-references-light.png';
import similarityScopesImage from '../../../docs/images/v313-similarity-scopes.png';
import similarityImage from '../../../docs/images/v312-similarity.png';
import similarityFlowImage from '../../../docs/images/v312-similarity-flow.svg';
import windowCaptureImage from '../../../docs/images/v301-window-capture.svg';
import syncSettingsImage from '../../../docs/images/v310-sync-settings.png';
import syncActionsImage from '../../../docs/images/v310-sync-actions.png';
import syncFlowImage from '../../../docs/images/v300-sync-flow.svg';
import guideDarkImage from '../../../docs/images/v316-guide-dark.png';
import guideLightImage from '../../../docs/images/v316-guide-light.png';
import revisionsImage from '../../../docs/images/v105-revisions.svg';
import mathOcrInputImage from '../../../docs/images/v104-math-ocr-input.png';
import mathOcrWorkflowImage from '../../../docs/images/v104-math-ocr-workflow.svg';
import ocrIndexImage from '../../../docs/images/v103-ocr-index.svg';
import {useEffect,useMemo,useRef,useState,useCallback} from 'react';
import {ChevronRight,Search,BookOpen} from 'lucide-react';
import text from '../../../docs/USER_GUIDE.md?raw';
import {MarkdownPreview} from '../document/MarkdownPreview';
import {Modal} from '../../shared/ui/Modal';
import {guideTopicGroups as groups,guideSections,searchGuide,guideOffsets,visibleGuideLocation,guideExamples,type GuideSection} from './navigation';
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
const guideImages={'images/v419-pdf-spread.png':pdfSpreadImage,'images/v419-pdf-search.png':pdfSearchImage,'images/v419-pdf-markdown.png':pdfMarkdownImage,'images/v418-citation-library.png':citationLibraryImage,'images/v418-citation-completion.png':citationCompletionImage,'images/v418-citation-bibliography.png':citationBibliographyImage,'images/v417-document-directory.png':documentDirectoryImage,'images/v417-document-history.png':documentHistoryImage,'images/v416-markdown-paper.png':markdownPaperImage,'images/v415-markdown-table.png':markdownTableImage,'images/v414-markdown-visual.png':markdownVisualImage,'images/v413-markdown-toolbar.png':markdownToolbarImage,'images/v412-command-tasks.png':commandTasksImage,'images/v318-command-palette.png':commandPaletteImage,'images/v317-anchor-link.png':anchorLinkImage,'images/v315-graph.png':graphImage,'images/v315-backlinks.png':backlinksImage,'images/v313-references-light.png':referencesImage,'images/v313-similarity-scopes.png':similarityScopesImage,'images/v312-similarity.png':similarityImage,'images/v312-similarity-flow.svg':similarityFlowImage,'images/v301-window-capture.svg':windowCaptureImage,'images/v310-sync-settings.png':syncSettingsImage,'images/v310-sync-actions.png':syncActionsImage,'images/v300-sync-flow.svg':syncFlowImage,'images/v316-guide-dark.png':guideDarkImage,'images/v316-guide-light.png':guideLightImage,'images/v105-revisions.svg':revisionsImage,'images/v104-math-ocr-input.png':mathOcrInputImage,'images/v104-math-ocr-workflow.svg':mathOcrWorkflowImage,'images/v103-ocr-index.svg':ocrIndexImage,'images/v102-clipper-settings.png':clipperSettingsImage,'images/v102-clipper-extension.png':clipperExtensionImage,'images/v101-scrolling.png':scrollingImage,'images/v021-projects.png':projectsImage,'images/v021-math.png':mathImage,'images/v100-capture.png':captureImage,'images/v100-library-grid.png':library_gridImage,'images/v100-library-list.png':library_listImage,'images/v100-editor.png':editorImage,'images/v100-card.png':cardImage,'images/v100-compare.png':compareImage,'images/v100-markdown.png':markdownImage,'images/v100-settings.png':settingsImage};
const guideAssets={};


function GuideBranch({nodes,depth,collapsed,active,searching,onToggle,onJump}:{nodes:GuideSection[];depth:number;collapsed:Set<number>;active:number|null;searching:boolean;onToggle:(offset:number)=>void;onJump:(offset:number)=>void}){
 return <ol className="guide-tree">{nodes.filter(n=>n.heading.depth<=depth).map(node=>{const h=node.heading,children=node.children.length>0&&h.depth<depth,expanded=searching||!collapsed.has(h.offset);return <li key={h.offset} data-guide-depth={h.depth}><div className="guide-tree-row">{children?<button className="guide-tree-toggle" aria-label={`${expanded?'收起':'展开'} ${h.text}`} aria-expanded={expanded} onClick={()=>onToggle(h.offset)}><ChevronRight size={13} style={{transform:expanded?'rotate(90deg)':undefined}}/></button>:<span className="guide-tree-spacer"/>}<button className="guide-tree-link" aria-current={active===h.offset?'location':undefined} onClick={()=>onJump(h.offset)}>{h.text}</button></div>{children&&expanded&&<GuideBranch nodes={node.children} depth={depth} collapsed={collapsed} active={active} searching={searching} onToggle={onToggle} onJump={onJump}/>}</li>;})}</ol>;
}
export function GuideView(){
 const sections=useMemo(()=>guideSections(text),[]),body=useRef<HTMLDivElement>(null),[query,setQuery]=useState(''),[examplesOnly,setExamplesOnly]=useState(false),[active,setActive]=useState<number|null>(null),[figure,setFigure]=useState<{url:string;alt:string}|null>(null),[depth,setDepth]=useState(3),[collapsed,setCollapsed]=useState<Set<number>>(new Set());
 const openImage=useCallback((url:string,alt:string)=>setFigure({url,alt}),[setFigure]);
 const examples=useMemo(()=>guideExamples(sections),[sections]);
 const exampleCount=examples.flatMap(s=>s.children.flatMap(c=>c.children)).length;
 const visible=useMemo(()=>searchGuide(examplesOnly?examples:sections,query),[sections,examples,examplesOnly,query]);
 const jump=(offset:number)=>{setActive(offset);const node=body.current?.querySelector<HTMLElement>(`[data-heading-offset="${offset}"]`);if(node){node.tabIndex=-1;node.scrollIntoView({block:'start'});node.focus({preventScroll:true});}};
 useEffect(()=>{
  const host=body.current?.closest<HTMLElement>('.workspace-scroll'),offsets=new Set(guideOffsets(sections,4));let frame=0;
  const update=()=>{frame=0;if(!host)return;const top=host.getBoundingClientRect().top+80;const nodes=Array.from(body.current?.querySelectorAll<HTMLElement>('[data-heading-offset]')??[]).filter(n=>offsets.has(Number(n.dataset.headingOffset)));const current=nodes.filter(n=>n.getBoundingClientRect().top<=top).at(-1)??nodes[0];if(current)setActive(Number(current.dataset.headingOffset));};
  const scroll=()=>{if(!frame)frame=requestAnimationFrame(update);};host?.addEventListener('scroll',scroll,{passive:true});scroll();return()=>{host?.removeEventListener('scroll',scroll);cancelAnimationFrame(frame);};
 },[sections]);
 const shownDepth=query.trim()||examplesOnly?4:depth,shownActive=visibleGuideLocation(visible,active,shownDepth,query.trim()||examplesOnly?new Set():collapsed);
 const chapterIndex=sections.findIndex(s=>s.heading.offset===visibleGuideLocation(sections,active,2,new Set()));
 const toggle=(offset:number)=>setCollapsed(previous=>{const next=new Set(previous);if(next.has(offset))next.delete(offset);else next.add(offset);return next;});
 return <section className="guide-workspace" aria-label="完整使用指南"><header className="page-heading"><div><p className="guide-eyebrow"><BookOpen size={14}/> RESEARCHCLIP · 使用手册</p><h2>从截图到研究成果</h2><p>选一个任务，照着示例完成。每项功能都列出操作与完成标志。</p></div><span className="guide-edition">v{__APP_VERSION__} · Windows</span></header>
 <div className="guide-quick-start" aria-label="推荐阅读路线">{[{section:1,title:'开始使用',text:'工作区 · 采集 · 三种保存',number:'01'},{section:5,title:'整理科研素材',text:'项目分类 · 标签 · 批量操作',number:'02'},{section:10,title:'编写图文笔记',text:'Markdown · 公式 · 导出',number:'03'},{section:14,title:'连接研究知识',text:'内部引用 · 反向链接 · 图谱',number:'04'},{section:6,title:'描述画面找图片',text:'普通搜索 · 离线语义 · 索引管理',number:'05'},{section:16,title:'减少重复操作',text:'命令面板 · 工作流 · 结果确认',number:'06'},{section:19,title:'安排多个编辑面板',text:'标签页 · 拆分 · 隐藏与恢复',number:'07'},{section:20,title:'查看命令任务与诊断',text:'最近文档 · 受控插图 · 取消与错误',number:'08'},{section:21,title:'格式化 Markdown 选区',text:'工具栏 · 查找替换 · 光标与撤销',number:'09'},{section:22,title:'保真编辑与斜杠插入',text:'视图切换 · 同步滚动 · 图文拖动',number:'10'},{section:23,title:'编辑表格、图片与代码',text:'网格 · 图文组 · 高亮与折叠',number:'11'},{section:24,title:'撰写数学论文',text:'公式模板 · 定理 · 编号与引用',number:'12'},{section:25,title:'组织科研文档',text:'目录收藏 · 双向链接 · 历史差异',number:'13'},{section:26,title:'记录科研引用',text:'@补全 · 公式来源 · BibTeX',number:'14'},{section:27,title:'原生工作区阅读PDF',text:'目录搜索 · 分栏 · 阅读位置',number:'15'}].sort((a,b)=>a.section-b.section).map((item,index)=><button key={item.section} onClick={()=>{setQuery('');setExamplesOnly(false);const section=sections.find(s=>s.chapter===item.section);if(section)jump(section.heading.offset);}}><span className="guide-route-number">{String(index+1).padStart(2,'0')}</span><div><strong>{item.title}</strong><span>{item.text}</span></div><ChevronRight size={16}/></button>)}</div>
 <div className="guide-layout"><nav aria-label="使用指南目录"><div className="guide-directory-heading"><strong>使用指南目录</strong><span>{sections.length} 章</span></div><div className="guide-directory-mode" role="group" aria-label="目录内容"><button aria-pressed={!examplesOnly} onClick={()=>setExamplesOnly(false)}>操作目录</button><button aria-pressed={examplesOnly} onClick={()=>setExamplesOnly(true)}>功能示例 <span>{exampleCount}</span></button></div><label className="guide-search-box"><Search size={15}/><input className="guide-search" aria-label="搜索指南章节" value={query} onChange={e=>setQuery(e.target.value)} placeholder={examplesOnly?'搜索示例，如“箭头”“DOI”…':'搜索标题或操作关键词…'}/></label><div className="guide-directory-tools"><label>目录层级<select aria-label="指南目录层级" disabled={examplesOnly} value={depth} onChange={e=>setDepth(Number(e.target.value))}><option value={2}>章节</option><option value={3}>章节与小节</option><option value={4}>详细操作步骤</option></select></label><div hidden={examplesOnly}><button onClick={()=>setCollapsed(new Set())}>展开全部</button><button onClick={()=>setCollapsed(new Set(sections.map(s=>s.heading.offset)))}>收起全部</button></div></div>
 {query&&<div className="guide-search-feedback" role="status"><span>{visible.length?`找到 ${visible.length} 个相关章节`:'没有匹配结果'}</span><button onClick={()=>setQuery('')}>清除</button></div>}
 {groups.map(group=>{const nodes=visible.filter(s=>group.sections.includes(s.chapter));return nodes.length?<div className="guide-topic-group" key={group.name}><h3>{group.name}</h3><GuideBranch nodes={nodes} depth={shownDepth} collapsed={collapsed} active={shownActive} searching={Boolean(query.trim())||examplesOnly} onToggle={toggle} onJump={jump}/></div>:null;})}{!visible.length&&<p className="guide-no-result">试试“版本”“插图”或“快捷键”。搜索会保留匹配小节的上级目录。</p>}</nav>
 <div ref={body} className="guide-content"><div className="guide-reading-bar"><span>{sections[chapterIndex]?.heading.text??"阅读使用指南"}</span><span className="guide-position">{chapterIndex+1} / {sections.length} 章</span><button className="secondary-button" onClick={()=>{body.current?.closest<HTMLElement>('.workspace-scroll')?.scrollTo({top:0});document.querySelector<HTMLInputElement>('.guide-search')?.focus({preventScroll:true});}}>回到目录</button></div><p className="guide-figure-hint">阅读提示：用“功能示例”快速演练，用“操作目录”查看完整步骤。示例需自行准备练习资料；图片按 Enter 放大，Esc 关闭。</p><MarkdownPreview markdown={text} assets={guideAssets} guideImages={guideImages} onImageOpen={openImage}/><nav className="guide-chapter-actions" aria-label="指南章节翻页"><button className="secondary-button" disabled={chapterIndex<=0} onClick={()=>jump(sections[chapterIndex-1].heading.offset)}>上一章</button><button className="secondary-button" disabled={chapterIndex>=sections.length-1} onClick={()=>jump(sections[chapterIndex+1].heading.offset)}>下一章</button></nav></div></div>
 {figure&&<Modal title="查看指南图片" onClose={()=>setFigure(null)}><div className="guide-image-full"><p>{figure.alt}</p><img src={figure.url} alt={figure.alt}/></div></Modal>}</section>;
}
