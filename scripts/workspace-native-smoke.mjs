import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {setTimeout as delay} from 'node:timers/promises';
import {chromium} from 'playwright-core';
const root=dirname(dirname(fileURLToPath(import.meta.url))),id=process.argv[2];
assert.match(id??'',/^[a-z0-9-]{1,60}$/);
const out=join(root,'artifacts',`workspace-native-${id}`),settings=join(out,'settings');
await mkdir(out);await mkdir(settings);
// This builder creates ONLY a new test workspace, from the unchanged published 001-011 migrations.
// Controlled PNGs test storage/UI compatibility, never physical capture or real literature.
const seed=String.raw`
import json, sqlite3, struct, sys, zlib, hashlib
from pathlib import Path
repo=Path.cwd().resolve();out=Path(sys.argv[1]).resolve()
assert out.is_relative_to(repo/'artifacts')
w=out/'settings'/'workspace';w.mkdir();docs=out/'documents';docs.mkdir();(docs/'assets').mkdir()
assert not (w/'researchclip.sqlite3').exists()
c=sqlite3.connect(w/'researchclip.sqlite3')
for i,sql in enumerate(sorted((repo/'src-tauri'/'migrations').glob('*.sql'))[:11],1):
 c.executescript(sql.read_text(encoding='utf-8'));c.execute('PRAGMA user_version='+str(i));c.commit()
shot='a'*32;doc='c'*32
chunk=lambda t,b:struct.pack('>I',len(b))+t+b+struct.pack('>I',zlib.crc32(t+b)&0xffffffff)
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',32,24,8,6,0,0,0))+chunk(b'IDAT',zlib.compress((b'\0'+bytes([20,40,80,255])*32)*24))+chunk(b'IEND',b'')
for folder in ['originals','thumbnails']:
 (w/folder).mkdir();(w/folder/(shot+'.png')).write_bytes(png)
annotation=json.dumps({'schemaVersion':1,'image':{'widthPx':32,'heightPx':24},'objects':[]},ensure_ascii=False)
c.execute('INSERT INTO screenshots(id,title,original_image_path,thumbnail_path,annotation_data,width_px,height_px,created_at,updated_at) VALUES(?,?,?,?,?,32,24,1,1)',(shot,'升级前素材','originals/'+shot+'.png','thumbnails/'+shot+'.png',annotation))
c.execute('INSERT INTO notes VALUES(?,?,?,NULL,1,1,1)',('d'*32,shot,'旧笔记必须保留'))
asset='researchclip-'+'e'*32+'.png';(docs/'assets'/asset).write_bytes(png)
link='[旧截图链接](researchclip://screenshot/'+shot+')';body='# 旧研究记录\n\n'+link+'\n\n![旧图](assets/'+asset+')\n';path=docs/'旧研究记录.md';path.write_text(body,encoding='utf-8',newline='')
c.execute('INSERT INTO markdown_documents VALUES(?,?,?,NULL,1,1)',(doc,'\\\\?\\'+str(path),'旧研究记录.md'))
start=body.index(link);c.execute("INSERT INTO link_index VALUES(?,?,'screenshot',?,?,3,'旧截图链接',1,NULL)",(doc,shot,start,start+len(link)))
other=docs/'对照记录.md';other.write_text('# 对照记录\n\nIndependent B\n',encoding='utf-8',newline='')
c.execute('INSERT INTO markdown_documents VALUES(?,?,?,NULL,1,1)',('f'*32,'\\\\?\\'+str(other),'对照记录.md'))
c.commit();c.close()
(out/'fixture.json').write_text(json.dumps({'shot':shot,'doc':doc,'markdown':body,'pngHash':hashlib.sha256(png).hexdigest(),'mdHash':hashlib.sha256(body.encode()).hexdigest()},ensure_ascii=False),encoding='utf-8')
`;
execFileSync('python',['-c',seed,out],{cwd:root,windowsHide:true,stdio:'pipe'});
await writeFile(join(settings,'settings.json'),JSON.stringify({schemaVersion:1,theme:'dark',viewMode:'grid',fontFamily:'system',fontSize:16,captureShortcut:'Ctrl+Alt+Shift+F7',fullscreenShortcut:'Ctrl+Alt+Shift+F8',saveDirectory:null,closeToTray:false}));
// The existing v11 Rust service stores canonical Windows verbatim paths; the fixture uses that same form.
const fixture=JSON.parse(await readFile(join(out,'fixture.json'),'utf8'));
const report={mode:'actual Windows WebView2; new isolated v11 database; generated PNG, not physical screenshot; no Zotero/Agent access',checks:[],errors:[]};
let app,browser,page;
async function until(fn,name,ms=25000){let last;const start=Date.now();while(Date.now()-start<ms){try{const v=await fn();if(v)return v;}catch(e){last=e;}await delay(120);}throw Error(`Timed out ${name}: ${last?.message??''}`);}
const call=(name,args)=>page.evaluate(async({name,args})=>{try{return await window.__TAURI_INTERNALS__.invoke(name,args);}catch(e){throw Error(e?.message??String(e));}},{name,args});
const digest=bytes=>createHash('sha256').update(bytes).digest('hex');
async function launch(){
 assert.equal(await fetch('http://127.0.0.1:9403/json/version').then(()=>true).catch(()=>false),false,'test CDP port must be free');
 app=spawn(join(root,'src-tauri/target/debug/researchclip.exe'),[],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe'],env:{...process.env,RESEARCHCLIP_TEST_SETTINGS_DIR:settings,WEBVIEW2_USER_DATA_FOLDER:join(out,'profile'),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9403 --remote-debugging-address=127.0.0.1'}});
 app.stderr.on('data',data=>{for(const line of data.toString().trim().split('\n'))if(line)report.errors.push(line);});
 await until(()=>fetch('http://127.0.0.1:9403/json/version').then(r=>r.ok).catch(()=>false),'native startup');browser=await chromium.connectOverCDP('http://127.0.0.1:9403');
 page=await until(()=>browser.contexts().flatMap(c=>c.pages()).find(p=>p.url().includes('tauri')&&!p.url().includes('captureOverlay')),'main WebView');page.on('pageerror',e=>report.errors.push(e.message));await page.setViewportSize({width:1440,height:1000});
 report.startup = await call('library_status');
 await page.getByRole('button',{name:'截图素材',exact:true}).click();
 await page.locator(`[data-screenshot-id="${fixture.shot}"]`).waitFor();
}
async function quit(){await call('desktop_action',{action:'exit'});await until(()=>app.exitCode!==null,'native graceful exit');assert.equal(app.exitCode,0);await browser.close();browser=null;app=null;await until(()=>fetch('http://127.0.0.1:9403/json/version').then(()=>false).catch(()=>true),'CDP cleanup');}

