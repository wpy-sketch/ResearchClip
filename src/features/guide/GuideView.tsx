import {useMemo,useRef,useState} from 'react';
import text from '../../../docs/USER_GUIDE.md?raw';
import {MarkdownPreview} from '../document/MarkdownPreview';
import {Modal} from '../../shared/ui/Modal';
import {documentOutline} from '../document/outline';
import projectsImage from '../../../docs/images/v021-projects.png';
import mathImage from '../../../docs/images/v021-math.png';
import captureImage from '../../../docs/images/v100-capture.png';
import library_gridImage from '../../../docs/images/v100-library-grid.png';
import library_listImage from '../../../docs/images/v100-library-list.png';
import editorImage from '../../../docs/images/v100-editor.png';
import cardImage from '../../../docs/images/v100-card.png';
import compareImage from '../../../docs/images/v100-compare.png';
import markdownImage from '../../../docs/images/v100-markdown.png';
import settingsImage from '../../../docs/images/v100-settings.png';
const guideImages={'images/v021-projects.png':projectsImage,'images/v021-math.png':mathImage,'images/v100-capture.png':captureImage,'images/v100-library-grid.png':library_gridImage,'images/v100-library-list.png':library_listImage,'images/v100-editor.png':editorImage,'images/v100-card.png':cardImage,'images/v100-compare.png':compareImage,'images/v100-markdown.png':markdownImage,'images/v100-settings.png':settingsImage};
const groups=[{name:'开始与采集',sections:[1,2,3]},{name:'资料与项目',sections:[4,5,7]},{name:'图片工具',sections:[6,8,12]},{name:'Markdown 写作',sections:[9,10,11]},{name:'设置与数据安全',sections:[13,14]}];
export function GuideView(){
 const headings=useMemo(()=>documentOutline(text).filter(h=>h.depth===2),[]),body=useRef<HTMLDivElement>(null),[query,setQuery]=useState(''),[active,setActive]=useState<number|null>(null),[figure,setFigure]=useState<{url:string;alt:string}|null>(null);
 const visible=headings.filter(h=>h.text.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()));
 const jump=(offset:number)=>{setActive(offset);body.current?.querySelector<HTMLElement>(`[data-heading-offset="${offset}"]`)?.scrollIntoView({block:'start'});};
 return <section className="guide-workspace" aria-label="完整使用指南"><header className="page-heading"><div><p className="text-xs text-accent">RESEARCHCLIP · 本地使用手册</p><h2>从图片到研究记录</h2><p>一步一步完成采集、标注、整理与写作。所有资料保存在本机。</p></div><span className="guide-edition">Windows · 完整使用指南</span></header><div className="guide-quick-start" aria-label="推荐阅读路线">{[{section:1,title:'01 · 第一次使用',text:'了解工作区和三种保存方式'},{section:5,title:'02 · 整理科研素材',text:'项目分类、多选和安全批量操作'},{section:9,title:'03 · 编写图文笔记',text:'插入图片、公式和保存 Markdown'}].map(item=><button key={item.section} onClick={()=>{setQuery('');const heading=headings.find(h=>Number(h.text.split('.')[0])===item.section);if(heading)jump(heading.offset);}}><strong>{item.title}</strong><span>{item.text}</span></button>)}</div>{query&&<div className="guide-search-feedback"><span>搜索章节：{query}</span><button className="secondary-button" onClick={()=>setQuery('')}>清除章节搜索</button></div>}<div className="guide-layout"><nav aria-label="使用指南目录"><p className="guide-directory-label">按任务查找 · 14 个章节</p><input className="guide-search" aria-label="搜索指南章节" value={query} onChange={e=>setQuery(e.target.value)} placeholder="搜索章节名称…"/>{groups.map(group=><div className="guide-topic-group" key={group.name}><h3>{group.name}</h3>{visible.filter(h=>group.sections.includes(Number(h.text.split('.')[0]))).map(h=><button key={h.offset} aria-current={active===h.offset?'location':undefined} onClick={()=>jump(h.offset)}>{h.text}</button>)}</div>)}{!visible.length&&<p className="text-xs text-muted">没有匹配的章节，试试“项目”或“保存”。</p>}</nav><div ref={body} className="guide-content"><p className="guide-figure-hint">操作图示可点击放大；支持键盘 Enter 打开、Esc 关闭。</p><MarkdownPreview markdown={text} assets={{}} guideImages={guideImages} onImageOpen={(url,alt)=>setFigure({url,alt})}/></div></div>{figure&&<Modal title="查看指南图片" onClose={()=>setFigure(null)}><div className="guide-image-full"><p>{figure.alt}</p><img src={figure.url} alt={figure.alt}/></div></Modal>}</section>;
}
