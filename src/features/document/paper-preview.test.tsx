import {describe,it,expect} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {MarkdownPreview} from './MarkdownPreview';
import {analyzePaper} from './paper-model';
const eq=(label:string)=>`<!-- rc-label: ${label} -->\n\n$$\nx=1\n$$`;
const render=(raw:string)=>renderToStaticMarkup(<MarkdownPreview markdown={raw} assets={{}}/>);
describe('paper preview derives display without source rewriting',()=>{
 it('renders theorem, equation, figure, table and resolved references with actual offsets',()=>{const raw=[eq('eq:a'),'<!-- rc-label: thm:a -->\n\n:::theorem 唯一性\n\n有 $x=1$，见[公式](#rc-ref:eq:a)。\n\n:::','<!-- rc-label: fig:a -->\n\n![图](missing.png)','<!-- rc-label: tab:a -->\n\n| a |\n| --- |\n| 1 |','[表](#rc-ref:tab:a)'].join('\n\n'),html=render(raw);expect(html).toContain('定理 1');expect(html).toContain('公式编号 1');expect(html).toContain('paper-reference');expect(html).toContain('data-paper-target="thm:a"');expect(html).toContain('data-paper-target="fig:a"');expect(html).toContain('data-paper-target="tab:a"');expect(html).toContain('katex-mathml');});
 it('uses full analysis for fragments and never resets equation numbering',()=>{const first=eq('eq:a'),last=eq('eq:b'),raw=first+'\n\n'+last,html=renderToStaticMarkup(<MarkdownPreview markdown={last} assets={{}} paper={analyzePaper(raw)} sourceOffset={first.length+2} showDiagnostics={false}/>);expect(html).toContain('公式编号 2');expect(html).not.toContain('公式编号 1');});
 it('leaves duplicate and missing references unlinked with source-readable diagnostics',()=>{const html=render(eq('eq:a')+'\n\n'+eq('eq:a')+'\n\n[重复](#rc-ref:eq:a) 和 [缺失](#rc-ref:no)');expect(html).toContain('eq:a · 重复');expect(html).toContain('no · 缺失');expect(html).not.toContain('data-paper-reference=');});
 it('shows malformed environments as exact source and never executes HTML',()=>{const raw=':::theorem\n<script>alert(1)</script>\n',html=render(raw);expect(html).toContain('paper-invalid-source');expect(html).toContain('&lt;script&gt;');expect(html).not.toContain('<script>');});
 it('preserves globally defined reference images in split theorem bodies',()=>{const raw=':::example\n\n![图][fig]\n\n:::\n\n[fig]: local-guide.png',html=renderToStaticMarkup(<MarkdownPreview markdown={raw} assets={{}} guideImages={{'local-guide.png':'data:image/png;base64,AA=='}}/>);expect(html).toContain('src="data:image/png;base64,AA=="');});
 it('avoids duplicate automatic labels on explicit TeX tags and preserves invalid LaTeX',()=>{const raw=eq('eq:a').replace('x=1',String.raw`x\tag{A}`)+'\n\n$$\n\\frac{1}\n$$',html=render(raw);expect(html).not.toContain('公式编号 A');expect(html).toContain('公式错误');expect(html).toContain(String.raw`\frac{1}`);});
});
