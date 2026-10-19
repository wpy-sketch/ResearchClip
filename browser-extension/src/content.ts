import DOMPurify from 'dompurify';
import {Readability} from '@mozilla/readability';
import TurndownService from 'turndown';
import {newId,safeUrl,type Mode,type Extracted,type ClipImage} from './types';
declare global {interface Window {__researchClipExtract?:(mode:Mode)=>Extracted}}
const clean=(html:string)=>DOMPurify.sanitize(html,{ALLOWED_TAGS:['p','div','article','section','main','h1','h2','h3','h4','h5','h6','ul','ol','li','blockquote','pre','code','strong','b','em','i','a','img','br','hr','span'],ALLOWED_ATTR:['href','src','alt','title','class'],ALLOW_DATA_ATTR:false,ALLOW_ARIA_ATTR:false,FORBID_TAGS:['form','input','textarea','select','button','script','style','iframe','object','embed','svg','math','video','audio'],FORBID_CONTENTS:['form','textarea','select','button','script','style','iframe','object','embed','svg','math'],FORBID_ATTR:['srcset']});
function convert(html:string):{markdown:string;images:ClipImage[]}{
 const doc=new DOMParser().parseFromString(clean(html),'text/html');const images:ClipImage[]=[];
 for(const link of doc.querySelectorAll('a')){const raw=link.getAttribute('href'),href=raw?safeUrl(raw,location.href):null;if(href)link.setAttribute('href',href);else link.removeAttribute('href');}
 for(const img of doc.querySelectorAll('img')){
  const raw=img.getAttribute('src'),url=raw?safeUrl(raw,location.href):null;if(!url||images.length>=32){img.replaceWith(doc.createTextNode('[图片来源不可用或超过32张上限]'));continue;}
  const id=newId();img.setAttribute('src',`clip-image://${id}`);images.push({id,url,alt:(img.alt||'网页图片').slice(0,200),pngBase64:null,warning:null,width:0,height:0,thumbnail:null});
 }
 const down=new TurndownService({headingStyle:'atx',codeBlockStyle:'fenced',bulletListMarker:'-',emDelimiter:'*'});
 down.addRule('safeImages',{filter:'img',replacement:(_content,node)=>{const e=node as HTMLImageElement;const alt=e.alt.replace(/[\\[\]\n\r]/g,' ');return `![${alt}](${e.src})`;}});
 down.addRule('sourceCode',{filter:'pre',replacement:(_content,node)=>{const text=node.textContent??'';const longest=Math.max(2,...(text.match(/`+/g)??[]).map(s=>s.length));const fence='`'.repeat(longest+1);return `\n\n${fence}\n${text}\n${fence}\n\n`;}});
 const markdown=down.turndown(doc.body).trim();if(new TextEncoder().encode(markdown).length>192*1024)throw Error('正文超过192 KiB，请改为选择文字剪藏。');return {markdown,images};
}
function extract(mode:Mode):Extracted {
 if(!safeUrl(location.href))throw Error('仅支持普通HTTP/HTTPS网页；不读取浏览器内部页、本地文件或扩展页。');
 const result:Extracted={title:(document.title||'未命名网页').slice(0,200).replace(/[\r\n\t]/g,' '),url:location.href,siteName:location.hostname,byline:'',mode,markdown:'',images:[]};
 if(mode==='link')return result;
 if(mode==='selection'){
  if(document.activeElement?.closest('input,textarea,select,[contenteditable="true"]'))throw Error('不剪藏输入框或表单中的敏感内容。请在网页正文选择文字。');
  const s=window.getSelection();if(!s||!s.rangeCount||s.isCollapsed||!s.toString().trim())throw Error('没有选中文字，请先在网页正文中选择内容。');
  const range=s.getRangeAt(0);const node=range.commonAncestorContainer;const parent=node instanceof Element?node:node.parentElement;
  if(parent?.closest('form,input,textarea,select,[contenteditable="true"]'))throw Error('不剪藏表单内容。');
  const box=document.createElement('div');box.append(range.cloneContents());if(box.innerHTML.length>2*1024*1024)throw Error('选中内容过大。');Object.assign(result,convert(box.innerHTML));
 }else if(mode==='article'){
  if(document.getElementsByTagName('*').length>20000)throw Error('页面过于复杂，请改为选中文字。');
  const clone=document.cloneNode(true) as Document;
  clone.querySelectorAll('script,style,form,input,textarea,select,button,iframe,object,embed,[contenteditable], [data-private]').forEach(n=>n.remove());
  if(clone.documentElement.outerHTML.length>2*1024*1024)throw Error('页面超过2 MiB，请改为选中文字。');
  const article=new Readability(clone,{maxElemsToParse:20000,charThreshold:120}).parse();
  if(!article?.content||!article.textContent?.trim())throw Error('未提取到可靠正文，请改为选中文字或保存链接。');
  result.byline=(article.byline??'').slice(0,200);result.siteName=(article.siteName??location.hostname).slice(0,200);Object.assign(result,convert(article.content));
 }else if(mode==='images'){
  const urls=new Set<string>();
  for(const img of document.images){const rect=img.getBoundingClientRect();const url=safeUrl(img.currentSrc||img.src,location.href);if(!url||urls.has(url)||rect.width<20||rect.height<20||img.closest('form,[data-private]'))continue;urls.add(url);
   let thumbnail:string|null=null;try{if(img.complete&&img.naturalWidth){const c=document.createElement('canvas');c.width=96;c.height=72;c.getContext('2d')?.drawImage(img,0,0,96,72);thumbnail=c.toDataURL('image/png');}}catch{/* Cross-origin pixels remain inaccessible; don't bypass. */}
   result.images.push({id:newId(),url,alt:(img.alt||'网页图片').slice(0,200),pngBase64:null,warning:null,width:img.naturalWidth,height:img.naturalHeight,thumbnail});if(result.images.length===32)break;
  }
  if(!result.images.length)throw Error('未找到兼容且可见的页面图片。');
 }
 return result;
}
// Installed only through activeTab after the user's explicit popup action, in ISOLATED world.
window.__researchClipExtract=extract;
