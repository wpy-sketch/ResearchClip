import {CodeBlock} from './CodeBlock';
import './content-tools.css';
import './paper-tools.css';
import remarkGfm from 'remark-gfm';
import {remarkHighlight} from './highlight';
import Markdown,{defaultUrlTransform,type Components,type ExtraProps} from 'react-markdown';
import {internalTarget,type Target} from '../links/model';
import remarkMath from 'remark-math';
import {MathFormula} from './MathFormula';
import type {Root,RootContent} from 'mdast';
import {memo,useMemo,useRef,type ComponentProps,type ReactNode} from 'react';
import {assetNameValid,type DocumentAsset} from './model';
import {analyzePaper,theoremNames,type PaperAnalysis} from './paper-model';
const emptyGuideImages:Record<string,string>={};
interface Props {markdown:string;assets:Record<string,DocumentAsset>;guideImages?:Record<string,string>;onImageOpen?:(url:string,alt:string)=>void;onInternalOpen?:(target:Target)=>void;paper?:PaperAnalysis;sourceOffset?:number;showDiagnostics?:boolean}
function mathBudget(paper:PaperAnalysis,base:number){return ()=> (tree:Root)=>{const walk=(node:Root|RootContent)=>{if(node.type==='math'||node.type==='inlineMath'){const from=base+(node.position?.start.offset??0),item=paper.items.find(i=>i.from===from&&i.kind==='equation');const properties:Record<string,string|number>={'data-paper-from':from};if(paper.enabled&&item){if(item.number&&!item.manual)properties['data-paper-number']=item.number;if(item.label)properties['data-paper-target']=item.label;}if(paper.mathErrors.has(from))properties['data-math-error']='本次预览公式超过256个或128KiB限制；源码仍完整保留。';node.data??={};node.data.hProperties={...node.data.hProperties,...properties};if(node.type==='math'&&node.data.hChildren?.[0]?.type==='element')Object.assign(node.data.hChildren[0].properties,properties);}if('children'in node)node.children.forEach(child=>walk(child as RootContent));};walk(tree);};}
export const MarkdownPreview=memo(function MarkdownPreview({markdown,assets,guideImages=emptyGuideImages,onImageOpen,onInternalOpen,paper:provided,sourceOffset=0,showDiagnostics=true}:Props){
 const own=useMemo(()=>provided??analyzePaper(markdown),[provided,markdown]),host=useRef<HTMLElement>(null);
 const reveal=(label:string)=>{const root=host.current?.closest('.md-visual-editor,.document-page')??host.current;const target=Array.from(root?.querySelectorAll<HTMLElement>('[data-paper-target]')??[]).find(n=>n.dataset.paperTarget===label);target?.scrollIntoView({block:'center',behavior:'auto'});if(target){target.tabIndex=-1;target.focus({preventScroll:true});}};
 const render=(text:string,base:number)=>{
  const target=(offset:number|undefined,kind:string)=>own.enabled?own.items.find(i=>i.from===base+(offset??0)&&i.kind===kind):undefined;
  const components:Components={
   ...Object.fromEntries((['h1','h2','h3','h4','h5','h6'] as const).map(Tag=>[Tag,({node,children}:ComponentProps<'h1'>&ExtraProps)=><Tag data-heading-offset={base+(node?.position?.start.offset??0)}>{children}</Tag>])),
   p:({children,node})=>{const item=target(node?.position?.start.offset,'figure');return item?<div className="paper-figure" data-paper-target={item.label} data-paper-from={item.from}><p>{children}</p><small>图 {item.number??'—'}{item.title&&item.title!=='图'?' · '+item.title:''}</small></div>:<p>{children}</p>;},
   table:({children,node})=>{const item=target(node?.position?.start.offset,'table');return item?<div className="paper-table" data-paper-target={item.label} data-paper-from={item.from}><small>表 {item.number??'—'}</small><table>{children}</table></div>:<table>{children}</table>;},
   pre:({children,node})=>{const code=node?.children[0];if(code?.type!=='element'||code.tagName!=='code')return <pre>{children}</pre>;const classes=Array.isArray(code.properties.className)?code.properties.className.map(String):[];if(classes.includes('math-display'))return <>{children}</>;const language=classes.find(c=>c.startsWith('language-'))?.slice(9)??'',text=code.children.map(n=>n.type==='text'?n.value:'').join('').replace(/\n$/,'');return <CodeBlock code={text} language={language}/>;},
   code:({children,className,node})=>{if(!className?.split(' ').some(c=>c==='math-inline'||c==='math-display'))return <code className={className}>{children}</code>;const p=node?.properties,number=p?.['data-paper-number'];return <span className={className.includes('math-display')?'paper-equation':undefined} data-paper-target={p?.['data-paper-target']} data-paper-from={p?.['data-paper-from']}><MathFormula source={String(children).replace(/\n$/,'')} display={className.includes('math-display')} error={p?.['data-math-error'] as string|undefined}/>{number&&<span className="paper-equation-number" aria-label={`公式编号 ${number}`}>({number})</span>}</span>;},
   a:({children,href})=>{if(href?.startsWith('#rc-ref:')){const label=href.slice(8),matches=own.labels.get(label);return matches?.length===1?<button type="button" className="paper-reference" data-paper-reference={label} onClick={()=>reveal(label)} title={`跳转到 ${label}`}>{children}{matches[0].number?' '+matches[0].number:''}</button>:<span className="paper-reference-error" title={`${matches?'重复':'缺失'}标签：${label}`}>{children} [{label} · {matches?'重复':'缺失'}]</span>;}const destination=href?internalTarget(href):null;return destination&&onInternalOpen?<button type="button" className="internal-link" title="打开内部引用" onClick={()=>onInternalOpen(destination)}>{children}</button>:<span className="underline">{children}</span>;},
   img:({src,alt,title})=>{const name=src?.startsWith('assets/')?src.slice(7):'',asset=assetNameValid(name)?assets[name]:undefined,url=asset?.url??(src&&Object.hasOwn(guideImages,src)?guideImages[src]:undefined);return url?(onImageOpen?<button className="guide-image-open" aria-label={`放大图片：${alt??'操作图示'}`} onClick={()=>onImageOpen(url,alt??'操作图示')}><img src={url} alt={alt??''} title={title} loading="lazy"/></button>:<img src={url} alt={alt??''} title={title} loading="lazy"/>):<span className="text-muted">[图片不可预览：{alt??'外部或缺失资源'}]</span>;}
  };
  return <Markdown skipHtml urlTransform={url=>url.startsWith('#rc-ref:')||internalTarget(url)?url:defaultUrlTransform(url)} remarkPlugins={[remarkMath,remarkGfm,remarkHighlight,mathBudget(own,base)]} components={components}>{text+(own.definitions.length?'\n\n'+own.definitions.join('\n\n'):'')}</Markdown>;
 };
 const parts:ReactNode[]=[];let cursor=0;
 for(const container of own.containers.filter(c=>c.from>=sourceOffset&&c.to<=sourceOffset+markdown.length)){
  const from=container.from-sourceOffset,to=container.to-sourceOffset;parts.push(<div key={`before:${from}`}>{render(markdown.slice(cursor,from),sourceOffset+cursor)}</div>);
  const item=own.items.find(i=>i.from===container.from&&i.kind==='theorem');
  parts.push(container.valid?<section key={from} className={`paper-theorem paper-${container.subtype}`} data-paper-target={item?.label} data-paper-from={container.from}><header>{theoremNames[container.subtype]}{item?.number?' '+item.number:''}{container.title?' · '+container.title:''}</header>{render(markdown.slice(container.bodyFrom-sourceOffset,container.bodyTo-sourceOffset),container.bodyFrom)}{container.subtype==='proof'&&<span className="paper-qed" aria-label="证毕">□</span>}</section>:<pre key={from} className="paper-invalid-source">{markdown.slice(from,to)}</pre>);cursor=to;
 }
 parts.push(<div key="tail">{render(markdown.slice(cursor),sourceOffset+cursor)}</div>);
 return <section ref={host} className="document-preview rounded-xl border border-edge p-5" aria-label="Markdown 实时预览">{parts}{showDiagnostics&&own.enabled&&own.diagnostics.length>0&&<details className="paper-diagnostics"><summary>论文诊断（{own.diagnostics.length}）</summary><ul>{own.diagnostics.slice(0,100).map((d,i)=><li key={i}>第{own.raw.slice(0,d.from).split(/\r\n|\n|\r/).length}行：{d.message}</li>)}</ul></details>}</section>;
});
