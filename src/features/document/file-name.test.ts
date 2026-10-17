import {describe,it,expect} from 'vitest';
import {normalizeMarkdownName} from './file-name';
describe('portable Markdown filenames',()=>{
  it('retains Chinese spaces and existing extension, appending .md only when missing',()=>{expect(normalizeMarkdownName('  数学 研究  ')).toBe('数学 研究.md');expect(normalizeMarkdownName('notes.MD')).toBe('notes.MD');});
  it('rejects paths, controls, devices, trailing dots and overlong UTF-16 names',()=>{for(const name of ['', '../x','C:\\x','x/y','a:b','a?b','name.','a\u0000b','CON','con.txt','LPT1','COM¹','x'.repeat(253),'🙂'.repeat(127)])expect(()=>normalizeMarkdownName(name),name).toThrow();});
});
