import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {setTimeout as delay} from 'node:timers/promises';
import {chromium} from 'playwright-core';
const root=dirname(dirname(fileURLToPath(import.meta.url))),id=process.argv[2]??Date.now().toString();assert.match(id,/^[a-z0-9-]{1,64}$/);
const output=join(root,'artifacts',`openwith-${id}`);await mkdir(output);const settings=join(output,'settings'),exportsDir=join(output,'exports'),workspace=join(settings,'workspace');for(const p of [settings,exportsDir,workspace])await mkdir(p);
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
async function launch(){assert.equal(await fetch('http://127.0.0.1:9351/json/version').then(()=>true).catch(()=>false),false);app=spawn(join(root,'src-tauri/target/debug/researchclip.exe'),[],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe'],env:{...process.env,RESEARCHCLIP_TEST_SETTINGS_DIR:settings,RESEARCHCLIP_TEST_CAPTURE_FIXTURE:'1',WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9351 --remote-debugging-address=127.0.0.1'}});app.stderr.on('data',d=>{for(const line of d.toString().trim().split('\n'))if(line)report.errors.push(line);});await until(()=>fetch('http://127.0.0.1:9351/json/version').then(r=>r.ok).catch(()=>false),'native startup');browser=await chromium.connectOverCDP('http://127.0.0.1:9351');page=await until(()=>browser.contexts().flatMap(c=>c.pages()).find(p=>!p.url().includes('captureOverlay')),'main');page.on('pageerror',e=>report.errors.push(e.message));await page.getByRole('button',{name:'新建项目',exact:true}).waitFor();await until(async()=>(await call('library_status')).schemaVersion===10,'schema10');}
async function stop(){input('close',app.pid);await until(()=>app.exitCode!==null,'exit');assert.equal(app.exitCode,0);await browser.close();browser=undefined;await until(()=>fetch('http://127.0.0.1:9351/json/version').then(()=>false).catch(()=>true),'WebView cleanup');}
try{
 await launch();const file=join(exportsDir,'OpenWith-test.md'),text='# Local math\n\n$V(x)=x^TPx$\n';await writeFile(file,text);
 await page.getByRole('button',{name:'打开 Markdown',exact:true}).click();await until(()=>{input('dialog-open-md',app.pid,file);return true;},'owned open dialog');await page.getByRole('textbox',{name:'Markdown 源文',exact:true}).waitFor();
 const processes=()=>JSON.parse(execFileSync('powershell',['-NoProfile','-Command',"ConvertTo-Json -InputObject @(Get-CimInstance Win32_Process | Select-Object ProcessId,Name,ExecutablePath) -Compress"],{windowsHide:true,encoding:'utf8'}));const old=processes().map(p=>p.ProcessId);await page.getByRole('button',{name:'使用其他应用打开',exact:true}).click();
 const chooser=await until(()=>processes().find(p=>!old.includes(p.ProcessId)&&p.Name==='OpenWith.exe'&&p.ExecutablePath?.toLowerCase()===join(process.env.SystemRoot,'System32','OpenWith.exe').toLowerCase()),'new verified Windows OpenWith host');
 const probe=join(root,'src-tauri/target/debug/examples/markdown_open_with_probe.exe');
 const inspect=()=>JSON.parse(execFileSync(probe,['inspect',String(chooser.ProcessId)],{windowsHide:true,encoding:'utf8'}));
 const popups=await until(()=>{const rows=inspect();return rows.length===1?rows:null;},'Windows Open With popup in verified host');
 assert.equal(popups.length,1);report.checks.push('SHOpenWithDialog displays a real Windows chooser: a newly created System32/OpenWith.exe host has exactly one visible popup');
 execFileSync(probe,['cancel',String(chooser.ProcessId)],{windowsHide:true,encoding:'utf8'});await until(async()=>await page.getByRole('button',{name:'使用其他应用打开',exact:true}).isEnabled(),'cancel Open With');assert.equal(await readFile(file,'utf8'),text);assert.equal(await page.locator('[data-markdown-panel]').getByRole('alert').count(),0);
 report.checks.push('cancelling the actual native chooser returns control without changing the .md source or showing an error; no default association changed');await page.getByRole('button',{name:'关闭文档',exact:true}).click();await stop();report.result='PASS';
}catch(e){report.result='FAIL';report.failure=e.stack;process.exitCode=1;if(page&&!page.isClosed())await page.screenshot({path:join(output,'failure.png')}).catch(()=>{});}
finally{if(browser)await browser.close().catch(()=>{});if(app&&app.exitCode===null)app.kill();await writeFile(join(output,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));}
