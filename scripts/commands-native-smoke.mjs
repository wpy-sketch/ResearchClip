import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {mkdir,readFile,writeFile,rename} from 'node:fs/promises';
import {join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {setTimeout as delay} from 'node:timers/promises';
import {chromium} from 'playwright-core';
const root=dirname(dirname(fileURLToPath(import.meta.url))),id=process.argv[2];
assert.match(id??'',/^[a-z0-9-]{1,60}$/);
const out=join(root,'artifacts',`commands-native-${id}`),settings=join(out,'settings');
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
shot='a'*32;doc='a'*32
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

let held=false;const original=join(settings,'workspace','originals',`${fixture.shot}.png`),temporary=original+'.qa-held';
const envelope=(name,args)=>({version:1,requestId:'1'.repeat(32),command:{name,args}}),key=(kind,id)=>({kind,id});
async function raw(name,args){return page.evaluate(async({name,args})=>{try{return {ok:true,value:await window.__TAURI_INTERNALS__.invoke(name,args)};}catch(e){return {ok:false,code:e?.code??null,message:typeof e==='string'?e:e?.message??String(e)};}},{name,args});}
async function recent(){await page.keyboard.press('Control+Shift+p');await page.getByRole('option').filter({hasText:'打开最近文档'}).click();const modal=page.getByRole('dialog',{name:'打开最近文档',exact:true});await modal.waitFor();await modal.getByRole('button').filter({hasText:'旧研究记录.md'}).click();}
async function delayPrepare(enabled){await page.evaluate(enabled=>{if(enabled){if(window.__RC_QA_ORIGINAL_FETCH)throw Error('Owned injection already active');const original=window.fetch;window.__RC_QA_ORIGINAL_FETCH=original;window.__RC_QA_PENDING=[];const delayed=async(input,options)=>{const url=new URL(typeof input==='string'?input:input.url);if(url.hostname==='ipc.localhost'&&url.pathname==='/prepare_research_command')await new Promise(resolve=>window.__RC_QA_PENDING.push(resolve));return original(input,options);};window.fetch=delayed;if(window.fetch!==delayed)throw Error('Fetch delay was not installed');}else{if(!window.__RC_QA_ORIGINAL_FETCH)throw Error('Owned injection missing');const pending=window.__RC_QA_PENDING;window.fetch=window.__RC_QA_ORIGINAL_FETCH;delete window.__RC_QA_ORIGINAL_FETCH;delete window.__RC_QA_PENDING;for(const resolve of pending)resolve();}},enabled);}

try{
 await launch();const doc=key('document',fixture.doc),shot=key('screenshot',fixture.shot);
 const resolved=await call('resolve_research_resource',{request:{version:1,key:doc}});assert.deepEqual(Object.keys(resolved).sort(),['editor','formatVersion','key','moduleAvailable']);assert.deepEqual(resolved.key,doc);assert.equal(resolved.editor,'markdown');assert.equal(resolved.moduleAvailable,true);
 const prepared=await call('prepare_research_command',{request:envelope('markdown.insertImage',{document:doc,image:shot})});assert.equal(prepared.length,2);assert.equal(prepared[1].editor,'image');assert.ok(!JSON.stringify(prepared).includes('filePath'));
 for(const request of [{...envelope('document.open',{resource:doc}),actor:'admin'},envelope('shell.execute',{}),envelope('document.open',{resource:{...doc,id:'C:/private'}}),envelope('document.open',{resource:doc,path:'secret'}),{...envelope('document.open',{resource:doc}),version:9}])assert.equal((await raw('prepare_research_command',{request})).ok,false);
 const missing=await raw('prepare_research_command',{request:envelope('document.open',{resource:key('document','9'.repeat(32))})});assert.equal(missing.code,'NOT_FOUND');
 for(const [name,args] of [['pdf.navigateTo',{location:key('pdfLocation','2'.repeat(32))}],['screenshot.capturePdfSelection',{location:key('pdfLocation','2'.repeat(32))}],['annotation.create',{location:key('pdfLocation','2'.repeat(32)),annotationType:'ink'}],['codeWorkspace.open',{workspace:key('codeWorkspace','2'.repeat(32))}],['agent.requestEdit',{session:key('agentSession','2'.repeat(32)),document:doc,expectedRevision:1}]]){const result=await raw('prepare_research_command',{request:envelope(name,args)});assert.equal(result.code,'MODULE_UNAVAILABLE');}
 report.checks.push('real bounded Rust IPC resolves stable IDs without paths/body/token, rejects malformed/unknown/caller-privilege/future requests and missing IDs; all five future modules explicitly reject without execution');
 // Delay only the actual metadata gate. It still calls real Rust; no fake result or application debug API is installed.
 await delayPrepare(true);await recent();await until(()=>page.evaluate(()=>window.__RC_QA_PENDING.length===1),'owned real IPC hold');await call('desktop_action',{action:'exit'});await page.getByText('退出已取消：科研资源命令尚未结束，请取消或等待完成后重试。',{exact:true}).waitFor();assert.equal(app.exitCode,null);report.checks.push('actual native quit request is denied while a controlled task is running, retaining the window and all document state');await page.getByRole('button',{name:'取消任务',exact:true}).click();await delayPrepare(false);assert.equal(await page.locator('.markdown-panel').count(),0);await page.getByRole('button',{name:'查看命令任务与诊断',exact:true}).click();let diagnostic=page.getByRole('dialog',{name:'命令任务与诊断',exact:true});await diagnostic.waitFor();await until(()=>diagnostic.innerText().then(t=>t.includes('已取消')),'cancelled diagnostic');await page.keyboard.press('Escape');
 report.checks.push('owned delay fault injection before real Rust lookup allows cooperative cancellation; no editor is opened and diagnostics accurately show cancelled');
 await recent();const sourceA=page.getByRole('textbox',{name:'Markdown 源文',exact:true});await sourceA.waitFor();const panelA=await page.locator('[data-editor-kind="markdown"]').getAttribute('data-panel-id');assert.equal(await sourceA.inputValue(),fixture.markdown);
 report.checks.push('existing recent-document UI now routes through typed registry and native gate into the original Markdown service');
 await page.getByRole('button',{name:'预览 Markdown',exact:true}).click();await page.locator('.markdown-panel .internal-link').filter({hasText:'旧截图链接'}).click();await page.keyboard.press('Control+Shift+p');await page.getByRole('option').filter({hasText:'打开当前素材面板'}).click();await page.getByRole('application',{name:'图片编辑画布',exact:true}).waitFor();assert.equal(await page.locator('[data-editor-kind="image"]').count(),1);await page.locator('.flexlayout__tab_button').filter({hasText:'旧研究记录.md'}).click();await page.locator('.markdown-panel').getByRole('button',{name:'继续编辑',exact:true}).click();assert.equal(await sourceA.inputValue(),fixture.markdown);report.checks.push('document and screenshot deliberately share the same hex ID in this fixture; the typed command opens the correct image editor while preserving the distinct Markdown session');

 await delayPrepare(true);await page.getByRole('button',{name:'截图素材',exact:true}).click();const insert=page.locator(`[data-screenshot-id="${fixture.shot}"]`).getByRole('button',{name:'插入到 Markdown',exact:true});await insert.click();await page.getByRole('button',{name:'新建空白 Markdown',exact:true}).click();const a=page.locator(`[data-panel-id="${panelA}"]`),b=page.locator(`[data-editor-kind="markdown"]:not([data-panel-id="${panelA}"])`),inputA=a.getByRole('textbox',{name:'Markdown 源文',exact:true});await b.getByRole('textbox',{name:'Markdown 源文',exact:true}).fill('B stays untouched');await delayPrepare(false);await until(()=>inputA.inputValue().then(text=>(text.match(/assets\//g)??[]).length===2),'stable-target insert');assert.equal(await b.getByRole('textbox',{name:'Markdown 源文',exact:true}).inputValue(),'B stays untouched');await inputA.press('Control+z');assert.equal(await inputA.inputValue(),fixture.markdown);await b.getByRole('button',{name:'关闭文档',exact:true}).click();await page.getByRole('dialog',{name:'保存 Markdown 修改',exact:true}).getByRole('button',{name:'放弃修改',exact:true}).click();
 report.checks.push('controlled image insertion remains bound to the registered document ID while focus changes to another draft; only target A changes, Ctrl+Z restores it and draft B stays independent');
 await page.getByRole('button',{name:'截图素材',exact:true}).click();const before=await inputA.inputValue();await rename(original,temporary);held=true;await insert.click();await page.locator('.application-command-error').waitFor();assert.equal(await page.locator('.application-command-status').getByText('素材图已插入指定 Markdown 草稿；请另行保存。',{exact:true}).count(),0);assert.equal(await inputA.inputValue(),before);await rename(temporary,original);held=false;
 await page.getByRole('button',{name:'查看命令任务与诊断',exact:true}).click();diagnostic=page.getByRole('dialog',{name:'命令任务与诊断',exact:true});await diagnostic.waitFor();const text=await diagnostic.innerText();assert.ok(text.includes('未完成'));assert.ok(!text.includes('qa-held')&&!text.includes('D:\\')&&!text.includes('旧笔记必须保留'));await page.screenshot({path:join(out,'native-diagnostics.png')});await page.keyboard.press('Escape');
 report.checks.push('owned missing-PNG fault triggers a failed task and safe visible diagnostic, preserves Markdown exactly, restores the test file and releases resource locks');
 await page.getByRole('button',{name:'截图素材',exact:true}).click();await insert.click();await until(()=>inputA.inputValue().then(t=>t!==before),'explicit fresh retry succeeds');await inputA.press('Control+z');assert.equal(await inputA.inputValue(),before);await page.setViewportSize({width:720,height:540});await page.getByRole('button',{name:'查看命令任务与诊断',exact:true}).click();diagnostic=page.getByRole('dialog',{name:'命令任务与诊断',exact:true});await diagnostic.waitFor();assert.ok(await diagnostic.getByText('模块接入状态',{exact:true}).isVisible());assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.keyboard.press('Escape');await page.setViewportSize({width:1440,height:1000});
 report.checks.push('new explicit request succeeds after file restoration; diagnostic modal and controls remain accessible in narrow native viewport');
 await a.getByRole('button',{name:'关闭文档',exact:true}).click();const confirmation=page.getByRole('dialog',{name:'保存 Markdown 修改',exact:true});if(await confirmation.isVisible())await confirmation.getByRole('button',{name:'放弃修改',exact:true}).click();await until(()=>page.locator('.markdown-panel').count().then(n=>n===0),'document close');await quit();assert.equal(digest(await readFile(original)),fixture.pngHash);assert.equal(digest(await readFile(join(out,'documents','旧研究记录.md'))),fixture.mdHash);assert.deepEqual(report.errors,[]);report.result='PASS';
}catch(e){report.result='FAIL';report.failure=e.stack;process.exitCode=1;if(page&&!page.isClosed()){report.diagnostic=(await page.locator('body').innerText()).slice(0,4500);await page.screenshot({path:join(out,'failure.png')}).catch(()=>{});}}
finally{if(held)await rename(temporary,original);if(browser)await browser.close().catch(()=>{});if(app&&app.exitCode===null)app.kill();await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));}