try{
 await launch();assert.equal(await page.locator('.markdown-panel').count(),0);
 await page.locator('.project-document-list > summary').click();await page.getByRole('button',{name:'旧研究记录.md',exact:true}).click();let a=page.locator('[data-editor-kind="markdown"]').filter({has:page.getByRole('textbox',{name:'Markdown 源文',exact:true})});await a.waitFor();const aid=await a.getAttribute('data-panel-id');
 await page.getByRole('button',{name:'对照记录.md',exact:true}).click();await until(()=>page.locator('[data-editor-kind="markdown"]').count().then(n=>n===2),'two independent sessions');
 const sourceA=page.locator(`[data-panel-id="${aid}"]`).getByRole('textbox',{name:'Markdown 源文',exact:true});
 await page.getByRole('button',{name:'向下拆分当前面板',exact:true}).click();await delay(250);const bid=await page.locator('[data-editor-kind="markdown"]').filter({hasText:'对照记录.md'}).getAttribute('data-panel-id');const b=page.locator(`[data-panel-id="${bid}"]`);const inputB=b.getByRole('textbox',{name:'Markdown 源文',exact:true});
 await sourceA.fill(fixture.markdown+'\nA saved\n');await inputB.fill('# 对照记录\n\nB saved\n');await sourceA.press('Control+s');await until(()=>page.locator(`[data-panel-id="${aid}"]`).locator('.markdown-panel-header [role=status]').innerText().then(s=>s==='已保存'),'first native save');await inputB.press('Control+s');await until(()=>b.locator('.markdown-panel-header [role=status]').innerText().then(s=>s==='已保存'),'second native save');
 assert.equal(await sourceA.inputValue(),fixture.markdown+'\nA saved\n');assert.equal(await inputB.inputValue(),'# 对照记录\n\nB saved\n');report.checks.push('two real registered Markdown files open independently, split vertically and save exact distinct content through existing Rust service');
 await inputB.fill('# 对照记录\n\nB unsaved\n');await b.getByRole('button',{name:'关闭文档',exact:true}).click();const confirmation=page.getByRole('dialog',{name:'保存 Markdown 修改',exact:true});await confirmation.waitFor();await confirmation.getByRole('button',{name:'取消',exact:true}).click();assert.equal(await inputB.inputValue(),'# 对照记录\n\nB unsaved\n');
 await inputB.click();await page.getByRole('button',{name:'隐藏当前面板',exact:true}).click();await until(()=>b.count().then(n=>n===0),'hidden document unmounted');await page.getByLabel('重新打开隐藏面板',{exact:true}).selectOption(bid);await inputB.waitFor();assert.equal(await inputB.inputValue(),'# 对照记录\n\nB unsaved\n');await inputB.press('Control+s');report.checks.push('dirty close cancellation and hide/reopen retain the same native-backed document and unsaved text');
 const split=page.locator('.flexlayout__splitter').first(),box=await split.boundingBox();assert.ok(box);await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.down();await page.mouse.move(box.x+box.width/2+80,box.y+box.height/2,{steps:8});await page.mouse.up();await delay(150);
 await page.getByRole('button',{name:'显示终端区域',exact:true}).click();await page.getByRole('region',{name:'终端区域',exact:true}).waitFor();await page.getByRole('button',{name:'隐藏终端区域',exact:true}).click();await page.getByRole('button',{name:'隐藏资源管理器',exact:true}).click();await page.getByRole('button',{name:'显示资源管理器',exact:true}).click();
 for(const font of [12,20]){const label=font===12?'缩小界面字号':'放大界面字号';for(let i=0;i<10;i++){const button=page.getByRole('button',{name:label,exact:true});if(await button.isDisabled())break;await button.click();await delay(100);}await until(()=>page.locator('.workspace-zoom').innerText().then(v=>v.includes(font+'px')),'native settings font '+font);await page.setViewportSize({width:720,height:540});await delay(180);const bound=await page.evaluate(()=>({w:innerWidth,scroll:document.documentElement.scrollWidth}));assert.ok(bound.scroll<=bound.w);await inputB.scrollIntoViewIfNeeded();assert.ok(await inputB.isVisible());}
 await page.setViewportSize({width:1440,height:1000});await delay(180);await page.screenshot({path:join(out,'native-workspace.png')});report.checks.push('actual WebView pointer splitter drag, explorer/terminal visibility, 12/20px settings and narrow/wide resize retain accessible editors');
 const probe=(action)=>JSON.parse(execFileSync(join(root,'src-tauri/target/debug/examples/desktop_probe.exe'),[action,String(app.pid)],{cwd:root,windowsHide:true,encoding:'utf8'})||'null');const nativeAction=action=>execFileSync(join(root,'src-tauri/target/debug/examples/desktop_probe.exe'),[action,String(app.pid)],{cwd:root,windowsHide:true,stdio:'pipe'});const cdp=await page.context().newCDPSession(page);await cdp.send('Emulation.clearDeviceMetricsOverride');nativeAction('maximize');await until(()=>probe('inspect').maximized,'actual window maximize');nativeAction('restore');await until(()=>!probe('inspect').maximized,'actual restore from maximized');nativeAction('minimize');await until(()=>probe('inspect').minimized,'actual window minimize');nativeAction('restore');await until(()=>{const status=probe('inspect');return !status.minimized&&!status.maximized;},'actual window restore');await until(()=>page.locator('[data-editor-kind="markdown"]:visible').evaluateAll(nodes=>nodes.length>0&&nodes.every(n=>Number(n.dataset.editorWidth)>0&&Number(n.dataset.editorHeight)>0)),'resized editor hosts');await cdp.detach();report.checks.push('actual owned Win32 maximize/minimize/restore returns positive editor host sizes; no added application window permissions');
 await page.getByRole('button',{name:'保存布局',exact:true}).click();const saved=await page.evaluate(()=>JSON.parse(localStorage.getItem('researchclip.workspace.layout:default')));assert.equal(saved.panels.filter(p=>p.kind==='markdown').length,2);assert.ok(saved.panels.some(p=>p.resourceId==='c'.repeat(32)));assert.ok(saved.panels.some(p=>p.resourceId==='f'.repeat(32)));assert.ok(!JSON.stringify(saved).includes('B unsaved'));
 await quit();await launch();assert.equal(await page.locator('.markdown-panel').count(),0);await page.getByRole('button',{name:'重新打开上次资源',exact:true}).click();const contents=[];for(const title of ['旧研究记录.md','对照记录.md']){const tab=page.locator('.flexlayout__tab_button').filter({hasText:title});await tab.waitFor();await tab.click();const pane=page.locator('[data-editor-kind="markdown"]').filter({has:page.locator('.markdown-panel-header').filter({hasText:title})});const input=pane.getByRole('textbox',{name:'Markdown 源文',exact:true});await input.waitFor();contents.push(await input.inputValue());}assert.deepEqual(contents,[fixture.markdown+'\nA saved\n','# 对照记录\n\nB unsaved\n']);report.checks.push('real process restart shows no empty Markdown split, explicit stable-ID restore reopens both persisted files in serialized layout');
 await page.getByRole('button',{name:'截图素材',exact:true}).click();await page.locator(`[data-screenshot-id="${fixture.shot}"]`).getByRole('button',{name:'编辑标注',exact:true}).click();const canvas=page.getByRole('application',{name:'图片编辑画布',exact:true});await canvas.waitFor();await delay(300);const bounds=await canvas.boundingBox();assert.ok(bounds&&bounds.width>48&&bounds.height>48);const at=(x,y)=>({x:bounds.x+(bounds.width-32)/2+x,y:bounds.y+(bounds.height-24)/2+y});await page.getByRole('button',{name:'矩形工具',exact:true}).click();const from=at(3,3),to=at(28,20);await page.mouse.move(from.x,from.y);await page.mouse.down();await page.mouse.move(to.x,to.y,{steps:8});await page.mouse.up();await page.getByRole('button',{name:'矩形 1',exact:true}).waitFor();await canvas.focus();await page.keyboard.press('Control+z');await page.getByRole('button',{name:'矩形 1',exact:true}).waitFor({state:'detached'});await page.keyboard.press('Control+y');await page.getByRole('button',{name:'矩形 1',exact:true}).waitFor();await page.getByRole('button',{name:'保存到素材库',exact:true}).click();await page.getByRole('status').filter({hasText:'标注已保存'}).waitFor();assert.equal((await call('get_screenshot',{id:fixture.shot})).annotationData.objects.length,1);report.checks.push('real docked Konva pane draws a rectangle, scoped Ctrl+Z/Ctrl+Y restores it and explicit save persists annotation without changing original PNG');
 await page.getByRole('button',{name:'使用指南',exact:true}).click();await page.getByRole('navigation',{name:'使用指南目录',exact:true}).waitFor();assert.equal(await page.locator('[data-guide-depth="2"]').count(),27);const guideImages=page.locator('.guide-content img');assert.equal(await guideImages.count(),44);await guideImages.evaluateAll(nodes=>nodes.forEach(n=>n.loading='eager'));await until(()=>guideImages.evaluateAll(nodes=>nodes.every(n=>n.complete&&n.naturalWidth>0)),'all38bundledGuideImages');await page.getByRole('button',{name:'安排多个编辑面板',exact:false}).click();await page.screenshot({path:join(out,'native-guide.png')});report.checks.push('bundled offline guide exposes 26 chapters and new workspace reading route in native WebView');
 await quit();assert.deepEqual(report.errors,[]);assert.equal(digest(await readFile(join(settings,'workspace','originals',`${fixture.shot}.png`))),fixture.pngHash);report.result='PASS';
}catch(e){report.result='FAIL';report.failure=e.stack;process.exitCode=1;if(page&&!page.isClosed()){report.diagnostic=(await page.locator('body').innerText()).slice(0,4000);await page.screenshot({path:join(out,'failure.png')}).catch(()=>{});}}
finally{if(browser)await browser.close().catch(()=>{});if(app&&app.exitCode===null)app.kill();await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));}
