import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {setTimeout as delay} from 'node:timers/promises';
import {chromium} from 'playwright-core';
const root=dirname(dirname(fileURLToPath(import.meta.url))),id=process.argv[2]??Date.now().toString();assert.match(id,/^[a-z0-9-]{1,64}$/);
const output=join(root,'artifacts',`math-${id}`);await mkdir(output);const settings=join(output,'settings'),exportsDir=join(output,'exports'),workspace=join(settings,'workspace');for(const p of [settings,exportsDir,workspace])await mkdir(p);
await writeFile(join(settings,'settings.json'),JSON.stringify({schemaVersion:1,theme:'dark',viewMode:'grid',fontFamily:'system',fontSize:16,captureShortcut:'Ctrl+Alt+Shift+F9',fullscreenShortcut:'Ctrl+Alt+Shift+F10',saveDirectory:exportsDir}));
// Seed a real v2 SQLite workspace before startup. Only these test-owned files may be changed.
execFileSync('python',['-c',String.raw`
from pathlib import Path
import sqlite3,json,zlib,struct,sys
root=Path(sys.argv[1]);workspace=Path(sys.argv[2]);(workspace/'originals').mkdir();(workspace/'thumbnails').mkdir()
def chunk(t,b):return struct.pack('>I',len(b))+t+b+struct.pack('>I',zlib.crc32(t+b))
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',2,2,8,6,0,0,0))+chunk(b'IDAT',zlib.compress((b'\x00'+bytes([30,60,90,128])*2)*2))+chunk(b'IEND',b'')
c=sqlite3.connect(workspace/'researchclip.sqlite3');c.executescript((root/'src-tauri/migrations/001_workspace.sql').read_text());c.executescript((root/'src-tauri/migrations/002_library.sql').read_text());c.execute('PRAGMA user_version=2')
for i in range(1,33):
 sid=f'{i:032x}';title=('科研' if i<=28 else '其他')+f' {i:02}';a=f'originals/{sid}.png';b=f'thumbnails/{sid}.png';(workspace/a).write_bytes(png);(workspace/b).write_bytes(png)
 doc=json.dumps({'schemaVersion':1,'image':{'widthPx':2,'heightPx':2},'objects':[]});c.execute('INSERT INTO screenshots VALUES(?,?,?,?,?,?,?,?,?,?,?)',(sid,title,a,b,doc,2,2,1000+i,1000+i,0,1))
c.execute('INSERT INTO notes VALUES(?,?,?,?,?,?,?)',('f'*32,f'{1:032x}','原始笔记 $\\alpha$ 中文','https://example.com',1000,1000,1));c.commit();c.close()
`,root,workspace],{windowsHide:true,encoding:'utf8'});
const helper=join(root,'src-tauri/target/debug/examples/workflow_input.exe');
const report={mode:'actual Windows WebView2, real v2 SQLite migration, native file dialogs and restart; test-owned PNGs, not screen capture',checks:[],errors:[]};let app,browser,page;
async function until(fn,name,timeout=20000){let last;const start=Date.now();while(Date.now()-start<timeout){try{const r=await fn();if(r)return r;}catch(e){last=e;}await delay(120);}throw new Error(`Timed out ${name}: ${last?.message??''}`);}
const input=(...args)=>execFileSync(helper,args.map(String),{cwd:root,windowsHide:true,encoding:'utf8',stdio:['ignore','pipe','pipe']});
const call=(name,args)=>page.evaluate(({name,args})=>window.__TAURI_INTERNALS__.invoke(name,args),{name,args});
async function launch(){assert.equal(await fetch('http://127.0.0.1:9350/json/version').then(()=>true).catch(()=>false),false);app=spawn(join(root,'src-tauri/target/debug/researchclip.exe'),[],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe'],env:{...process.env,RESEARCHCLIP_TEST_SETTINGS_DIR:settings,RESEARCHCLIP_TEST_CAPTURE_FIXTURE:'1',WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9350 --remote-debugging-address=127.0.0.1'}});app.stderr.on('data',d=>{for(const line of d.toString().trim().split('\n'))if(line)report.errors.push(line);});await until(()=>fetch('http://127.0.0.1:9350/json/version').then(r=>r.ok).catch(()=>false),'native startup');browser=await chromium.connectOverCDP('http://127.0.0.1:9350');page=await until(()=>browser.contexts().flatMap(c=>c.pages()).find(p=>!p.url().includes('captureOverlay')),'main');page.on('pageerror',e=>report.errors.push(e.message));await page.getByRole('button',{name:'新建项目',exact:true}).waitFor();await until(async()=>(await call('library_status')).schemaVersion===9,'schema9');}
async function stop(){input('close',app.pid);await until(()=>app.exitCode!==null,'exit');assert.equal(app.exitCode,0);await browser.close();browser=undefined;await until(()=>fetch('http://127.0.0.1:9350/json/version').then(()=>false).catch(()=>true),'WebView cleanup');}
const waitCards=async n=>until(async()=>(await page.locator('.library-card').count())===n,'library page');
async function makeProject(name){await page.getByRole('button',{name:'新建项目',exact:true}).click();await page.getByRole('textbox',{name:'项目名称',exact:true}).fill(name);await page.getByRole('button',{name:'保存项目',exact:true}).click();await page.getByRole('dialog',{name:'新建科研项目'}).waitFor({state:'detached'});return (await call('list_projects')).find(p=>p.name===name);}
try{
 await launch();await page.getByRole('button',{name:'取消',exact:true}).click();await waitCards(24);assert.equal(await page.locator('[data-markdown-panel]').count(),0);
 const a=await makeProject('控制理论'),b=await makeProject('机器学习');
 const rows=await call('select_library_screenshots',{favorites:false,sort:'createdDesc',filter:{query:'',tagIds:[],favorite:null}});
 await call('batch_library_screenshots',{items:rows.slice(0,2),action:{kind:'move',projectId:a.id}});await call('batch_library_screenshots',{items:rows.slice(2,3),action:{kind:'move',projectId:b.id}});
 await page.getByRole('button',{name:'刷新素材库',exact:true}).click();
 await page.getByRole('button',{name:'项目：控制理论',exact:true}).click();await until(async()=>await page.locator('.library-count').textContent()==='共 2 张素材','sidebar project A');assert.equal(await page.locator('.library-card').count(),2);
 await page.getByRole('button',{name:'项目：机器学习',exact:true}).click();await until(async()=>await page.locator('.library-count').textContent()==='共 1 张素材','sidebar project B');
 await page.getByRole('button',{name:'浏览未分类',exact:true}).click();await until(async()=>await page.locator('.library-count').textContent()==='共 29 张素材','sidebar unclassified');
 await page.getByRole('button',{name:'浏览全部素材',exact:true}).click();await waitCards(24);await until(async()=>await page.locator('.library-count').textContent()==='共 32 张素材','sidebar all');
 report.checks.push('real SQLite sidebar switches two projects, unclassified and all with correct counts; initially no Markdown pane');
 await page.getByRole('button',{name:'项目：控制理论',exact:true}).click();await waitCards(2);await mkdir(join(root,'docs/images'),{recursive:true});await page.setViewportSize({width:1180,height:1100});await page.locator('.workspace-scroll').first().evaluate(el=>{el.scrollTop=0;});if(process.argv.includes('--update-guide-images'))await page.screenshot({path:join(root,'docs/images/v021-projects.png'),clip:{x:0,y:0,width:1180,height:1020}});await page.setViewportSize({width:1180,height:800});
 await page.getByRole('button',{name:'新建空白 Markdown',exact:true}).click();let source=page.getByRole('textbox',{name:'Markdown 源文',exact:true});await source.waitFor();
 const text=String.raw`# 控制系统研究笔记

中文段落与行内公式 $V(x)=x^T P x$ 混排。

$$
\dot{x}=Ax+Bu,\qquad \frac{dV}{dt}<0
$$

$$
\|x-p\|=\lambda\|x-e\|
$$

$$
\begin{bmatrix}1&2\\3&4\end{bmatrix}
$$

$$
\begin{aligned}V(x)&=x^TPx\\\dot{V}(x)&<0\end{aligned}
$$

$$
\sum_{i=1}^{n}i+\int_0^1 x\,dx
$$
`;
 await source.fill(text);const live=page.locator('.markdown-live-preview');await until(async()=>await live.locator('.katex').count()===6,'live six equations');assert.equal(await live.getByRole('alert').count(),0);
 await page.evaluate(()=>document.fonts.ready);assert.equal(await page.evaluate(()=>[...document.fonts].some(f=>f.family.includes('KaTeX')&&f.status==='loaded')),true);
 const before=await live.innerHTML();await source.fill(text.replace('x^T P x','x^T P x+1'));await until(async()=>await live.innerHTML()!==before,'preview updates');await source.fill(text);
 await page.getByRole('button',{name:'预览 Markdown',exact:true}).click();assert.equal(await source.count(),0);assert.equal(await page.locator('[data-markdown-panel] .katex').count(),6);await page.getByRole('button',{name:'继续编辑',exact:true}).click();assert.equal(await source.inputValue(),text);
 report.checks.push('actual WebView offline KaTeX fonts load; inline/display formulas, matrix, Greek, aligned lines, sum/integral update live and survive source/preview switches');
 const bad=String.raw`\begin{bmatrix}1&2\\\3&4\end{bmatrix}`;await source.fill(text+'\n$$\n'+bad+'\n$$');await live.getByRole('alert').filter({hasText:'公式错误'}).waitFor();assert.ok((await source.inputValue()).includes(bad));
 await source.fill(text+'\n$\\href{javascript:alert(1)}{click}$\n\n<script>throw Error("unsafe")</script>\n\n![remote](https://evil.invalid/a.png)');await live.getByRole('alert').waitFor();assert.equal(await live.locator('script,iframe,a[href],img[src^="https:"]').count(),0);await source.fill(text);await until(async()=>await live.getByRole('alert').count()===0,'clear errors');
 report.checks.push('exact malformed matrix shows error without rewriting; hostile TeX URL, raw HTML and remote image remain inert');
 await page.getByRole('button',{name:'收藏',exact:true}).click();assert.equal(await source.inputValue(),text);await page.getByRole('button',{name:'截图素材',exact:true}).click();assert.equal(await source.inputValue(),text);
 await page.getByRole('button',{name:'关闭实时预览',exact:true}).click();assert.equal(await live.count(),0);await page.getByRole('button',{name:'开启实时预览',exact:true}).click();await until(async()=>await live.locator('.katex').count()===6,'live restored');
 await page.setViewportSize({width:480,height:400});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);assert.ok((await source.boundingBox()).height>=80);await page.getByRole('combobox',{name:'侧栏项目分类',exact:true}).selectOption(b.id);assert.equal(await source.inputValue(),text);await page.setViewportSize({width:1180,height:800});
 report.checks.push('left navigation/compact project selector preserve document; live toggle and 480x400 layout retain an 80px source editor');
 const file=join(exportsDir,'控制理论-公式.md');await source.press('Control+s');await until(()=>{input('dialog-save-md',app.pid,file);return true;},'save native dialog');await until(async()=>await readFile(file,'utf8')===text,'exact saved math source');
 await page.getByRole('combobox',{name:'当前 Markdown 项目',exact:true}).selectOption(a.id);await until(async()=>(await call('list_projects')).find(p=>p.id===a.id).documentCount===1,'indexed math document');
 await page.getByRole('button',{name:'显示大纲',exact:true}).click();await page.getByRole('navigation',{name:'文档大纲'}).getByRole('button').first().click();assert.equal(await source.inputValue(),text);await page.getByRole('button',{name:'隐藏大纲',exact:true}).click();
 await page.getByRole('button',{name:'项目：控制理论',exact:true}).click();await waitCards(2);await page.getByRole('button',{name:'预览 Markdown',exact:true}).click();await page.setViewportSize({width:1180,height:1000});if(process.argv.includes('--update-guide-images'))await page.screenshot({path:join(root,'docs/images/v021-math.png'),clip:{x:0,y:0,width:1180,height:910}});await page.setViewportSize({width:1180,height:800});await page.getByRole('button',{name:'继续编辑',exact:true}).click();
 report.checks.push('native Save preserves exact UTF-8 LaTeX in .md; association and outline keep source unchanged');
 await assert.rejects(call('open_markdown_external',{token:'../unauthorized.md'}));
 await writeFile(file,text+'\nexternal change');await page.getByRole('button',{name:'使用其他应用打开',exact:true}).click();await page.locator('[data-markdown-panel]').getByRole('alert').filter({hasText:'文件已被其他程序修改'}).waitFor();assert.equal(await readFile(file,'utf8'),text+'\nexternal change');await writeFile(file,text);
 report.checks.push('external-open rejects arbitrary paths/invalid grants and modified documents before launching; file bytes remain untouched');
 await page.getByRole('button',{name:'关闭文档',exact:true}).click();await stop();await launch();assert.equal(await page.locator('[data-markdown-panel]').count(),0);await page.getByRole('button',{name:'打开 Markdown',exact:true}).click();await until(()=>{input('dialog-open-md',app.pid,file);return true;},'open native dialog');source=page.getByRole('textbox',{name:'Markdown 源文',exact:true});await source.waitFor();assert.equal(await source.inputValue(),text);await until(async()=>await page.locator('[data-markdown-panel] .katex').count()===6,'equations after restart');
 const crlf=join(exportsDir,'CRLF 数学.md'),raw=text.replaceAll('\n','\r\n');await writeFile(crlf,raw);await page.getByRole('button',{name:'打开 Markdown',exact:true}).click();await until(()=>{input('dialog-open-md',app.pid,crlf);return true;},'open CRLF');await until(async()=>await page.getByRole('button',{name:'文件名：CRLF 数学.md',exact:true}).count()===1,'CRLF loaded');await source.press('Control+s');await until(async()=>(await page.locator('.markdown-notice').textContent()).includes('Markdown 与 assets 已保存'),'save CRLF');assert.equal(await readFile(crlf,'utf8'),raw);
 report.checks.push('process restart reopens saved equations; existing CRLF Markdown is opened/saved with byte-exact LaTeX and newlines');
 await page.getByRole('button',{name:'关闭文档',exact:true}).click();assert.equal(await page.locator('[data-markdown-panel]').count(),0);
 await page.getByRole('button',{name:'使用指南',exact:true}).click();const guide=page.getByRole('region',{name:'完整使用指南',exact:true});await guide.getByRole('navigation',{name:'使用指南目录'}).waitFor();assert.equal(await guide.getByRole('navigation',{name:'使用指南目录'}).getByRole('button').count(),14);for(const alt of ['左侧项目分类与对应图片（隔离测试资料）','真实 Windows Markdown 数学预览（隔离测试资料）']){const img=guide.getByRole('img',{name:alt,exact:true});await img.scrollIntoViewIfNeeded();await until(()=>img.evaluate(el=>el.complete&&el.naturalWidth>0),'bundled guide image');}assert.equal(await guide.getByRole('alert').count(),0);assert.ok((await guide.locator('pre code').allTextContents()).some(t=>t.includes('\\dot{x}=Ax+Bu')));report.checks.push('bundled task guide retains 14 searchable chapters, loads both controlled screenshots and preserves copyable LaTeX examples as code');
 await stop();assert.equal(report.errors.length,0);report.result='PASS';
}catch(e){report.result='FAIL';report.failure=e.stack;process.exitCode=1;if(page&&!page.isClosed())await page.screenshot({path:join(output,'failure.png')}).catch(()=>{});}
finally{if(browser)await browser.close().catch(()=>{});if(app&&app.exitCode===null)app.kill();await writeFile(join(output,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));}
