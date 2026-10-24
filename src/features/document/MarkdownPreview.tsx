import remarkGfm from 'remark-gfm';
import {remarkHighlight} from './highlight';
import Markdown,{defaultUrlTransform} from 'react-markdown';
import {internalTarget,type Target} from '../links/model';
import remarkMath from 'remark-math';
import {MathFormula} from './MathFormula';
import type {Root,RootContent} from 'mdast';
import type {Components,ExtraProps} from 'react-markdown';
import {useMemo,type ComponentProps} from 'react';
const emptyGuideImages:Record<string,string>={};
const heading=(Tag:'h1'|'h2'|'h3'|'h4'|'h5'|'h6')=>function Heading({node,children}:ComponentProps<'h1'>&ExtraProps){return <Tag data-heading-offset={node?.position?.start.offset}>{children}</Tag>;};
const headings:Components={h1:heading('h1'),h2:heading('h2'),h3:heading('h3'),h4:heading('h4'),h5:heading('h5'),h6:heading('h6')};
import {assetNameValid} from './model';
import type {DocumentAsset} from './model';
function mathBudget(){return (tree:Root)=>{let count=0,total=0;const walk=(node:Root|RootContent)=>{if(node.type==='math'||node.type==='inlineMath'){count++;total+=node.value.length;if(count>256||total>131072){node.data??={};node.data.hProperties??={};node.data.hProperties['data-math-error']='本次预览公式超过 256 个或 128 KiB 限制；源码仍完整保留。';if(node.type==='math'&&node.data.hChildren?.[0]?.type==='element')node.data.hChildren[0].properties['data-math-error']=node.data.hProperties['data-math-error'];}}if('children'in node)node.children.forEach(child=>walk(child as RootContent));};walk(tree);};}
export function MarkdownPreview({markdown,assets,guideImages=emptyGuideImages,onImageOpen,onInternalOpen}:{markdown:string;assets:Record<string,DocumentAsset>;guideImages?:Record<string,string>;onImageOpen?:(url:string,alt:string)=>void;onInternalOpen?:(target:Target)=>void}){
  const components=useMemo<Components>(()=>({...headings,
    code:({children,className,node})=>className?.split(' ').some(c=>c==='math-inline'||c==='math-display')?<MathFormula source={String(children).replace(/\n$/,'')} display={className.includes('math-display')} error={node?.properties['data-math-error'] as string|undefined}/>:<code className={className}>{children}</code>,
    a:({children,href})=>{const target=href?internalTarget(href):null;return target&&onInternalOpen?<button type="button" className="internal-link" title="打开内部引用" onClick={()=>onInternalOpen(target)}>{children}</button>:<span className="underline">{children}</span>;},
    img:({src,alt})=>{const name=src?.startsWith('assets/')?src.slice(7):'',asset=assetNameValid(name)?assets[name]:undefined,url=asset?.url??(src&&Object.hasOwn(guideImages,src)?guideImages[src]:undefined);return url?(onImageOpen?<button className="guide-image-open" aria-label={`放大图片：${alt??'操作图示'}`} onClick={()=>onImageOpen(url,alt??'操作图示')}><img src={url} alt={alt??''} loading="lazy"/></button>:<img src={url} alt={alt??''} loading="lazy"/>):<span className="text-muted">[图片不可预览：{alt??'外部或缺失资源'}]</span>;}
  }),[assets,guideImages,onImageOpen,onInternalOpen]);
  return <section className="document-preview rounded-xl border border-edge p-5" aria-label="Markdown 实时预览"><Markdown skipHtml urlTransform={url=>internalTarget(url)?url:defaultUrlTransform(url)} remarkPlugins={[remarkMath,remarkGfm,remarkHighlight,mathBudget]} components={components}>{markdown}</Markdown></section>;
}
