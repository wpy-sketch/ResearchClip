import Markdown from 'react-markdown';
import {assetNameValid} from './model';
import type {DocumentAsset} from './model';
export function MarkdownPreview({markdown,assets}:{markdown:string;assets:Record<string,DocumentAsset>}){
  return <section className="document-preview rounded-xl border border-edge p-5" aria-label="Markdown 实时预览"><Markdown skipHtml components={{a:({children})=><span className="underline">{children}</span>,img:({src,alt})=>{const name=src?.startsWith('assets/')?src.slice(7):'',asset=assetNameValid(name)?assets[name]:undefined;return asset?<img src={asset.url} alt={alt??''}/>:<span className="text-muted">[图片不可预览：{alt??'外部或缺失资源'}]</span>;}}}>{markdown}</Markdown></section>;
}
