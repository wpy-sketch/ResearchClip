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
const out=join(root,'artifacts',`markdown-workspace-native-${id}`),settings=join(out,'settings');
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
title='图文格式练习';block='e'*32;caption='图片下面的中文注释';imagebody='![练习图](assets/'+asset+')\n\n'+caption
head=lambda kind,value:'<!-- ResearchClip:'+kind+' '+json.dumps(value,ensure_ascii=False,separators=(',',':'))+' -->\n'
structured=head('document',{'version':1,'id':'8'*32,'title':title})+'# '+title+'\n\n'+head('block',{'id':block,'type':'image','assetName':asset,'sourceScreenshotId':shot,'alt':'练习图','length':len(imagebody)})+imagebody+'\n<!-- ResearchClip:end -->\n\n'
sp=docs/'图文格式练习.md';sp.write_text(structured,encoding='utf-8',newline='')
c.execute('INSERT INTO markdown_documents VALUES(?,?,?,NULL,1,1)',('9'*32,'\\\\?\\'+str(sp),'图文格式练习.md'))
c.commit();c.close()
(out/'fixture.json').write_text(json.dumps({'shot':shot,'doc':doc,'markdown':body,'pngHash':hashlib.sha256(png).hexdigest(),'mdHash':hashlib.sha256(body.encode()).hexdigest()},ensure_ascii=False),encoding='utf-8')
`;
execFileSync('python',['-c',seed,out],{cwd:root,windowsHide:true,stdio:'pipe'});
await writeFile(join(settings,'settings.json'),JSON.stringify({schemaVersion:1,theme:'dark',viewMode:'grid',fontFamily:'system',fontSize:16,captureShortcut:'Ctrl+Alt+Shift+F7',fullscreenShortcut:'Ctrl+Alt+Shift+F8',saveDirectory:null,closeToTray:false}));
// The existing v11 Rust service stores canonical Windows verbatim paths; the fixture uses that same form.
const originalMixed=(await readFile(join(root,'docs/examples/markdown-fidelity-mixed.md'),'utf8')).replace(/\r?\n/g,'\r\n');await writeFile(join(out,'documents','旧研究记录.md'),originalMixed);
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

async function open(title){await page.keyboard.press('Control+Shift+p');await page.getByRole('option').filter({hasText:'打开最近文档'}).click();await page.getByRole('dialog',{name:'打开最近文档',exact:true}).getByRole('button').filter({hasText:title}).click();await page.locator('.markdown-panel-header').filter({hasText:title}).waitFor();return page.locator('[data-markdown-panel]').filter({has:page.locator('.markdown-panel-header').filter({hasText:title})});}
async function select(input,start,end){await input.evaluate((node,{start,end})=>{node.focus();node.setSelectionRange(start,end);node.dispatchEvent(new Event('select',{bubbles:true}));},{start,end});await delay(100);}
try{
 await launch();assert.equal(await page.locator('[data-markdown-panel]').count(),0);const a=await open('旧研究记录.md'),modes=a.getByRole('group',{name:'Markdown 编辑模式',exact:true}),input=a.getByRole('textbox',{name:'Markdown 源文',exact:true}),disk=join(out,'documents','旧研究记录.md');
 for(let i=0;i<2;i++)for(const mode of ['近所见即所得','只读预览','实时预览','源码']){await modes.getByRole('button',{name:mode,exact:true}).click();await delay(70);assert.equal(await a.locator('.markdown-panel-header [role=status]').innerText(),'已保存');assert.equal(await a.getByRole('toolbar',{name:'Markdown 格式工具栏',exact:true}).count(),1);}
 await input.press('Control+s');await until(()=>a.locator('.markdown-notice').innerText().then(v=>v.includes('未重写文件')),'clean save');assert.equal(await readFile(disk,'utf8'),originalMixed);report.checks.push('eight native mode switches and clean Ctrl+S preserve exact original CRLF complex file, math, table, title, relative asset and unknown syntax');
 await modes.getByRole('button',{name:'近所见即所得',exact:true}).click();await until(()=>a.locator('.md-block-render img').count().then(n=>n===1),'titled image preview');await a.locator('.md-block-render img').evaluate(n=>n.loading='eager');await until(()=>a.locator('.md-block-render img').evaluate(n=>n.complete&&n.naturalWidth>0),'native relative image decode');const paragraph=a.getByRole('button',{name:/编辑 Markdown 块/}).filter({hasText:'前缀'});await paragraph.click();const active=a.getByRole('textbox',{name:'可视块源码',exact:true});const text=await active.inputValue(),start=text.indexOf('前缀');await select(active,start,start+2);await active.press('Control+b');await active.press('Control+s');await until(()=>a.locator('.markdown-panel-header [role=status]').innerText().then(v=>v==='已保存'),'visual native save');const exact=originalMixed.replace('前缀 **粗体**','**前缀** **粗体**');assert.equal(await readFile(disk,'utf8'),exact);await active.press('Control+z');await active.press('Control+s');await until(async()=>await readFile(disk,'utf8')===originalMixed,'visual undo save');await a.getByRole('button',{name:'完成编辑',exact:true}).click();await page.screenshot({path:join(out,'native-visual.png')});report.checks.push('titled managed image decodes; visual selected-text formatting and undo save through original Rust service while every other source byte is unchanged');
 await modes.getByRole('button',{name:'源码',exact:true}).click();for(const [command,expected] of [['heading','# '],['table','| 列一 | 列二 |'],['math','$$'],['code','```'],['quote','> ']]){await input.fill('/'+command);await select(input,command.length+1,command.length+1);await a.getByRole('listbox',{name:'Markdown 斜杠命令',exact:true}).waitFor();await input.press('Enter');await until(()=>input.inputValue().then(v=>v.includes(expected)&&!v.includes('/'+command)),command);await input.press('Control+z');assert.equal(await input.inputValue(),'/'+command);}
 await input.fill('/image');await select(input,6,6);await a.getByRole('listbox').waitFor();await input.press('Enter');let picker=page.getByRole('dialog',{name:'从素材库添加图片',exact:true});await picker.getByRole('button',{name:'取消',exact:true}).click();assert.equal(await input.inputValue(),'/image');await select(input,6,6);await a.getByRole('listbox').waitFor();await input.press('Enter');picker=page.getByRole('dialog',{name:'从素材库添加图片',exact:true});await picker.getByRole('checkbox',{name:'添加图片 升级前素材',exact:true}).check();await picker.getByRole('button',{name:'添加所选 1 张',exact:true}).click();await until(()=>input.inputValue().then(v=>v.includes('](assets/')&&!v.includes('/image')),'native slash image');await input.press('Control+z');assert.equal(await input.inputValue(),'/image');report.checks.push('all six slash commands run in native WebView; image cancel is unchanged, actual library insert creates relative asset and undoes as one action');
 const dragRaw='# 标题\n\n![图](assets/researchclip-'+'e'.repeat(32)+'.png)\n\n说明甲\n\n说明乙\n\n## 后文\n\n独立段\n';await input.fill(dragRaw);await modes.getByRole('button',{name:'近所见即所得',exact:true}).click();await page.setViewportSize({width:1440,height:1800});const handle=a.getByRole('button',{name:'拖动 Markdown 块 2',exact:true}),target=a.locator('.md-drop-end'),hb=await handle.boundingBox(),tb=await target.boundingBox();await page.mouse.move(hb.x+hb.width/2,hb.y+hb.height/2);await page.mouse.down();await page.mouse.move(hb.x+hb.width/2+8,hb.y+hb.height/2,{steps:4});await page.mouse.move(tb.x+tb.width/2,tb.y+tb.height/2,{steps:15});await page.mouse.up();await delay(120);await modes.getByRole('button',{name:'源码',exact:true}).click();assert.ok((await input.inputValue()).indexOf('## 后文')<(await input.inputValue()).indexOf('![图]'));assert.ok((await input.inputValue()).includes('说明甲\n\n说明乙'));await input.press('Control+z');assert.equal(await input.inputValue(),dragRaw);report.checks.push('native HTML5 pointer drag moves image and both caption paragraphs together; original undo restores all source');
 await input.fill(Array.from({length:100},(_,i)=>`## 研究 ${i}\n\n内容 ${i} $x$\n\n`).join(''));await modes.getByRole('button',{name:'实时预览',exact:true}).click();await page.setViewportSize({width:1440,height:1000});const live=a.getByLabel('实时预览区域',{exact:true});for(const [from,to] of [[input,live],[live,input]]){await from.evaluate(n=>{n.scrollTop=(n.scrollHeight-n.clientHeight)*.55;});await delay(220);const ratios=await Promise.all([from,to].map(n=>n.evaluate(n=>n.scrollTop/Math.max(1,n.scrollHeight-n.clientHeight))));assert.ok(Math.abs(ratios[0]-ratios[1])<.04,JSON.stringify(ratios));}await a.getByRole('button',{name:'关闭文档',exact:true}).click();await page.getByRole('dialog',{name:'保存 Markdown 修改',exact:true}).getByRole('button',{name:'取消',exact:true}).click();assert.equal(await input.count(),1);await input.press('Control+s');await until(()=>a.locator('.markdown-panel-header [role=status]').innerText().then(v=>v==='已保存'),'save long source');const finalSource=await readFile(disk,'utf8');report.checks.push('native source/preview bidirectional ratio scroll, dirty-close cancel and Ctrl+S use existing session lifecycle');
 const b=await open('图文格式练习.md'),bmodes=b.getByRole('group',{name:'Markdown 编辑模式',exact:true}),legacyPath=join(out,'documents','图文格式练习.md'),legacy=await readFile(legacyPath,'utf8');for(const mode of ['近所见即所得','只读预览','实时预览','源码'])await bmodes.getByRole('button',{name:mode,exact:true}).click();await b.getByRole('textbox',{name:'图片注释 1',exact:true}).press('Control+s');assert.equal(await readFile(legacyPath,'utf8'),legacy);await b.getByRole('button',{name:'添加 Markdown 文本',exact:true}).click();await b.getByRole('textbox',{name:'Markdown 文本 2',exact:true}).fill('独立后文');await bmodes.getByRole('button',{name:'近所见即所得',exact:true}).click();await page.setViewportSize({width:1440,height:1800});const lh=await b.getByRole('button',{name:'拖动图文块 1',exact:true}).boundingBox(),lt=await b.locator('.md-drop-end').boundingBox();await page.mouse.move(lh.x+lh.width/2,lh.y+lh.height/2);await page.mouse.down();await page.mouse.move(lh.x+lh.width/2+8,lh.y+lh.height/2,{steps:4});await page.mouse.move(lt.x+lt.width/2,lt.y+lt.height/2,{steps:15});await page.mouse.up();await delay(150);assert.equal(await b.locator('[data-block-id]').last().getAttribute('data-block-id'),'e'.repeat(32));await b.getByRole('button',{name:'拖动图文块 2',exact:true}).focus();await page.keyboard.press('Control+z');assert.equal(await b.locator('[data-block-id]').first().getAttribute('data-block-id'),'e'.repeat(32));await bmodes.getByRole('button',{name:'源码',exact:true}).click();await b.getByRole('textbox',{name:'图片注释 1',exact:true}).press('Control+s');await until(()=>b.locator('.markdown-panel-header [role=status]').innerText().then(v=>v==='已保存'),'legacy save');const legacyFinal=await readFile(legacyPath,'utf8');assert.ok(legacyFinal.includes('"sourceScreenshotId":"'+fixture.shot+'"'));assert.ok(legacyFinal.includes('图片下面的中文注释'));report.checks.push('legacy mode cycles preserve exact private-format bytes; actual whole-image/caption drag and undo retain block ID and screenshot association');
 await quit();await launch();assert.equal(await page.locator('[data-markdown-panel]').count(),0);const reopened=await open('旧研究记录.md');assert.equal(await reopened.getByRole('textbox',{name:'Markdown 源文',exact:true}).inputValue(),finalSource.replace(/\r\n/g,'\n'));await open('图文格式练习.md');await quit();assert.equal(digest(await readFile(join(settings,'workspace','originals',`${fixture.shot}.png`))),fixture.pngHash);assert.equal(digest(await readFile(join(out,'documents','assets','researchclip-'+'e'.repeat(32)+'.png'))),fixture.pngHash);assert.deepEqual(report.errors,[]);report.checks.push('actual process restart restores saved ordinary and legacy Markdown; original asset bytes remain unchanged');report.result='PASS';
}catch(e){report.result='FAIL';report.failure=e.stack;process.exitCode=1;if(page&&!page.isClosed())await page.screenshot({path:join(out,'failure.png')}).catch(()=>{});}
finally{if(browser)await browser.close().catch(()=>{});if(app&&app.exitCode===null)app.kill();await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));}
