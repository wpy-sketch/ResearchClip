import {fromMarkdown} from 'mdast-util-from-markdown';
import {math} from 'micromark-extension-math';
import {mathFromMarkdown} from 'mdast-util-math';
import {projectMarkdown,normalizedText} from './projection';
import {assetNameValid,escapeInline} from './model';
export type Alignment='none'|'left'|'center'|'right';
export interface TableData {rows:string[][];align:Alignment[]}
export interface TableRange {from:number;to:number;data:TableData;eol:string;original:string}
const eolOf=(raw:string)=>raw.match(/\r\n|\n|\r/)?.[0]??'\n';
function cells(line:string){
 const result:string[]=[];let cell='',slashes=0;
 for(const c of line.trim()){if(c==='|'&&slashes%2===0){result.push(cell.trim());cell='';}else cell+=c;slashes=c==='\\'?slashes+1:0;}
 result.push(cell.trim());if(line.trim().startsWith('|'))result.shift();if(line.trim().endsWith('|')&&cell==='')result.pop();return result;
}
const blocked=(raw:string)=>fromMarkdown(raw,{extensions:[math()],mdastExtensions:[mathFromMarkdown()]}).children.filter(n=>['code','math','html'].includes(n.type)).map(n=>({from:n.position?.start.offset??0,to:n.position?.end.offset??0}));
export function tableAt(raw:string,caret:number):TableRange|null{
 const lines=Array.from(raw.matchAll(/[^\r\n]*(?:\r\n|\n|\r|$)/g)).filter(m=>m[0].length).map(m=>({from:m.index,to:m.index+m[0].replace(/(?:\r\n|\n|\r)$/,'').length,line:m[0].replace(/(?:\r\n|\n|\r)$/,'')})),excluded=blocked(raw);
 for(let i=0;i+1<lines.length;i++){
  if(!lines[i].line.includes('|')||excluded.some(r=>lines[i].from>=r.from&&lines[i].from<r.to))continue;
  const delimiter=cells(lines[i+1].line);if(!delimiter.length||!delimiter.every(c=>/^:?-{3,}:?$/.test(c)))continue;
  let end=i+2;while(end<lines.length&&lines[end].line.trim()&&lines[end].line.includes('|')&&!excluded.some(r=>lines[end].from>=r.from&&lines[end].from<r.to))end++;
  const from=lines[i].from,to=lines[end-1].to;if(caret<from||caret>to){i=end-1;continue;}
  const rows=[cells(lines[i].line),...lines.slice(i+2,end).map(l=>cells(l.line))];if(rows.some(row=>row.length!==delimiter.length))throw new Error('表格行列不规则，请在源码中修正；未截断任何单元格。');
  const data:TableData={rows,align:delimiter.map(c=>c.startsWith(':')?c.endsWith(':')?'center':'left':c.endsWith(':')?'right':'none')};validateTable(data);return {from,to,data,eol:eolOf(raw.slice(from,to)),original:raw.slice(from,to)};
 }
 return null;
}
export function validateTable(data:TableData){if(!data.rows.length||data.rows.length>101||!data.align.length||data.align.length>20||data.rows.some(r=>r.length!==data.align.length||r.some(c=>typeof c!=='string'||c.length>4096||/[\r\n\0]/.test(c)))||data.align.some(a=>!['none','left','center','right'].includes(a)))throw new Error('表格限101行（含表头）、20列，单元格最多4096字且不能含换行；原文保留。');}
export function createTable(rows:number,columns:number):TableData{if(!Number.isInteger(rows)||!Number.isInteger(columns)||rows<1||columns<1||rows>101||columns>20)throw new Error('表格尺寸无效。');return {rows:Array.from({length:rows},(_,r)=>Array.from({length:columns},(_,c)=>r===0?`列${c+1}`:'')),align:Array.from({length:columns},()=>'none')};}
function escapePipes(value:string){let result='',slashes=0;for(const c of value){if(c==='|'&&slashes%2===0)result+='\\';result+=c;slashes=c==='\\'?slashes+1:0;}return result;}
export function tableMarkdown(data:TableData,eol='\n'){validateTable(data);const row=(r:string[])=>'| '+r.map(escapePipes).join(' | ')+' |';return [row(data.rows[0]),row(data.align.map(a=>({none:'---',left:':---',center:':---:',right:'---:'})[a])),...data.rows.slice(1).map(row)].join(eol);}
export function insertContentBlock(raw:string,start:number,end:number,value:string){const eol=eolOf(raw),before=raw.slice(0,start),after=raw.slice(end),left=before&&!normalizedText(before).endsWith('\n\n')?before.endsWith('\n')||before.endsWith('\r')?eol:eol+eol:'',right=after&&!normalizedText(after).startsWith('\n\n')?after.startsWith('\n')||after.startsWith('\r')?eol:eol+eol:'';const at=start+left.length;return {text:before+left+value+right+after,start:at,end:at+value.length};}
export function codeAt(raw:string,caret:number){const nodes=fromMarkdown(raw,{extensions:[math()],mdastExtensions:[mathFromMarkdown()]}).children;const code=nodes.find(n=>n.type==='code'&&caret>=(n.position?.start.offset??0)&&caret<=(n.position?.end.offset??0));if(!code)return null;const from=code.position?.start.offset??0,to=code.position?.end.offset??0,match=/^([ \t]{0,3})(`{3,}|~{3,})([^\r\n]*)/.exec(raw.slice(from,to));return match?{from,to,infoFrom:from+match[1].length+match[2].length,infoTo:from+match[0].length,info:match[3]}:null;}
export function setCodeLanguage(raw:string,caret:number,language:string){if(!/^(?:[a-z0-9_+-]{1,32})?$/.test(language))throw new Error('语言标识无效。');const code=codeAt(raw,caret);if(!code)throw new Error('请先把光标放入围栏代码块。');const current=/^\s*[^\s]*/.exec(code.info)?.[0]??'',suffix=code.info.slice(current.length);if(current.trim()&&!/^[A-Za-z0-9_+-]{1,64}$/.test(current.trim()))throw new Error('围栏使用未知扩展info，请在源码修改；原文保留。');return {text:raw.slice(0,code.infoFrom)+language+suffix+raw.slice(code.infoTo),start:code.infoFrom+language.length,end:code.infoFrom+language.length};}
export function imageAt(raw:string,caret:number){const projection=projectMarkdown(raw);if(projection.reason)return null;const group=projection.blocks.find(b=>b.image&&caret>=b.from&&caret<=b.to);if(!group)return null;const node=fromMarkdown(raw.slice(group.from,group.to)).children[0];if(node.type!=='paragraph')return null;const image=node.children.find(n=>n.type==='image');if(!image)return null;return {group,from:group.from+(image.position?.start.offset??0),to:group.from+(image.position?.end.offset??0),alt:image.alt??'',title:image.title??'',url:image.url};}
export function setImageProperties(raw:string,caret:number,alt:string,title:string){if(alt.length>200||title.length>200||/[\r\n\0]/.test(alt+title))throw new Error('图片标题与替代文本各限200字且不能换行。');const image=imageAt(raw,caret);if(!image||!image.url.startsWith('assets/')||!assetNameValid(image.url.slice(7)))throw new Error('此图片使用未支持的引用形式；请在源码中编辑，原引用保留。');const value=`![${escapeInline(alt)}](${image.url}${title?' "'+title.replaceAll('\\','\\\\').replaceAll('"','\\"')+'"':''})`;return {text:raw.slice(0,image.from)+value+raw.slice(image.to),start:image.from,end:image.from+value.length};}
export function legacyImageTitle(caption:string){const match=/^\*\*图题：([^\r\n]*)\*\*(?:\r\n|\n){2}/.exec(caption);return {title:match?.[1].replace(/\\([\\`*_[\]{}()#+.!<>~|&])/g,'$1')??'',body:match?caption.slice(match[0].length):caption};}
export function withLegacyImageTitle(caption:string,title:string){if(title.length>200||/[\r\n\0]/.test(title))throw new Error('图片标题限200字且不能换行。');return (title?`**图题：${escapeInline(title)}**${eolOf(caption)}${eolOf(caption)}`:'')+legacyImageTitle(caption).body;}
export function contentShortcut(event:Pick<KeyboardEvent,'key'|'ctrlKey'|'metaKey'|'shiftKey'|'altKey'|'isComposing'|'repeat'|'keyCode'>,reserved:string[]=[]){if(!(event.ctrlKey||event.metaKey)||!event.shiftKey||event.altKey||event.isComposing||event.repeat||event.keyCode===229)return null;const chord=(event.ctrlKey?'ctrl+':'')+(event.metaKey?'meta+':'')+'shift+'+event.key.toLowerCase();if(reserved.some(s=>s.toLowerCase().replaceAll('commandorcontrol','ctrl').replaceAll('control','ctrl').replaceAll('command','meta').replaceAll('super','meta').replaceAll('cmd','meta').replaceAll('option','alt').split('+').sort().join('+')===chord.split('+').sort().join('+')))return null;return event.key.toLowerCase()==='t'?'table':event.key.toLowerCase()==='i'?'image':null;}
