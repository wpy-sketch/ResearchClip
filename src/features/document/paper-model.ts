import {fromMarkdown} from 'mdast-util-from-markdown';
import {math} from 'micromark-extension-math';
import {mathFromMarkdown} from 'mdast-util-math';
import {gfm} from 'micromark-extension-gfm';
import {gfmFromMarkdown} from 'mdast-util-gfm';
export const theoremNames={definition:'定义',assumption:'假设',lemma:'引理',theorem:'定理',corollary:'推论',proof:'证明',example:'例子',counterexample:'反例'} as const;
export type TheoremKind=keyof typeof theoremNames;
export type PaperKind='equation'|'theorem'|'figure'|'table';
export interface PaperConfig {version:1;scope:'document'|'section';headingDepth:1|2;theoremCounter:'shared'|'byType';proofNumbered:boolean;equationStart:number;theoremStart:number;figureStart:number;tableStart:number}
export const defaultPaperConfig:PaperConfig={version:1,scope:'document',headingDepth:1,theoremCounter:'shared',proofNumbered:false,equationStart:1,theoremStart:1,figureStart:1,tableStart:1};
export interface AstNode {type:string;value?:string;url?:string;alt?:string;depth?:number;children?:AstNode[];position?:{start:{offset?:number};end:{offset?:number}}}
export interface PaperContainer {from:number;to:number;bodyFrom:number;bodyTo:number;subtype:TheoremKind;title:string;valid:boolean}
export interface PaperItem {from:number;to:number;kind:PaperKind;subtype?:TheoremKind;title:string;number:string|null;label?:string;manual?:boolean}
export interface PaperDiagnostic {from:number;message:string}
export interface PaperAnalysis {raw:string;enabled:boolean;config:PaperConfig;configValid:boolean;configs:{from:number;to:number}[];containers:PaperContainer[];items:PaperItem[];labels:Map<string,PaperItem[]>;diagnostics:PaperDiagnostic[];mathErrors:Set<number>;definitions:string[]}
export const labelValid=(value:string)=>/^[A-Za-z][A-Za-z0-9_.:-]{0,63}$/.test(value);
export const paperAst=(raw:string)=>fromMarkdown(raw,{extensions:[math(),gfm()],mdastExtensions:[mathFromMarkdown(),gfmFromMarkdown()]}) as unknown as AstNode;
const begin=(node:AstNode)=>node.position?.start.offset??0,end=(node:AstNode)=>node.position?.end.offset??0;
function walk(node:AstNode,visit:(node:AstNode)=>void){visit(node);node.children?.forEach(n=>walk(n,visit));}
export function parsePaperConfig(value:unknown):PaperConfig{
 if(!value||typeof value!=='object'||Array.isArray(value))throw new Error('论文配置必须是版本1的对象。');
 const c=value as Record<string,unknown>,keys=Object.keys(defaultPaperConfig);
 if(Object.keys(c).some(k=>!keys.includes(k))||c.version!==1)throw new Error('论文配置包含未知字段或版本，未套用。');
 const next={...defaultPaperConfig,...c} as PaperConfig;
 if(!['document','section'].includes(next.scope)||![1,2].includes(next.headingDepth)||!['shared','byType'].includes(next.theoremCounter)||typeof next.proofNumbered!=='boolean'||[next.equationStart,next.theoremStart,next.figureStart,next.tableStart].some(n=>!Number.isSafeInteger(n)||n<1||n>9999))throw new Error('论文编号规则或起始值无效。');
 return next;
}
export function paperContainers(raw:string,tree=paperAst(raw)){
 const excluded:{from:number;to:number}[]=[];walk(tree,n=>{if(['code','math','html'].includes(n.type))excluded.push({from:begin(n),to:end(n)});});
 const containers:PaperContainer[]=[],diagnostics:PaperDiagnostic[]=[];let open:PaperContainer|null=null,depth=0;
 for(const match of raw.matchAll(/^[^\r\n]*(?:\r\n|\n|\r|$)/gm)){
  if(!match[0])continue;const line=match[0].replace(/(?:\r\n|\n|\r)$/,''),from=match.index;
  if(excluded.some(r=>from>=r.from&&from<r.to))continue;
  const header=/^:::([a-z]+)(?:[ \t]+([^\r\n]{0,200}))?[ \t]*$/.exec(line);
  if(header&&Object.hasOwn(theoremNames,header[1])){
   if(open){depth++;open.valid=false;diagnostics.push({from,message:'不支持嵌套定理容器，保留完整原文。'});continue;}
   open={from,to:raw.length,bodyFrom:from+match[0].length,bodyTo:raw.length,subtype:header[1] as TheoremKind,title:(header[2]??'').trim(),valid:true};
   depth=1;
  }else if(line.trim()===':::'&&open){if(--depth===0){open.bodyTo=from;open.to=from+line.length;containers.push(open);open=null;}}
  else if(open&&/^:::/.test(line)){depth++;open.valid=false;diagnostics.push({from,message:'定理内有未知容器，保留原文并暂停环境排版。'});}
 }
 if(open){open.valid=false;containers.push(open);diagnostics.push({from:open.from,message:'定理环境未闭合，原文保留。'});}
 return {containers,diagnostics};
}
function manualTag(value:string){
 const text=value.split(/\r\n|\n|\r/).map(line=>{for(let i=0;i<line.length;i++)if(line[i]==='%'){let slashes=0;for(let j=i-1;j>=0&&line[j]==='\\';j--)slashes++;if(slashes%2===0)return line.slice(0,i);}return line;}).join('\n');
 const tag=/(?<!\\)\\tag\*?\s*\{/.exec(text);
 if(tag){let level=1,i=tag.index+tag[0].length;const start=i;for(;i<text.length;i++){if(text[i]==='\\'){i++;continue;}if(text[i]==='{')level++;if(text[i]==='}'&&--level===0)break;}return {manual:true,number:text.slice(start,i).slice(0,100)};}
 return {manual:/(?<!\\)\\(?:notag|nonumber)\b/.test(text),number:null};
}
export function analyzePaper(raw:string):PaperAnalysis{
 const result:PaperAnalysis={raw,enabled:false,config:{...defaultPaperConfig},configValid:true,configs:[],containers:[],items:[],labels:new Map(),diagnostics:[],mathErrors:new Set(),definitions:[]};
 if(raw.length>2*1024*1024){result.enabled=true;result.diagnostics.push({from:0,message:'文稿超出2Mi字符分析预算，保留源码。'});return result;}
 const tree=paperAst(raw),scanned=paperContainers(raw,tree);result.containers=scanned.containers;result.diagnostics.push(...scanned.diagnostics);
 const excluded=(at:number)=>result.containers.some(c=>!c.valid&&at>=c.from&&at<c.to);
 const front=/^\uFEFF?(---|\+\+\+)[ \t]*\r?\n/.exec(raw);let metadataEnd=0;
 if(front){const close=new RegExp('^'+front[1].replaceAll('+','\\+')+'[ \\t]*\\r?$','gm');close.lastIndex=front[0].length;const m=close.exec(raw);metadataEnd=m?m.index+m[0].length:raw.length;}
 const markers:{from:number;to:number;label:string}[]=[],references:{from:number;label:string}[]=[],equations=new Map<number,string>();let count=0,total=0;
 walk(tree,node=>{
  const from=begin(node);if(from<metadataEnd||excluded(from))return;
  if(node.type==='definition')result.definitions.push(raw.slice(from,end(node)));
  if(node.type==='html'){
   const value=node.value?.trim()??'';
   if(value.startsWith('<!-- rc-paper')){
    result.configs.push({from,to:end(node)});try{const m=/^<!-- rc-paper ([^\r\n]{1,4096}) -->$/.exec(value);if(!m)throw new Error('论文配置语法无效，原文保留。');result.config=parsePaperConfig(JSON.parse(m[1]));}catch(e){result.configValid=false;result.diagnostics.push({from,message:e instanceof Error?e.message:'论文配置错误。'});}
   }
   if(value.startsWith('<!-- rc-label:')){const m=/^<!-- rc-label: ([A-Za-z][A-Za-z0-9_.:-]{0,63}) -->$/.exec(value);if(m)markers.push({from,to:end(node),label:m[1]});else result.diagnostics.push({from,message:'文内标签语法无效，原注释保留。'});}
  }
  if(node.type==='math'||node.type==='inlineMath'){count++;total+=(node.value??'').length;if(count>256||total>131072)result.mathErrors.add(from);}
  if(node.type==='math'){result.items.push({from,to:end(node),kind:'equation',title:'公式',number:null});equations.set(from,node.value??'');}
  if(node.type==='table')result.items.push({from,to:end(node),kind:'table',title:'表',number:null});
  if(node.type==='paragraph'&&node.children?.some(n=>n.type==='image'||n.type==='imageReference')&&node.children.every(n=>['image','imageReference'].includes(n.type)||n.type==='text'&&!n.value?.trim()))result.items.push({from,to:end(node),kind:'figure',title:node.children.find(n=>n.type==='image'||n.type==='imageReference')?.alt??'图',number:null});
  if(node.type==='link'&&node.url?.startsWith('#rc-ref:'))references.push({from,label:node.url.slice(8)});
 });
 for(const c of result.containers.filter(c=>c.valid))result.items.push({from:c.from,to:c.to,kind:'theorem',subtype:c.subtype,title:c.title,number:null});
 result.items.sort((a,b)=>a.from-b.from);
 result.enabled=result.configs.length>0||markers.length>0||references.length>0||result.containers.length>0;
 if(!result.enabled)return result;
 if(result.configs.length>1){result.configValid=false;result.diagnostics.push({from:result.configs[1].from,message:'每篇只能有一条论文配置，未套用重复配置。'});}
 if(result.items.length>2048){result.items=[];result.diagnostics.push({from:0,message:'论文对象超过2048个，暂停编号并保留原文。'});return result;}
 const headings=(tree.children??[]).filter(n=>n.type==='heading'&&n.depth===result.config.headingDepth&&begin(n)>=metadataEnd&&!result.containers.some(c=>begin(n)>c.from&&begin(n)<c.to));
 let section=0,h=0;const counters=new Map<string,number>();
 for(const item of result.items){
  while(h<headings.length&&begin(headings[h])<item.from){section++;h++;if(result.config.scope==='section')counters.clear();}
  if(!result.configValid)continue;
  if(item.kind==='equation'){const tag=manualTag(equations.get(item.from)??'');if(tag.manual){item.manual=true;item.number=tag.number;continue;}}
  if(item.subtype==='proof'&&!result.config.proofNumbered)continue;
  const key=item.kind==='theorem'&&result.config.theoremCounter==='byType'?'theorem:'+item.subtype:item.kind,start=result.config[(item.kind+'Start') as 'equationStart'],number=counters.get(key)??start;counters.set(key,number+1);item.number=(result.config.scope==='section'?section+'.':'')+number;
 }
 for(const marker of markers){const item=result.items.find(i=>i.from>=marker.to&&/^\s*$/.test(raw.slice(marker.to,i.from)));if(!item){result.diagnostics.push({from:marker.from,message:'孤立标签 '+marker.label+'：未紧邻可编号对象。'});continue;}if(item.label){result.diagnostics.push({from:marker.from,message:'同一对象不能有多个文内标签。'});continue;}item.label=marker.label;const matches=result.labels.get(marker.label)??[];matches.push(item);result.labels.set(marker.label,matches);}
 for(const [label,items]of result.labels)if(items.length>1)result.diagnostics.push({from:items[0].from,message:'重复标签 '+label+'：引用不会猜测目标。'});
 for(const ref of references)if(!labelValid(ref.label)||!result.labels.has(ref.label))result.diagnostics.push({from:ref.from,message:'缺失引用 '+ref.label+'：保留原链接文字。'});
 return result;
}
export function mathAt(raw:string,caret:number):{from:number;to:number;innerFrom:number;innerTo:number;display:boolean}|null{let found:{from:number;to:number;innerFrom:number;innerTo:number;display:boolean}|null=null;walk(paperAst(raw),n=>{if((n.type==='math'||n.type==='inlineMath')&&caret>=begin(n)&&caret<=end(n)){const from=begin(n),to=end(n),width=/^\$+/.exec(raw.slice(from,to))?.[0].length??0;if(width&&raw.slice(from,to).endsWith('$'.repeat(width)))found={from,to,innerFrom:from+width,innerTo:to-width,display:n.type==='math'};}});return found;}
export function configText(config:PaperConfig){return '<!-- rc-paper '+JSON.stringify(parsePaperConfig(config))+' -->';}
export function replaceConfig(raw:string,config:PaperConfig){const analysis=analyzePaper(raw);if(!analysis.configValid)throw new Error('现有配置无效或重复，请先在源码中修复。');const marker=configText(config),range=analysis.configs[0];if(range)return raw.slice(0,range.from)+marker+raw.slice(range.to);const eol=raw.match(/\r\n|\n|\r/)?.[0]??'\n';let at=raw.startsWith('\uFEFF')?1:0;const front=/^(---|\+\+\+)[ \t]*\r?\n/.exec(raw.slice(at));if(front){const close=new RegExp('^'+front[1].replaceAll('+','\\+')+'[ \\t]*(?:\\r\\n|\\n|\\r|$)','gm');close.lastIndex=at+front[0].length;const match=close.exec(raw);if(!match)throw new Error('前置元数据未闭合，请先修复源码。');at=match.index+match[0].length;}return raw.slice(0,at)+marker+eol+eol+raw.slice(at);}
