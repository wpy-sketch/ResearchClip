import {describe,it,expect} from 'vitest';
import {builtInTemplates,templateDocument,templateMarkdown} from './templates';
import {documentOutline} from './outline';
import {serializeDocument,createDocument,parseMarkdown} from './model';
describe('editable standard Markdown templates',()=>{
  it('creates a genuinely empty in-memory document with distinct session and block IDs',()=>{const a=templateDocument(builtInTemplates[0]),b=templateDocument(builtInTemplates[0]);expect(serializeDocument(a)).toBe('');expect(a.id).not.toBe(b.id);expect(a.blocks[0].id).not.toBe(b.blocks[0].id);});
  it('contains every requested paper and theorem section and normal experiment headings',()=>{
    expect(documentOutline(builtInTemplates[1].markdown).slice(1).map(h=>h.text)).toEqual(['论文信息','研究问题','核心创新点','模型与假设','主要方法','关键公式','实验与结果','个人思考']);
    expect(documentOutline(builtInTemplates[2].markdown).slice(1).map(h=>h.text)).toEqual(['问题定义','假设','引理','定理','证明','反例','推论','待解决问题']);
    expect(documentOutline(builtInTemplates[3].markdown).some(h=>h.text==='实验步骤')).toBe(true);
  });
  it('keeps edited Unicode, formula and image references through standard save/reload without internal metadata',()=>{
    const text='# 研究 & English\n\n$\\alpha_1 < \\beta$\n\n![图](assets/researchclip-'+ 'a'.repeat(32)+'.png)\n\n对应注释';
    const doc=templateDocument({name:'自定义',markdown:text,assets:{}});expect(templateMarkdown(parseMarkdown(serializeDocument(doc),'saved.md'))).toBe(text);
    const structured=createDocument();structured.blocks=[{id:'b'.repeat(32),type:'text',markdown:'## 正文'}];expect(templateMarkdown(structured)).toBe('# 未命名文档\n\n## 正文');expect(templateMarkdown(structured)).not.toContain('ResearchClip:');
  });
});
describe('CommonMark outline source offsets',()=>{
  it('recognizes H1–H6, Setext, formatting, duplicates, links, Unicode and exact jump offsets',()=>{
    const text='# 同名 **粗体** &amp; [link](https://example.com)\n\n## 同名\n### C\n#### D\n##### E\n###### 六 `x`\n\nSetext 标题\n============\n';const rows=documentOutline(text);
    expect(rows.map(h=>h.depth)).toEqual([1,2,3,4,5,6,1]);expect(rows[0].text).toBe('同名 粗体 & link');expect(rows[5].text).toBe('六 x');expect(new Set(rows.map(h=>h.offset)).size).toBe(7);
    for(const row of rows)expect(text.slice(row.offset).startsWith(row.depth===1&&row.text==='Setext 标题'?'Setext':'#')).toBe(true);
  });
  it('ignores fenced and indented code, escaped markers, HTML and invalid depth, and updates offsets after edits',()=>{
    const text='```md\n# not a heading\n```\n\n~~~\n## no\n~~~\n\n    # code\n\n\\# escaped\n\n####### invalid\n\n<div>\n# hidden\n</div>\n\n## 实际';expect(documentOutline(text).map(h=>h.text)).toEqual(['实际']);
    const updated='prefix\n\n'+text.replace('## 实际','###### 已更新');const row=documentOutline(updated)[0];expect(row.depth).toBe(6);expect(row.text).toBe('已更新');expect(updated.slice(row.offset)).toBe('###### 已更新');
  });
  it('handles empty content, empty headings, CRLF and container headings',()=>{expect(documentOutline('')).toEqual([]);const rows=documentOutline('#\r\n\r\n> ## 引用\r\n\r\n- ### 列表');expect(rows.map(h=>h.text)).toEqual(['（空标题）','引用','列表']);expect(rows.map(h=>h.line)).toEqual([1,3,5]);});
});

it('provides literature review and research log as ordinary independently editable templates',()=>{for(const name of ['文献综述','研究日志']){const seed=builtInTemplates.find(t=>t.name===name)!;expect(seed).toBeDefined();const doc=templateDocument(seed);expect(serializeDocument(doc)).toBe(seed.markdown);expect(documentOutline(seed.markdown).length).toBeGreaterThan(5);expect(doc.mode).toBe('source');}expect(builtInTemplates[2].markdown).toContain(':::theorem');expect(builtInTemplates[2].markdown).toContain(':::proof');});
