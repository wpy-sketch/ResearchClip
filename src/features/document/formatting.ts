import {MAX_TEXT_BYTES} from './model';
export type FormatAction='h1'|'h2'|'h3'|'h4'|'h5'|'h6'|'bold'|'italic'|'strike'|'highlight'|'bullet'|'ordered'|'task'|'quote'|'rule'|'link'|'table'|'code'|'codeBlock'|'math'|'mathBlock';
export interface TextSelection {start:number;end:number}
export interface TextChange extends TextSelection {text:string}
const wrappers:Partial<Record<FormatAction,string>>={bold:'**',italic:'*',strike:'~~',highlight:'==',code:'`',math:'$'};
const clamp=(text:string,s:TextSelection):TextSelection=>({start:Math.max(0,Math.min(text.length,s.start)),end:Math.max(0,Math.min(text.length,Math.max(s.start,s.end)))});
const replace=(text:string,start:number,end:number,value:string,a=0,b=value.length):TextChange=>({text:text.slice(0,start)+value+text.slice(end),start:start+a,end:start+b});
function lines(text:string,selection:TextSelection){
 const s=clamp(text,selection),start=text.slice(0,s.start).lastIndexOf('\n')+1;
 const last=s.end>s.start&&text[s.end-1]==='\n'?s.end-1:s.end,endAt=text.indexOf('\n',last),end=endAt<0?text.length:endAt;
 return {start,end,body:text.slice(start,end),eol:text.includes('\r\n')?'\r\n':'\n'};
}
const prefix=(action:FormatAction,index:number)=>/^h[1-6]$/.test(action)?'#'.repeat(Number(action[1]))+' ':action==='bullet'?'- ':action==='ordered'?`${index+1}. `:action==='task'?'- [ ] ':action==='quote'?'> ':'';
const lineMarker=/^(?:#{1,6}\s+|[-+*]\s+(?:\[[ xX]\]\s+)?|\d+[.)]\s+|>\s?)/;
function hasLineFormat(line:string,action:FormatAction){
 const trimmed=line.trimStart();
 if(/^h[1-6]$/.test(action))return new RegExp(`^#{${action[1]}}\\s`).test(trimmed);
 return action==='task'?/^[-+*]\s+\[[ xX]\]\s/.test(trimmed):action==='bullet'?/^[-+*]\s+(?!\[[ xX]\]\s)/.test(trimmed):action==='ordered'?/^\d+[.)]\s/.test(trimmed):action==='quote'?/^>\s?/.test(trimmed):false;
}
function block(text:string,s:TextSelection,value:string,innerStart=0,innerEnd=value.length){
 const eol=text.includes('\r\n')?'\r\n':'\n',before=s.start&&text[s.start-1]!=='\n'?eol+eol:'',after=s.end<text.length&&text[s.end]!=='\n'?eol+eol:'';
 return replace(text,s.start,s.end,before+value+after,before.length+innerStart,before.length+innerEnd);
}
export function formatSelection(text:string,selection:TextSelection,action:FormatAction,link='https://example.org'):TextChange{
 const s=clamp(text,selection),selected=text.slice(s.start,s.end),eol=text.includes('\r\n')?'\r\n':'\n';
 let marker=wrappers[action];
 if(marker){
  if(action==='code'&&selected.includes('`'))marker='`'.repeat(Math.max(...[...selected.matchAll(/`+/g)].map(m=>m[0].length))+1);
  if(selected.startsWith(marker)&&selected.endsWith(marker)&&selected.length>=marker.length*2)return replace(text,s.start,s.end,selected.slice(marker.length,-marker.length));
  if(text.slice(0,s.start).endsWith(marker)&&text.slice(s.end).startsWith(marker))return replace(text,s.start-marker.length,s.end+marker.length,selected);
  if(/\r?\n/.test(selected)){
   const parts=selected.split(/(\r?\n)/),remove=parts.filter((_,i)=>i%2===0).filter(Boolean).every(v=>v.startsWith(marker!)&&v.endsWith(marker!));
   const value=parts.map((v,i)=>i%2||!v?v:remove?v.slice(marker!.length,-marker!.length):marker+v+marker).join('');
   return replace(text,s.start,s.end,value);
  }
  const value=selected||({code:'代码',math:'x',bold:'文字',italic:'文字',strike:'文字',highlight:'文字'}[action as 'code']??'文字');
  return replace(text,s.start,s.end,marker+value+marker,marker.length,marker.length+value.length);
 }
 if(prefix(action,0)){
  const span=lines(text,s),rows=span.body.split(/\r?\n/),remove=rows.some(l=>l.trim())&&rows.filter(l=>l.trim()).every(l=>hasLineFormat(l,action));
  const value=rows.map((line,index)=>{const indent=line.match(/^[ \t]*/)?.[0]??'',body=line.slice(indent.length);return indent+(remove?body.replace(lineMarker,''):prefix(action,index)+body.replace(lineMarker,''));}).join(span.eol);
  return replace(text,span.start,span.end,value);
 }
 if(action==='link'){
  if(!link.trim()||Array.from(link).some(c=>c.charCodeAt(0)<32)||!(/^(https?:\/\/|mailto:|researchclip:\/\/)/i.test(link)||!/^([a-z][a-z0-9+.-]*:|\/\/)/i.test(link)))throw new Error('链接仅支持 http、https、mailto、内部引用或相对路径。');
  const label=(selected||'链接文字').replace(/([[\]])/g,'\\$1'),url=link.trim().replaceAll(' ','%20').replaceAll('(','%28').replaceAll(')','%29').replaceAll('<','%3C').replaceAll('>','%3E');
  return replace(text,s.start,s.end,`[${label}](${url})`,1,1+label.length);
 }
 if(action==='rule')return block(text,s,'---'+eol);
 if(action==='table'){
  const label=(selected||'内容').replaceAll('|','\\|').replace(/\r?\n/g,' '),value=`| 列一 | 列二 |${eol}| --- | --- |${eol}| ${label} | 内容 |${eol}`;
  return block(text,s,value,value.indexOf(label),value.indexOf(label)+label.length);
 }
 const content=selected||(action==='mathBlock'?'x = y':'代码'),fence=action==='mathBlock'?'$$':'`'.repeat(Math.max(3,...[...content.matchAll(/`+/g)].map(m=>m[0].length+1))),value=fence+eol+content+eol+fence;
 return block(text,s,value,fence.length+eol.length,fence.length+eol.length+content.length);
}
export function activeFormats(text:string,selection:TextSelection):Set<FormatAction>{
 const s=clamp(text,selection),active=new Set<FormatAction>(),span=lines(text,s);
 for(const action of ['h1','h2','h3','h4','h5','h6','bullet','ordered','task','quote'] as FormatAction[])if(span.body.split(/\r?\n/).every(l=>hasLineFormat(l,action)))active.add(action);
 // Delimiter state on the current line, including a collapsed caret inside a span.
 const lineStart=text.slice(0,s.start).lastIndexOf('\n')+1,lineEnd=text.indexOf('\n',s.end),line=text.slice(lineStart,lineEnd<0?text.length:lineEnd);
 const patterns:[FormatAction,RegExp][]=[['bold',/\*\*(?=\S)(.+?)\*\*/g],['italic',/(?<!\*)\*(?!\*)(?=\S)(.+?)\*(?!\*)/g],['strike',/~~(.+?)~~/g],['highlight',/==(.+?)==/g],['code',/`([^`]+)`/g],['math',/(?<!\$)\$(?!\$)(.+?)\$(?!\$)/g],['link',/\[([^\]]*)\]\([^)]*\)/g]];
 for(const [action,pattern] of patterns)for(const match of line.matchAll(pattern)){const start=lineStart+match.index!,end=start+match[0].length;if(s.start>=start&&s.end<=end)active.add(action);}
 return active;
}
export interface ShortcutInput {key:string;ctrlKey:boolean;metaKey:boolean;altKey:boolean;shiftKey:boolean;isComposing?:boolean;keyCode?:number}
export function markdownShortcut(event:ShortcutInput):FormatAction|'undo'|'redo'|'find'|'replace'|null{
 if(!(event.ctrlKey||event.metaKey)||event.altKey||event.isComposing||event.keyCode===229)return null;
 const key=event.key.toLowerCase();
 if(key==='z')return event.shiftKey?'redo':'undo';if(key==='y')return 'redo';
 if(event.shiftKey)return null;
 return ({b:'bold',i:'italic',k:'link',f:'find',h:'replace'} as const)[key as 'b']??null;
}
const fold=(text:string)=>Array.from(text,c=>{const lower=c.toLowerCase();return lower.length===c.length?lower:c;}).join('');
export function findText(text:string,query:string,from:number,matchCase=false):TextSelection|null{
 if(!query)return null;const source=matchCase?text:fold(text),needle=matchCase?query:fold(query),position=source.indexOf(needle,Math.max(0,from)),start=position<0?source.indexOf(needle):position;
 return start<0?null:{start,end:start+query.length};
}
export function replaceAllText(text:string,query:string,value:string,matchCase=false):{text:string;count:number}{
 if(!query)return {text,count:0};const source=matchCase?text:fold(text),needle=matchCase?query:fold(query);let matches=0;for(let at=source.indexOf(needle);at>=0;at=source.indexOf(needle,at+query.length))matches++;if(text.length+matches*(value.length-query.length)>MAX_TEXT_BYTES)throw new Error('替换后文档超过2 MiB，原文和撤销记录未改动。');let offset=0,count=0,result='';
 for(let at=source.indexOf(needle);at>=0;at=source.indexOf(needle,offset)){result+=text.slice(offset,at)+value;offset=at+query.length;count++;}return {text:result+text.slice(offset),count};
}

export {fold as foldText};
