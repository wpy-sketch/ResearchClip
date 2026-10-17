import {describe,it,expect} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {MarkdownPreview} from './MarkdownPreview';
describe('safe CommonMark preview (renderer, not native screen capture)',()=>{
  it('renders requested formatting and bilingual text without executing HTML or remote URLs',()=>{
    const html=renderToStaticMarkup(<MarkdownPreview assets={{}} markdown={'# 中文 English\n\n**bold** *italic*\n\n1. first\n2. second\n\n- one\n\n```js\ncode()\n```\n`inline`\n\n<script>alert(1)</script>\n\n![secret](https://evil.invalid/collect)\n\n[link](javascript:alert)\n'}/>);
    for(const tag of ['<h1>','<strong>','<em>','<ol>','<ul>','<pre>','<code'])expect(html).toContain(tag);
    expect(html).toContain('中文 English');expect(html).not.toMatch(/<script|<iframe|href=|src=|evil\.invalid/);
  });
  it('maps only controlled relative resources to a local blob',()=>{const name=`researchclip-${'a'.repeat(32)}.png`;const html=renderToStaticMarkup(<MarkdownPreview assets={{[name]:{blob:new Blob(),url:'blob:local-preview'}}} markdown={`![图](assets/${name})\n\n注释`}/>);expect(html).toContain('src="blob:local-preview"');expect(html.indexOf('<img')).toBeLessThan(html.indexOf('注释'));});
});
