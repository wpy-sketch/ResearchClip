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
const out=join(root,'artifacts',`resources-native-${id}`),settings=join(out,'settings');
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
 await launch();const status=await call('get_workspace_status');assert.equal(status.schemaVersion,13);report.checks.push('real app upgrades independent v11 fixture to schema13 without resetting existing library');
 const shot=await call('get_screenshot',{id:fixture.shot});assert.equal(shot.title,'升级前素材');assert.equal(shot.widthPx,32);assert.equal(shot.heightPx,24);assert.deepEqual(shot.annotationData.objects,[]);
 const bytes=await page.evaluate(async id=>Array.from(new Uint8Array(await window.__TAURI_INTERNALS__.invoke('screenshot_image',{id,thumbnail:false}))),fixture.shot);assert.equal(digest(Buffer.from(bytes)),fixture.pngHash);
 report.checks.push('unchanged screenshot ID, annotation and exact original PNG bytes load through real IPC');
 await page.locator('.project-document-list > summary').click();await page.getByRole('button',{name:'旧研究记录.md',exact:true}).click();const source=page.getByRole('textbox',{name:'Markdown 源文',exact:true});await source.waitFor();assert.equal(await source.inputValue(),fixture.markdown);
 await page.getByRole('button',{name:'预览 Markdown',exact:true}).click();const image=page.locator('.markdown-panel').getByRole('img',{name:'旧图',exact:true});await image.waitFor();await image.evaluate(n=>n.decode());
 report.checks.push('old registered Markdown ID opens in existing right pane with exact UTF-8 text and relative PNG assets');
 await page.locator('.markdown-panel .internal-link').filter({hasText:'旧截图链接'}).click();await page.getByRole('button',{name:'编辑图片标注',exact:true}).click();await page.getByRole('application',{name:'图片编辑画布'}).waitFor();
 report.checks.push('unchanged researchclip screenshot URI navigates from old Markdown preview to the existing card and actual Konva editor while preserving right document');
 await page.getByRole('button',{name:'继续编辑',exact:true}).click();assert.equal(await source.inputValue(),fixture.markdown);await page.getByRole('button',{name:'关闭文档',exact:true}).click();await quit();
 await launch();await page.locator('.project-document-list > summary').click();await page.getByRole('button',{name:'旧研究记录.md',exact:true}).click();const reopened=page.getByRole('textbox',{name:'Markdown 源文',exact:true});await reopened.waitFor();assert.equal(await reopened.inputValue(),fixture.markdown);await page.getByRole('button',{name:'关闭文档',exact:true}).click();await quit();
 report.checks.push('real process restart retains old document association, content and schema13 library');
 const inspect=String.raw`import sqlite3,json,sys
from pathlib import Path
out=Path(sys.argv[1]);root=out/'settings'/'workspace'
c=sqlite3.connect((root/'researchclip.sqlite3').as_uri()+'?mode=ro',uri=True)
assert c.execute('PRAGMA user_version').fetchone()[0]==13
assert c.execute('SELECT count(*) FROM screenshots').fetchone()[0]==1
assert c.execute('SELECT content FROM notes').fetchone()[0]=='旧笔记必须保留'
assert c.execute('SELECT id FROM markdown_documents').fetchall()==[('c'*32,)]
assert c.execute('SELECT document_id FROM research_document_fingerprints').fetchall()==[('c'*32,)]
assert c.execute('SELECT document_id FROM document_organization').fetchall()==[('c'*32,)]
assert c.execute('SELECT document_id,version_number FROM markdown_revisions').fetchall()==[('c'*32,1)]
c.close()
backups=list((root/'backups').glob('schema-11-*.sqlite3'));assert len(backups)==1
c=sqlite3.connect(backups[0].as_uri()+'?mode=ro',uri=True);assert c.execute('PRAGMA user_version').fetchone()[0]==11;c.close()`;
 execFileSync('python',['-c',inspect,out],{cwd:root,windowsHide:true,stdio:'pipe'});
 assert.equal(digest(await readFile(join(out,'documents','旧研究记录.md'))),fixture.mdHash);assert.equal(digest(await readFile(join(settings,'workspace','originals',`${fixture.shot}.png`))),fixture.pngHash);
 assert.equal(report.errors.length,0);report.checks.push('read-only post-exit audit verifies v11 safety snapshot, schema13, note, fingerprint and unchanged original/Markdown hashes');report.result='PASS';
}catch(e){report.result='FAIL';report.failure=e.stack;process.exitCode=1;if(page&&!page.isClosed()){report.diagnostic=(await page.locator('body').innerText()).slice(0,3500);await page.screenshot({path:join(out,'failure.png')}).catch(()=>{});}}
finally{
 if(browser)await browser.close().catch(()=>{});if(app&&app.exitCode===null)app.kill();
 await writeFile(join(out,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
}
