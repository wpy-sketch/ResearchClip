import Markdown from 'react-markdown';
import type {Components,ExtraProps} from 'react-markdown';
import type {ComponentProps} from 'react';
const heading=(Tag:'h1'|'h2'|'h3'|'h4'|'h5'|'h6')=>function Heading({node,children}:ComponentProps<'h1'>&ExtraProps){return <Tag data-heading-offset={node?.position?.start.offset}>{children}</Tag>;};
const headings:Components={h1:heading('h1'),h2:heading('h2'),h3:heading('h3'),h4:heading('h4'),h5:heading('h5'),h6:heading('h6')};
import {assetNameValid} from './model';
import type {DocumentAsset} from './model';
export function MarkdownPreview({markdown,assets}:{markdown:string;assets:Record<string,DocumentAsset>}){
  return <section className="document-preview rounded-xl border border-edge p-5" aria-label="Markdown 实时预览"><Markdown skipHtml components={{...headings,a:({children})=><span className="underline">{children}</span>,img:({src,alt})=>{const name=src?.startsWith('assets/')?src.slice(7):'',asset=assetNameValid(name)?assets[name]:undefined;return asset?<img src={asset.url} alt={alt??''}/>:<span className="text-muted">[图片不可预览：{alt??'外部或缺失资源'}]</span>;}}}>{markdown}</Markdown></section>;
}
