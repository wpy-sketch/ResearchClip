import {useMemo,useRef} from 'react';
import text from '../../../docs/USER_GUIDE.md?raw';
import {MarkdownPreview} from '../document/MarkdownPreview';
import {documentOutline} from '../document/outline';
export function GuideView(){
  const headings=useMemo(()=>documentOutline(text).filter(h=>h.depth===2),[]),body=useRef<HTMLDivElement>(null);
  return <section className="guide-workspace" aria-label="完整使用指南"><header className="page-heading"><div><p className="text-xs text-accent">RESEARCHCLIP · 本地使用手册</p><h2>从图片到研究记录</h2><p>采集、整理、比较与写作，在一个本地工作区中完成。</p></div></header><div className="guide-layout"><nav aria-label="使用指南目录">{headings.map(h=><button key={h.offset} onClick={()=>body.current?.querySelector<HTMLElement>(`[data-heading-offset="${h.offset}"]`)?.scrollIntoView({block:'start'})}>{h.text}</button>)}</nav><div ref={body} className="guide-content"><MarkdownPreview markdown={text} assets={{}}/></div></div></section>;
}
