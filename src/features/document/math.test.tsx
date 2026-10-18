import {describe,it,expect} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {MarkdownPreview} from './MarkdownPreview';
import {parseMarkdown,serializeDocument} from './model';
import {documentOutline} from './outline';
const render=(markdown:string)=>renderToStaticMarkup(<MarkdownPreview markdown={markdown} assets={{}}/>);
describe('offline math, one Markdown preview pipeline',()=>{
  const examples=[String.raw`V(x)=x^T P x`,String.raw`\dot{x}=Ax+Bu`,String.raw`\frac{dV}{dt}<0`,String.raw`\|x-p\|=\lambda\|x-e\|`,String.raw`\begin{bmatrix}1&2\\3&4\end{bmatrix}`,String.raw`\sum_{i=1}^{n}i+\int_0^1 x\,dx`,String.raw`\begin{aligned}\dot{x}&=Ax+Bu\\V(x)&=x^TPx\end{aligned}`];
  it.each(examples)('renders requested math without changing source: %s',(formula)=>{const md=`中文 $${formula}$ English\n\n$$\n${formula}\n$$\n`,html=render(md);expect(html).toContain('class="katex"');expect(html).toContain('<math');expect(html).not.toContain('role="alert"');expect(serializeDocument(parseMarkdown(md,'数学.md'))).toBe(md);});
  it('updates output on edits and keeps formula source exact across reload',()=>{const a='# 数学\r\n\r\n$V=x^TPx$\r\n',b=a.replace('Px','Px+1');expect(render(a)).not.toBe(render(b));expect(serializeDocument(parseMarkdown(b))).toBe(b);});
  it('reports the exact malformed matrix supplied in the request without silently correcting it',()=>{const formula=String.raw`\begin{bmatrix}1&2\\\3&4\end{bmatrix}`,md=`$$\n${formula}\n$$`;expect(render(md)).toContain('role="alert"');expect(render(md)).toContain('公式错误');expect(serializeDocument(parseMarkdown(md))).toBe(md);});
  it.each([String.raw`\href{javascript:alert(1)}{click}`,String.raw`\includegraphics{https://evil.invalid/x}`,String.raw`\htmlClass{evil}{x}`,String.raw`\def\a{\a}\a`,String.raw`\unknowncommand{x}`])('rejects unsafe or unbounded math: %s',formula=>{const html=render(`$${formula}$`);expect(html).toContain('公式错误');expect(html).not.toMatch(/<script|href=|src="https:|class="evil"/);});
  it('isolates macros between expressions',()=>{const html=render(String.raw`$\gdef\secret{123}x$ $\secret$`);expect(html).toContain('公式错误');});
  it('leaves inline code, code fences, escaped dollars and raw HTML inert',()=>{const html=render('`$x$`\n\n```tex\n$$x$$\n```\n\n\\$money\n\n<span class="math-inline">\\href{https://evil.invalid}{x}</span>');expect(html).not.toContain('class="katex"');expect(html).not.toMatch(/href=|<script/);});
  it('bounds expression size and per-document math work',()=>{expect(render(`$${'a'.repeat(8193)}$`)).toContain('8192');expect(render(Array(257).fill('$x$').join(' '))).toContain('256');expect(render('$$\n'+'a'.repeat(8193)+'\n$$')).toContain('8192');});
  it('loads only explicit bundled guide images without broadening user-document image access',()=>{
    const md='![guide](images/v021-projects.png) ![prototype](constructor) ![outside](file:///C:/secret.png)';
    const html=renderToStaticMarkup(<MarkdownPreview markdown={md} assets={{}} guideImages={{'images/v021-projects.png':'/assets/guide.png'}}/>);
    expect(html).toContain('src="/assets/guide.png"');expect(html).not.toContain('src="file:');expect(html.match(/<img /g)).toHaveLength(1);
    expect(render(md)).not.toContain('<img ');
  });
  it('offers figure zoom only for explicitly resolved images, keeping arbitrary document resources inert',()=>{
    const html=renderToStaticMarkup(<MarkdownPreview markdown="![step](images/step.png) ![untrusted](https://evil.invalid/a.png)" assets={{}} guideImages={{'images/step.png':'/assets/step.png'}} onImageOpen={()=>{}}/>);
    expect(html.match(/<button /g)).toHaveLength(1);expect(html).toContain('放大图片：step');expect(html).not.toContain('src="https:');expect(render('![step](images/step.png)')).not.toContain('<button');
  });
  it('ignores pseudo-headings inside math blocks and aligns real heading offsets',()=>{const md='$$\n# not a title\n$$\n\n## 真标题 $x_1$\n';expect(documentOutline(md)).toHaveLength(1);expect(render(md)).toContain(`data-heading-offset="${md.indexOf('##')}"`);});
});
