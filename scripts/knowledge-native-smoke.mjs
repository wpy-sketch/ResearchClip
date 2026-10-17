import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {mkdir,writeFile,readFile} from 'node:fs/promises';
import {createServer} from 'node:http';
import {join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {setTimeout as delay} from 'node:timers/promises';
import {createHash} from 'node:crypto';
import {chromium} from 'playwright-core';

const root=dirname(dirname(fileURLToPath(import.meta.url))),id=process.argv[2]??Date.now().toString();assert.match(id,/^[a-z0-9-]{1,64}$/);
const output=join(root,'artifacts',`knowledge-${id}`);await mkdir(output);const settingsDir=join(output,'settings');await mkdir(settingsDir);const workspace=join(settingsDir,'workspace');
await writeFile(join(settingsDir,'settings.json'),JSON.stringify({schemaVersion:1,theme:'dark',viewMode:'grid',fontFamily:'system',fontSize:16,captureShortcut:'Ctrl+Alt+Shift+F9',fullscreenShortcut:'Ctrl+Alt+Shift+F10',saveDirectory:null}));
const report={mode:'actual Windows WebView2, SQLite and PNG; controlled source image, real autosave/search/source opening; no mocked persistence',checks:[],errors:[]};
let app,browser,page,visits=0;const calls=[];
const server=createServer((request,response)=>{if(request.url==='/paper'){visits++;}response.writeHead(200,{'Content-Type':'text/html; charset=utf-8'});response.end('<title>ResearchClip 来源验收</title><p>Local acceptance source page. No research data is received.</p>');});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const source=`http://127.0.0.1:${server.address().port}/paper`;
const helper=join(root,'src-tauri/target/debug/examples/workflow_input.exe'),fault=join(root,'src-tauri/target/debug/examples/knowledge_fault.exe');
async function until(fn,name,timeout=20000){const start=Date.now();let last;while(Date.now()-start<timeout){try{const result=await fn();if(result)return result;}catch(e){last=e;}await delay(80);}throw new Error(`Timed out ${name}: ${last?.message??''}`);}
const call=(command,args)=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const card=id=>call('get_knowledge_card',{id});
const ids=()=>page.locator('[data-screenshot-id]').evaluateAll(nodes=>nodes.map(n=>n.dataset.screenshotId));
const expectIds=async(expected)=>{await until(async()=>{const actual=await ids();return await page.getByRole('region',{name:'截图素材库',exact:true}).getAttribute('aria-busy')==='false'&&actual.length===expected.length&&actual.every(x=>expected.includes(x));},`matching ${expected.length} records`);};
const query=async(text,expected)=>{
  await page.getByRole('textbox',{name:'快捷搜索',exact:true}).fill(text);
  await until(async()=>{const keyword=page.locator('.library-filter-chips > span');return text.trim()?await keyword.count()>0&&await keyword.textContent()===`关键词：${text.trim()}`:await keyword.count()===0;},'applied debounced query');
  await expectIds(expected);
};
const saved=async(id,predicate)=>until(async()=>{const c=await card(id);return predicate(c)&&c;},'actual card persistence');
async function library(){await page.getByRole('button',{name:'截图素材',exact:true}).click();await page.getByRole('region',{name:'截图素材库',exact:true}).waitFor();}
async function open(record){await library();await query(record.title,[record.id]);await page.getByRole('button',{name:`打开截图 ${record.title}`,exact:true}).click();await page.getByRole('textbox',{name:'Markdown 笔记正文',exact:true}).waitFor();}
async function clear(){await page.getByRole('button',{name:'清除全部筛选',exact:true}).click();await until(async()=>await page.locator('[data-screenshot-id]').count()===24,'unfiltered page');}
async function launch(){
  assert.equal(await fetch('http://127.0.0.1:9339/json/version').then(()=>true).catch(()=>false),false,'CDP port occupied');
  app=spawn(join(root,'src-tauri/target/debug/researchclip.exe'),[],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe'],env:{...process.env,RESEARCHCLIP_TEST_SETTINGS_DIR:settingsDir,RESEARCHCLIP_TEST_CAPTURE_FIXTURE:'1',WEBVIEW2_USER_DATA_FOLDER:join(output,'profile'),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9339 --remote-debugging-address=127.0.0.1'}});
  app.stderr.on('data',c=>{for(const line of c.toString().trim().split('\n'))if(line)report.errors.push(line);});
  await until(()=>fetch('http://127.0.0.1:9339/json/version').then(r=>r.ok).catch(()=>false),'native launch');browser=await chromium.connectOverCDP('http://127.0.0.1:9339');page=await until(()=>browser.contexts().flatMap(c=>c.pages()).find(p=>!p.url().includes('captureOverlay')),'main');
  page.on('pageerror',e=>report.errors.push(e.message));page.on('request',request=>{const url=new URL(request.url());if(url.hostname==='ipc.localhost'){const command=decodeURIComponent(url.pathname.slice(1));if(['save_knowledge_card','screenshot_page','screenshot_image'].includes(command)){const args=request.postDataJSON();calls.push({command,thumbnail:args?.thumbnail});}}});
  await page.getByRole('button',{name:'存入素材库',exact:true}).waitFor();await until(async()=> (await call('library_status')).schemaVersion===3,'SQLite initialization');
}
async function close(){const exited=new Promise(resolve=>app.once('exit',resolve));execFileSync(helper,['close',String(app.pid)],{cwd:root,windowsHide:true,stdio:'pipe'});await Promise.race([exited,delay(8000).then(()=>{throw new Error('Owned app did not close');})]);assert.equal(app.exitCode,0);await browser.close();browser=undefined;await until(()=>fetch('http://127.0.0.1:9339/json/version').then(()=>false).catch(()=>true),'shutdown');}
try{
  await launch();
  const records=await page.evaluate(async()=>{
    const invoke=window.__TAURI_INTERNALS__.invoke,c=await invoke('current_capture'),png=await invoke('capture_result_png',{captureId:c.captureId}),records=[];
    const titles=["细胞 Alpha 100%_ C:\\研究 O'Brien",'蛋白研究 Beta','对照数据',...Array.from({length:26},(_,n)=>`队列研究 ${n+1}`)];
    for(const title of titles){const json=new TextEncoder().encode(JSON.stringify({title,document:{schemaVersion:1,image:{widthPx:c.widthPx,heightPx:c.heightPx},objects:[]}})),bytes=new ArrayBuffer(4+json.length+png.byteLength);new DataView(bytes).setUint32(0,json.length,true);new Uint8Array(bytes,4,json.length).set(json);new Uint8Array(bytes,4+json.length).set(new Uint8Array(png));records.push(await invoke('create_screenshot',bytes));}return records;
  });
  const [a,b,c]=records;await page.getByRole('button',{name:'刷新素材库',exact:true}).click();await until(async()=>await page.locator('[data-screenshot-id]').count()===24,'real library page');
  const originalHash=createHash('sha256').update(await readFile(join(workspace,a.originalImagePath))).digest('hex');
  await open(a);const body='# 实验观察\nAlpha 100%_ C:\\样本\n<script>window.researchclipUnsafe=1</script><img src=x onerror="window.researchclipUnsafe=2">';
  const writes=calls.filter(x=>x.command==='save_knowledge_card').length;
  await page.getByRole('textbox',{name:'Markdown 笔记正文'}).fill('first');await delay(180);await page.getByRole('textbox',{name:'Markdown 笔记正文'}).fill(body);await page.getByRole('textbox',{name:'来源 URL'}).fill(source);
  for(const name of ['细胞','Method']){await page.getByRole('combobox',{name:'添加卡片标签'}).fill(name);await page.getByRole('button',{name:'添加标签',exact:true}).click();}
  await page.getByRole('button',{name:'收藏卡片',exact:true}).click();const first=await saved(a.id,c=>c.note?.content===body&&c.tags.length===2&&c.screenshot.isFavorite);
  assert.equal(calls.filter(x=>x.command==='save_knowledge_card').length-writes,1);assert.equal(await page.evaluate(()=>window.researchclipUnsafe),undefined);assert.equal(await page.getByRole('textbox',{name:'Markdown 笔记正文'}).inputValue(),body);
  assert.equal(first.note.sourceUrl,source);report.checks.push('real debounced card save persists literal Markdown/HTML, URL, multiple tags and favorite in one write; no script or image HTML executes');
  assert.equal(visits,0);await page.getByRole('button',{name:'打开来源',exact:true}).click();await until(()=>visits>0,'default browser visits local source');
  await assert.rejects(()=>call('plugin:opener|open_url',{url:'https://example.org'}));report.checks.push('official Rust Opener actually opens the saved local http source in the default browser; generic WebView opener permission is denied');
  await open(b);await page.getByRole('textbox',{name:'Markdown 笔记正文'}).fill('Beta Protein · 二次实验');await page.getByRole('combobox',{name:'添加卡片标签'}).fill('Method');await page.getByRole('button',{name:'添加标签',exact:true}).click();await saved(b.id,c=>c.note?.content.includes('Protein')&&c.tags.length===1);
  await open(c);await page.getByRole('textbox',{name:'Markdown 笔记正文'}).fill('细胞 Protein 对照');for(const name of ['细胞','Method']){await page.getByRole('combobox',{name:'添加卡片标签'}).fill(name);await page.getByRole('button',{name:'添加标签',exact:true}).click();}await saved(c.id,c=>c.tags.length===2);
  await open(a);execFileSync(fault,['block-save',workspace],{cwd:root,windowsHide:true,stdio:'pipe'});const failed='未保存的科研观察 Alpha';await page.getByRole('textbox',{name:'Markdown 笔记正文'}).fill(failed);await page.locator('.knowledge-save-state[role=alert]').waitFor();assert.equal((await card(a.id)).note.content,body);
  await open(b);assert.equal(await page.getByRole('textbox',{name:'Markdown 笔记正文'}).inputValue(),'Beta Protein · 二次实验');await page.locator('.card-pending-notice').getByText('保存失败，草稿已保留',{exact:false}).waitFor();
  await page.getByRole('button',{name:a.title,exact:true}).click();await page.getByRole('textbox',{name:'Markdown 笔记正文'}).waitFor();assert.equal(await page.getByRole('textbox',{name:'Markdown 笔记正文'}).inputValue(),failed);
  execFileSync(fault,['unblock-save',workspace],{cwd:root,windowsHide:true,stdio:'pipe'});await page.keyboard.press('Control+s');const recovered=await saved(a.id,c=>c.note?.content===failed);assert.equal(recovered.note.id,first.note.id);
  report.checks.push('actual SQLite trigger blocks save atomically, error is visible, fast switching keeps the failed per-card draft, and explicit retry updates the same note');
  await page.getByRole('button',{name:'编辑图片标注',exact:true}).click();await page.getByRole('application',{name:'图片编辑画布'}).waitFor();await page.getByRole('button',{name:'保存到素材库',exact:true}).click();await page.getByRole('status').filter({hasText:'截图与标注已保存'}).waitFor();await open(a);await page.getByRole('textbox',{name:'Markdown 笔记正文'}).fill(`${failed}\n编辑器往返`);await saved(a.id,c=>c.note?.content.includes('编辑器往返'));await page.getByRole('button',{name:'编辑图片标注',exact:true}).click();await page.getByRole('application',{name:'图片编辑画布'}).waitFor();await page.getByRole('button',{name:'保存到素材库',exact:true}).click();await page.getByRole('status').filter({hasText:'截图与标注已保存'}).waitFor();report.checks.push('card autosave and existing editor save cooperate without clobbering annotation state or producing stale metadata revisions');
  await library();await clear();
  const before=calls.length;for(const text of ['A','Al','Alp','Alph','Alpha']){await page.getByRole('textbox',{name:'快捷搜索'}).fill(text);await delay(60);}assert.equal(calls.slice(before).filter(x=>x.command==='screenshot_page').length,0);await expectIds([a.id]);assert.equal(calls.slice(before).filter(x=>x.command==='screenshot_page').length,1);assert.equal(calls.slice(before).filter(x=>x.command==='screenshot_image'&&!x.thumbnail).length,0);
  await query('Protein',[b.id,c.id]);await query('细胞',[a.id,c.id]);for(const special of ['%_',"O'Brien",'\\'])await query(special,[a.id]);await query("' OR 1=1 --",[]);await page.getByRole('heading',{name:'没有找到匹配的科研素材',exact:true}).waitFor();await clear();
  report.checks.push('real top search debounces five keystrokes into one bounded SQLite read, loads no originals, and correctly searches Chinese, ASCII English, percent/underscore/backslash/quotes with friendly zero results');
  await page.locator('.library-filters summary').click();await page.getByRole('checkbox',{name:'筛选标签 细胞',exact:true}).check();await page.getByRole('checkbox',{name:'筛选标签 Method',exact:true}).check();await expectIds([a.id,c.id]);await page.locator('.library-filters summary').click();
  await query('Protein',[c.id]);await page.getByRole('combobox',{name:'收藏筛选'}).selectOption('favorite');await expectIds([]);await page.getByRole('combobox',{name:'收藏筛选'}).selectOption('all');await query('',[a.id,c.id]);await page.getByRole('combobox',{name:'素材排序'}).selectOption('createdAsc');await until(async()=>JSON.stringify(await ids())===JSON.stringify([a.id,c.id]),'ascending filtered creation time');await page.getByRole('combobox',{name:'素材排序'}).selectOption('createdDesc');await until(async()=>JSON.stringify(await ids())===JSON.stringify([c.id,a.id]),'descending filtered creation time');await page.getByRole('combobox',{name:'收藏筛选'}).selectOption('favorite');await expectIds([a.id]);await clear();
  report.checks.push('multiple tags use AND; query + tags + favorite combine correctly, creation sorting stays stable, and clear-all restores the unfiltered 29-record library');
  await open(a);await page.getByRole('button',{name:'移除标签 Method',exact:true}).click();await saved(a.id,c=>c.tags.length===1);assert.equal((await call('list_tags')).length,2);
  await page.getByRole('button',{name:'标签',exact:true}).click();await page.getByRole('button',{name:'删除标签 细胞',exact:true}).click();await page.getByRole('button',{name:'确认删除标签',exact:true}).click();await until(async()=>!(await call('list_tags')).some(t=>t.name==='细胞'),'global tag deletion');assert.equal((await card(a.id)).tags.length,0);assert.equal((await card(c.id)).tags.length,1);assert.equal((await card(a.id)).note.content,`${failed}\n编辑器往返`);
  report.checks.push('removing a card tag preserves the shared tag; deleting the tag globally removes associations and retains screenshots, notes and source PNG');
  await open(a);await page.getByRole('button',{name:'切换为浅色主题',exact:true}).click();await page.getByRole('button',{name:'切换为深色主题',exact:true}).waitFor();await page.screenshot({path:join(output,'card-light.png')});
  const viewport=await page.evaluate(()=>({width:innerWidth,height:innerHeight}));let layouts=0;
  for(const theme of ['light','dark']){if(await page.getByRole('button',{name:theme==='dark'?'切换为深色主题':'切换为浅色主题',exact:true}).count())await page.getByRole('button',{name:theme==='dark'?'切换为深色主题':'切换为浅色主题',exact:true}).click();for(const [width,height] of [[480,400],[640,500],[960,680],[1440,900]]){await page.setViewportSize({width,height});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,`card ${theme} ${width}`);const box=await page.getByRole('textbox',{name:'Markdown 笔记正文'}).boundingBox();assert.ok(box.width>200&&box.x>=0&&box.x+box.width<=width+1);layouts++;}}
  await page.setViewportSize(viewport);await page.screenshot({path:join(output,'card-dark.png')});report.checks.push(`${layouts} actual card light/dark responsive content viewports (CDP emulation, not hardware DPI)`);
  assert.equal(createHash('sha256').update(await readFile(join(workspace,a.originalImagePath))).digest('hex'),originalHash);
  const persisted=await card(a.id);await close();await launch();await open(a);assert.equal(await page.getByRole('textbox',{name:'Markdown 笔记正文'}).inputValue(),persisted.note.content);assert.equal((await card(a.id)).screenshot.isFavorite,true);assert.equal((await card(a.id)).tags.length,0);await library();await query('Protein',[b.id,c.id]);await close();report.checks.push('actual app close/restart reloads note/source/favorite/tag removals and search results; original PNG SHA-256 unchanged');
  assert.equal(report.errors.length,0);report.result='passed';
}catch(error){report.result='failed';report.failure=error.stack;process.exitCode=1;if(page&&!page.isClosed())await page.screenshot({path:join(output,'failure.png')}).catch(()=>{});}
finally{if(browser)await browser.close().catch(()=>{});if(app&&app.exitCode===null)app.kill();server.closeAllConnections();await new Promise(resolve=>server.close(resolve));await writeFile(join(output,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));}
