import {describe,it,expect} from 'vitest';
import {activeFormats,findText,formatSelection,markdownShortcut,replaceAllText,type FormatAction} from './formatting';
describe('selection formatting (UTF-16 textarea offsets)',()=>{
 it('bolds only the selected Chinese text and retains its selection',()=>{expect(formatSelection('前缀中文后缀',{start:2,end:4},'bold')).toEqual({text:'前缀**中文**后缀',start:4,end:6});});
 it('toggles adjacent delimiters without changing surrounding text',()=>{expect(formatSelection('前**中文**后',{start:3,end:5},'bold')).toEqual({text:'前中文后',start:1,end:3});});
 it('inserts an editable placeholder at an empty caret',()=>{expect(formatSelection('左右',{start:1,end:1},'italic')).toEqual({text:'左*文字*右',start:2,end:4});});
 it('does not split astral Chinese/emoji text outside the selected range',()=>{expect(formatSelection('😀中文尾',{start:2,end:4},'highlight').text).toBe('😀==中文==尾');});
 for(const action of ['bold','italic','strike','highlight','code','math'] as FormatAction[])it(`formats nonempty lines independently for ${action}`,()=>{const changed=formatSelection('首\n中文\n\n末',{start:2,end:5},action);expect(changed.text.startsWith('首\n')).toBe(true);expect(changed.text.endsWith('\n末')).toBe(true);expect(changed.text).not.toBe('首\n中文\n\n末');});
 for(let depth=1;depth<=6;depth++)it(`applies H${depth} to all selected lines only`,()=>{expect(formatSelection('前\n甲\n乙\n尾',{start:2,end:6},`h${depth}` as FormatAction).text).toBe(`前\n${'#'.repeat(depth)} 甲\n${'#'.repeat(depth)} 乙\n尾`);});
 it('changes the existing heading level instead of stacking prefixes',()=>{expect(formatSelection('## 标题',{start:4,end:4},'h3').text).toBe('### 标题');});
 it('toggles a selected heading back to plain text',()=>{expect(formatSelection('# 标题',{start:3,end:3},'h1').text).toBe('标题');});
 it('preserves CRLF and excludes the following line when selection ends at its start',()=>{expect(formatSelection('前\r\n甲\r\n乙',{start:3,end:6},'bullet').text).toBe('前\r\n- 甲\r\n乙');});
 it('supports caret at offset zero on an empty first line',()=>{expect(formatSelection('\n后',{start:0,end:0},'h1').text).toBe('# \n后');});
 it('numbers a multiline selection and toggles task lists',()=>{expect(formatSelection('甲\n乙',{start:0,end:3},'ordered').text).toBe('1. 甲\n2. 乙');expect(formatSelection('- [ ] 甲\n- [x] 乙',{start:0,end:15},'task').text).toBe('甲\n乙');});
 it('adds quote prefixes and preserves indentation',()=>{expect(formatSelection('  甲\n  乙',{start:0,end:7},'quote').text).toBe('  > 甲\n  > 乙');});
 it('creates a code fence longer than any selected fence',()=>{expect(formatSelection('a```b',{start:0,end:5},'codeBlock').text).toBe('````\na```b\n````');});
 it('creates inline/block math and tables as ordinary Markdown',()=>{expect(formatSelection('x',{start:0,end:1},'math').text).toBe('$x$');expect(formatSelection('x',{start:0,end:1},'mathBlock').text).toBe('$$\nx\n$$');expect(formatSelection('中|文',{start:0,end:3},'table').text).toContain('| 中\\|文 | 内容 |');});
 it('separates inserted blocks from the middle of surrounding text',()=>{expect(formatSelection('左右',{start:1,end:1},'rule').text).toBe('左\n\n---\n\n\n右');});
 it('validates link protocols and escapes destination parentheses',()=>{expect(formatSelection('中文',{start:0,end:2},'link','https://example.org/a(b) c').text).toBe('[中文](https://example.org/a%28b%29%20c)');for(const url of ['javascript:alert(1)','data:text/html,x','//evil.example','https://a\nsecret'])expect(()=>formatSelection('',{start:0,end:0},'link',url)).toThrow();});
 it('reports formatting at a caret and across a selected span',()=>{expect(activeFormats('## **中文**',{start:6,end:6})).toEqual(new Set(['h2','bold']));expect(activeFormats('- [ ] 一\n- [x] 二',{start:0,end:15}).has('task')).toBe(true);expect(activeFormats('*斜体*',{start:1,end:3}).has('italic')).toBe(true);});
});
describe('scoped shortcuts and literal search',()=>{
 const key=(key:string,extra={})=>markdownShortcut({key,ctrlKey:true,metaKey:false,altKey:false,shiftKey:false,...extra});
 it('maps editor shortcuts and undo/redo explicitly',()=>{for(const [k,value] of [['B','bold'],['i','italic'],['k','link'],['f','find'],['h','replace'],['z','undo'],['y','redo']])expect(key(k)).toBe(value);expect(key('z',{shiftKey:true})).toBe('redo');expect(key('b',{ctrlKey:false,metaKey:true})).toBe('bold');});
 it('leaves capture and global palette combinations unhandled',()=>{for(const k of ['s','p','a','F7','F8'])expect(key(k,{shiftKey:true})).toBeNull();expect(key('b',{altKey:true})).toBeNull();expect(key('k',{ctrlKey:false})).toBeNull();});
 it('never formats during Chinese IME composition or legacy keyCode229',()=>{expect(key('b',{isComposing:true})).toBeNull();expect(key('k',{keyCode:229})).toBeNull();});
 it('finds literal punctuation, wraps and handles empty queries',()=>{expect(findText('A.[中] a.[中]','.[中]',7)).toEqual({start:7,end:11});expect(findText('A A','a',3)).toEqual({start:0,end:1});expect(findText('A','a',0,true)).toBeNull();expect(findText('A','',0)).toBeNull();});
 it('replaces literal dollar signs without regex expansion or loops',()=>{expect(replaceAllText('中文 中文','中文','$&')).toEqual({text:'$& $&',count:2});expect(replaceAllText('aa','a','aaa')).toEqual({text:'aaaaaa',count:2});expect(replaceAllText('keep','','x')).toEqual({text:'keep',count:0});});
});

it('case folding retains original UTF-16 offsets when Unicode lowercase expands',()=>{expect(findText('İ甲 İ乙','乙',0)).toEqual({start:4,end:5});expect(replaceAllText('İ甲 İ乙','乙','新')).toEqual({text:'İ甲 İ新',count:1});});
