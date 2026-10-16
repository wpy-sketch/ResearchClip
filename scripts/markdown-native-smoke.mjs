import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {mkdir,writeFile,readFile,readdir,rename,access,unlink} from 'node:fs/promises';
import {join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {setTimeout as delay} from 'node:timers/promises';
import {createHash} from 'node:crypto';
import {chromium} from 'playwright-core';
const root=dirname(dirname(fileURLToPath(import.meta.url))),id=process.argv[2]??Date.now().toString();assert.match(id,/^[a-z0-9-]{1,64}$/);
const output=join(root,'artifacts',`markdown-${id}`);await mkdir(output);const settings=join(output,'settings');await mkdir(settings);
await writeFile(join(settings,'settings.json'),JSON.stringify({schemaVersion:1,theme:'dark',viewMode:'grid',fontFamily:'system',fontSize:16,captureShortcut:'Ctrl+Alt+Shift+F9',fullscreenShortcut:'Ctrl+Alt+Shift+F10',saveDirectory:null}));
const destination=join(output,'中文 导出 [研究] # 100%');await mkdir(destination);
const report={mode:'actual Windows WebView2 and native folder picker, isolated SQLite, controlled PNG (not physical capture)',checks:[],errors:[],dialogControls:[]};
let app,browser,page;
const helper=join(root,'src-tauri/target/debug/examples/workflow_input.exe');
const native=(action,path)=>execFileSync(join(root,'src-tauri/target/debug/examples/markdown_dialog.exe'),[action,String(app.pid),...(path?[path]:[])],{cwd:root,windowsHide:true,encoding:'utf8',stdio:['ignore','pipe','pipe'],timeout:12000});
async function until(fn,name,timeout=20000){const start=Date.now();let last;while(Date.now()-start<timeout){try{const value=await fn();if(value)return value;}catch(e){last=e;}await delay(120);}throw new Error(`Timed out ${name}: ${last?.message??''}`);}
const call=(command,args)=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
async function choose(action='folder'){
  const inventory=await until(()=>{const controls=JSON.parse(native('inspect'));return controls.some(control=>control.id==='1')&&controls;},'fully initialized owned folder dialog inventory');report.dialogControls.push(inventory);
  if(process.argv.includes('--inspect-only')){native('cancel');throw new Error('Inspection-only run; no export attempted');}
  native(action,action==='folder'?destination:undefined);
}
async function exportSelected(){await page.getByRole('button',{name:'导出所选 Markdown',exact:true}).click();await choose();await until(async()=>/Markdown 导出完成/.test(await page.locator('[aria-label="Markdown 导出状态"]').textContent()),'completed export');}
async function verifyBundle(folder,count,original){
  const markdown=await readFile(join(folder,'notes.md'),'utf8'),references=Array.from(markdown.matchAll(/^!\[.*\]\((assets\/[a-z0-9.-]+\.png)\)$/gm),match=>match[1]);assert.equal(references.length,count);
  assert.ok(!markdown.includes(settings)&&!markdown.includes(destination)&&!markdown.includes('originals/')&&!markdown.includes('thumbnails/'));
  for(const relative of references){const bytes=await readFile(join(folder,...relative.split('/')));assert.equal(hash(bytes),hash(original));assert.equal(bytes.readUInt32BE(16),320);assert.equal(bytes.readUInt32BE(20),200);}
  await assert.rejects(access(join(folder,'EXPORT-INCOMPLETE.txt')));return markdown;
}
try{
  assert.equal(await fetch('http://127.0.0.1:9340/json/version').then(()=>true).catch(()=>false),false,'CDP port occupied');
  app=spawn(join(root,'src-tauri/target/debug/researchclip.exe'),[],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe'],env:{...process.env,RESEARCHCLIP_TEST_SETTINGS_DIR:settings,RESEARCHCLIP_TEST_CAPTURE_FIXTURE:'1',WEBVIEW2_USER_DATA_FOLDER:join(output,'profile'),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9340 --remote-debugging-address=127.0.0.1'}});
  app.stderr.on('data',chunk=>{for(const line of chunk.toString().trim().split('\n'))if(line)report.errors.push(line);});
  await until(()=>fetch('http://127.0.0.1:9340/json/version').then(r=>r.ok).catch(()=>false),'app launch');browser=await chromium.connectOverCDP('http://127.0.0.1:9340');page=await until(()=>browser.contexts().flatMap(c=>c.pages()).find(p=>!p.url().includes('captureOverlay')),'main');page.on('pageerror',e=>report.errors.push(e.message));
  await page.getByRole('button',{name:'存入素材库',exact:true}).waitFor();await call('library_status');
  const data=await page.evaluate(async()=>{
    const invoke=window.__TAURI_INTERNALS__.invoke,c=await invoke('current_capture'),png=await invoke('capture_result_png',{captureId:c.captureId}),items=[];
    for(const title of ['Apollonius [Circle] #100%','中文科研 <> : /?*','缺失原图']){const json=new TextEncoder().encode(JSON.stringify({title,document:{schemaVersion:1,image:{widthPx:c.widthPx,heightPx:c.heightPx},objects:[]}})),bytes=new ArrayBuffer(4+json.length+png.byteLength);new DataView(bytes).setUint32(0,json.length,true);new Uint8Array(bytes,4,json.length).set(json);new Uint8Array(bytes,4+json.length).set(new Uint8Array(png));items.push(await invoke('create_screenshot',bytes));}return {items,png:Array.from(new Uint8Array(png))};
  });const [a,b,c]=data.items,original=Buffer.from(data.png);
  await page.getByRole('button',{name:'刷新素材库',exact:true}).click();await page.getByRole('button',{name:`打开截图 ${a.title}`,exact:true}).click();await page.getByRole('textbox',{name:'Markdown 笔记正文'}).waitFor();
  const body='## 几何观察\n\n公式 $x^2+y^2=r^2$\n\n- 中文与 English\n- 保留 Markdown 格式';
  await page.getByRole('textbox',{name:'Markdown 笔记正文'}).fill(body);await page.getByRole('textbox',{name:'来源 URL'}).fill('https://example.org/paper?q=geometry');await page.getByRole('combobox',{name:'添加卡片标签'}).fill('几何方法');await page.getByRole('button',{name:'添加标签',exact:true}).click();
  await page.getByRole('button',{name:'导出 Markdown',exact:true}).click();await choose();await until(async()=>/Markdown 导出完成/.test(await page.locator('[aria-label="Markdown 导出状态"]').textContent()),'single-card export');
  const single=join(destination,'ResearchClip-Export'),singleText=await verifyBundle(single,1,original);assert.ok(singleText.includes(body)&&singleText.includes('#几何方法')&&singleText.includes('https://example.org/paper?q=geometry')&&singleText.includes('创建时间：'));
  report.checks.push('single-card UI flushes unsaved Markdown, source and tags before a real native folder selection; notes.md and byte-identical relative assets are created under a Unicode/special-character parent');
  const firstHash=hash(await readFile(join(single,'notes.md')));
  await page.getByRole('button',{name:'返回素材库',exact:true}).click();await page.getByRole('checkbox',{name:`选择导出 ${a.title}`,exact:true}).check();await page.getByRole('checkbox',{name:`选择导出 ${b.title}`,exact:true}).check();await exportSelected();
  const multiple=join(destination,'ResearchClip-Export-001');await verifyBundle(multiple,2,original);assert.equal(hash(await readFile(join(single,'notes.md'))),firstHash);report.checks.push('batch selection exports two cards into a fresh numbered directory; original export is unchanged and duplicate/special titles produce safe unique relative filenames');
  await page.getByRole('button',{name:'导出所选 Markdown',exact:true}).click();await choose('cancel');await until(async()=>/已取消/.test(await page.locator('[aria-label="Markdown 导出状态"]').textContent()),'cancel notification');assert.equal((await readdir(destination)).length,2);report.checks.push('cancelling the real directory picker writes no new files and shows cancellation');
  await unlink(join(settings,'workspace',c.originalImagePath));await page.getByRole('checkbox',{name:`选择导出 ${c.title}`,exact:true}).check();await exportSelected();
  const partial=join(destination,'ResearchClip-Export-002');await verifyBundle(partial,2,original);assert.ok((await readFile(join(partial,'export-report.txt'),'utf8')).includes(c.id));assert.ok((await page.locator('[aria-label="Markdown 导出状态"]').textContent()).includes('未导出 1 张'));assert.equal((await call('get_knowledge_card',{id:c.id})).screenshot.id,c.id);report.checks.push('a genuinely missing managed original yields explicit partial success, no dangling image link and a path-free export report; screenshot record remains');
  await page.getByRole('button',{name:'清空选择',exact:true}).click();await page.getByRole('checkbox',{name:`选择导出 ${c.title}`,exact:true}).check();await page.getByRole('button',{name:'导出所选 Markdown',exact:true}).click();await choose();await until(async()=>/均无法导出/.test(await page.locator('[aria-label="Markdown 导出状态"]').textContent()),'all-failed notification');assert.equal((await readdir(destination)).length,3);report.checks.push('all missing images fail visibly and publish no empty export');
  const moved=join(output,'移动后的导出');await rename(multiple,moved);await verifyBundle(moved,2,original);report.checks.push('moving the entire generated bundle keeps every Markdown image reference resolvable and byte-identical');
  const unchosen=join(output,'not-authorized-by-picker'),extra=call('export_knowledge_markdown',{ids:[a.id],directory:unchosen});await choose('cancel');assert.equal(await extra,null);await assert.rejects(access(unchosen));await assert.rejects(()=>call('export_knowledge_markdown',{ids:['../x']}));await assert.rejects(()=>call('plugin:dialog|open',{options:{directory:true}}));report.checks.push('extra frontend path arguments cannot bypass the owned picker; invalid IDs and generic frontend dialog access are denied');
  await page.screenshot({path:join(output,'export-result.png')});
  const exited=new Promise(resolve=>app.once('exit',resolve));execFileSync(helper,['close',String(app.pid)],{cwd:root,windowsHide:true,stdio:'pipe'});await Promise.race([exited,delay(8000).then(()=>{throw new Error('Owned app did not close');})]);assert.equal(app.exitCode,0);await browser.close();browser=undefined;assert.equal(report.errors.length,0);report.result='passed';
}catch(error){report.result='failed';report.failure=error.stack;process.exitCode=1;if(page&&!page.isClosed())await page.screenshot({path:join(output,'failure.png')}).catch(()=>{});}
finally{if(browser)await browser.close().catch(()=>{});if(app&&app.exitCode===null)app.kill();await writeFile(join(output,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify({...report,dialogControls:report.dialogControls.map(controls=>controls.filter(control=>['1','2','1152'].includes(control.id)))},null,2));}
