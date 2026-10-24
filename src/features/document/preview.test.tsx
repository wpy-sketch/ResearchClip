import {describe,it,expect} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {MarkdownPreview} from './MarkdownPreview';
import {documentOutline} from './outline';
describe('safe CommonMark preview (renderer, not native screen capture)',()=>{
  it('renders requested formatting and bilingual text without executing HTML or remote URLs',()=>{
    const html=renderToStaticMarkup(<MarkdownPreview assets={{}} markdown={'# 中文 English\n\n**bold** *italic*\n\n1. first\n2. second\n\n- one\n\n```js\ncode()\n```\n`inline`\n\n<script>alert(1)</script>\n\n![secret](https://evil.invalid/collect)\n\n[link](javascript:alert)\n'}/>);
    for(const tag of ['<strong>','<em>','<ol>','<ul>','<pre>','<code'])expect(html).toContain(tag);
    expect(html).toMatch(/<h1 data-heading-offset="0">中文 English<\/h1>/);
    expect(html).toContain('中文 English');expect(html).not.toMatch(/<script|<iframe|href=|src=|evil\.invalid/);
  });
  it('maps only controlled relative resources to a local blob',()=>{const name=`researchclip-${'a'.repeat(32)}.png`;const html=renderToStaticMarkup(<MarkdownPreview assets={{[name]:{blob:new Blob(),url:'blob:local-preview'}}} markdown={`![图](assets/${name})\n\n注释`}/>);expect(html).toContain('src="blob:local-preview"');expect(html.indexOf('<img')).toBeLessThan(html.indexOf('注释'));});
  it('uses the same source offsets for outline navigation and all rendered headings',()=>{const markdown='# A\n\n## B\n\n### C\n\n#### D\n\n##### E\n\n###### 六\n\nSetext\n------\n\n> ## quote\n\n- ### list';const html=renderToStaticMarkup(<MarkdownPreview markdown={markdown} assets={{}}/>);for(const h of documentOutline(markdown))expect(html).toContain(`<h${h.depth} data-heading-offset="${h.offset}">${h.text}</h${h.depth}>`);});
});

 it('renders GFM tables, disabled task checkboxes, strike and highlight without interpreting code as formatting',()=>{const html=renderToStaticMarkup(<MarkdownPreview assets={{}} markdown={'~~删除~~ ==高亮==\n\n| 一 | 二 |\n| --- | --- |\n| 中 | 文 |\n\n- [x] 完成\n- [ ] 未完成\n\n`==代码==`\n\n```txt\n==源码==\n```'}/>);for(const tag of ['<del>删除</del>','<mark>高亮</mark>','<table>','type="checkbox"','disabled=""','<code>==代码==</code>'])expect(html).toContain(tag);expect(html).not.toContain('<mark>源码</mark>');});
