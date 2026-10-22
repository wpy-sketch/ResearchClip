import assert from 'node:assert/strict';
import {spawn, execFileSync} from 'node:child_process';
import {mkdir, writeFile, readFile, access} from 'node:fs/promises';
import {join, dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {setTimeout as delay} from 'node:timers/promises';
import {createHash} from 'node:crypto';
import {chromium} from 'playwright-core';

const root=dirname(dirname(fileURLToPath(import.meta.url))), id=process.argv[2]??Date.now().toString();
assert.match(id,/^[a-z0-9-]{1,64}$/);
const output=join(root,'artifacts',`library-${id}`); await mkdir(output);
const settingsDir=join(output,'settings'); await mkdir(settingsDir);
const workspace=join(settingsDir,'workspace');
await writeFile(join(settingsDir,'settings.json'),JSON.stringify({schemaVersion:1,theme:'dark',viewMode:'grid',fontFamily:'system',fontSize:16,captureShortcut:'Ctrl+Shift+2',fullscreenShortcut:'Ctrl+Shift+1',saveDirectory:null}));
const report={mode:'actual Windows WebView2 + SQLite + managed PNG files, controlled source PNG; no mocked persistence or real screen capture',checks:[],errors:[]};
let app,browser,page;
const imageRequests=[];
const helper=join(root,'src-tauri/target/debug/examples/workflow_input.exe');
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
async function until(fn,name,timeout=15000){const start=Date.now();let last;while(Date.now()-start<timeout){try{const value=await fn();if(value)return value;}catch(e){last=e;}await delay(80);}throw new Error(`Timed out ${name}: ${last?.message??''}`);}
const call=(command,args)=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const readPage=(n=0,sort='createdDesc',favorites=false)=>call('screenshot_page',{page:n,limit:24,sort,favorites});
const cards=()=>page.locator('[data-screenshot-id]');
const cardIds=()=>cards().evaluateAll(nodes=>nodes.map(n=>n.dataset.screenshotId));
const waitPage=async(n,total)=>{
  const expected=await readPage(n,await page.getByRole('combobox',{name:'素材排序'}).inputValue());
  assert.equal(expected.total,total);
  await until(async()=>JSON.stringify(await cardIds())===JSON.stringify(expected.items.map(i=>i.id)),`rendered page ${n}`);
  if(total) await page.getByRole('navigation',{name:'素材分页'}).getByText(`第 ${expected.page+1} / ${Math.ceil(total/24)} 页`,{exact:false}).waitFor();
  return expected;
};
async function launch(){
  assert.equal(await fetch('http://127.0.0.1:9338/json/version').then(()=>true).catch(()=>false),false,'CDP port occupied');
  app=spawn(join(root,'src-tauri/target/debug/researchclip.exe'),[],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe'],env:{...process.env,RESEARCHCLIP_TEST_SETTINGS_DIR:settingsDir,RESEARCHCLIP_TEST_CAPTURE_FIXTURE:'1',WEBVIEW2_USER_DATA_FOLDER:join(output,'profile'),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9338 --remote-debugging-address=127.0.0.1'}});
  app.stderr.on('data',c=>{for(const line of c.toString().trim().split('\n'))if(line)report.errors.push(line);});
  await until(()=>fetch('http://127.0.0.1:9338/json/version').then(r=>r.ok).catch(()=>false),'native launch');
  browser=await chromium.connectOverCDP('http://127.0.0.1:9338');
  page=await until(()=>browser.contexts().flatMap(c=>c.pages()).find(p=>!p.url().includes('captureOverlay')),'main');
  page.on('pageerror',e=>report.errors.push(e.message));
  page.on('request',request=>{
    const url=new URL(request.url());
    if(url.hostname==='ipc.localhost'&&decodeURIComponent(url.pathname.slice(1))==='screenshot_image')imageRequests.push(request.postDataJSON());
  });
  await page.getByRole('button',{name:'存入素材库',exact:true}).waitFor();
  await until(async()=> (await call('library_status')).schemaVersion===11,'real SQLite');
}
async function close(){
  const exited=new Promise(resolve=>app.once('exit',resolve));
  execFileSync(helper,['close',String(app.pid)],{cwd:root,windowsHide:true,stdio:'pipe'});
  await Promise.race([exited,delay(8000).then(()=>{throw new Error('Owned app did not close');})]);
  assert.equal(app.exitCode,0);await browser.close();browser=undefined;
  await until(()=>fetch('http://127.0.0.1:9338/json/version').then(()=>false).catch(()=>true),'native shutdown');
}
try {
  await launch();assert.equal((await readPage()).total,0);
  await page.getByRole('heading',{name:'你的第一张素材，从这里开始',exact:true}).waitFor();
  report.checks.push('empty SQLite library renders guidance without inserting demonstration records');
  const records=await page.evaluate(async()=>{
    const invoke=window.__TAURI_INTERNALS__.invoke,c=await invoke('current_capture');
    const png=await invoke('capture_result_png',{captureId:c.captureId}),records=[];
    // Explicit acceptance records go through the real production creation command in a fresh isolated workspace.
    for(let n=1;n<=49;n++){
      const json=new TextEncoder().encode(JSON.stringify({title:`研究素材 ${String(n).padStart(2,'0')}`,document:{schemaVersion:1,image:{widthPx:c.widthPx,heightPx:c.heightPx},objects:[]}}));
      const bytes=new ArrayBuffer(4+json.length+png.byteLength);new DataView(bytes).setUint32(0,json.length,true);
      new Uint8Array(bytes,4,json.length).set(json);new Uint8Array(bytes,4+json.length).set(new Uint8Array(png));
      records.push(await invoke('create_screenshot',bytes));
    }
    return records;
  });
  assert.equal((await readFile(join(workspace,'researchclip.sqlite3'))).subarray(0,16).toString(),'SQLite format 3\0');
  await page.getByRole('button',{name:'刷新素材库',exact:true}).click();
  const first=await waitPage(0,49);
  await until(async()=>await cards().locator('img').count()===24,'24 thumbnail resources');
  // loading=lazy defers off-screen decode; exercise visibility instead of depending on Chromium's preload margin.
  for(let n=0;n<24;n++)await cards().nth(n).scrollIntoViewIfNeeded();
  await until(async()=>await cards().locator('img').evaluateAll(images=>images.length===24&&images.every(i=>i.complete&&i.naturalWidth>0)),'24 decoded thumbnails');
  assert.equal(imageRequests.filter(r=>!r.thumbnail).length,0);
  assert.equal(imageRequests.filter(r=>r.thumbnail).length,24);
  const immutableHash=hash(await readFile(join(workspace,first.items[0].originalImagePath)));
  report.checks.push('49 actual SQLite records; bounded 24-card grid; thumbnails requested without loading originals or annotation documents');
  const visited=[...await cardIds()];await page.getByRole('button',{name:'下一页',exact:true}).click();await waitPage(1,49);visited.push(...await cardIds());
  await page.getByRole('button',{name:'下一页',exact:true}).click();const final=await waitPage(2,49);visited.push(...await cardIds());
  assert.equal(new Set(visited).size,49);assert.deepEqual(new Set(visited),new Set(records.map(r=>r.id)));
  assert.equal(await page.getByRole('button',{name:'下一页',exact:true}).isDisabled(),true);
  await page.getByRole('combobox',{name:'素材排序'}).selectOption('createdAsc');const ascending=await waitPage(0,49);
  assert.equal(ascending.items[0].id,final.items[0].id);
  await page.getByRole('combobox',{name:'素材排序'}).selectOption('createdDesc');await waitPage(0,49);
  report.checks.push('creation-time ascending/descending sorting; three pages cover all 49 records without duplicates; sort resets to page one');
  const target=first.items[0],name='研究：细胞图像 A';
  await page.getByRole('button',{name:`修改标题 ${target.title}`,exact:true}).click();
  await page.getByRole('textbox',{name:'素材标题'}).fill(name);await page.getByRole('button',{name:'保存标题',exact:true}).click();
  await page.getByRole('button',{name:`收藏 ${name}`,exact:true}).waitFor();assert.deepEqual(await cardIds(),first.items.map(i=>i.id));
  await page.getByRole('button',{name:`收藏 ${name}`,exact:true}).click();await page.getByRole('button',{name:`取消收藏 ${name}`,exact:true}).waitFor();
  await page.getByRole('button',{name:'收藏',exact:true}).click();await until(async()=>await cards().count()===1,'favorite filter');
  assert.deepEqual(await cardIds(),[target.id]);await page.getByRole('button',{name:`取消收藏 ${name}`,exact:true}).click();
  await page.getByRole('heading',{name:'为重要的图像留个位置',exact:true}).waitFor();assert.equal((await readPage(0,'createdDesc',true)).total,0);
  await page.getByRole('button',{name:'截图素材',exact:true}).click();await waitPage(0,49);
  await page.getByRole('button',{name:`收藏 ${name}`,exact:true}).click();await page.getByRole('button',{name:`取消收藏 ${name}`,exact:true}).waitFor();
  report.checks.push('rename/favorite/unfavorite use real revisions, preserve creation ordering, and refresh the favorites empty state correctly');
  await page.getByRole('button',{name:`打开截图 ${name}`,exact:true}).click();
  await page.getByRole('textbox',{name:'Markdown 笔记正文',exact:true}).waitFor();
  await page.getByRole('img',{name:`${name}图片预览`,exact:true}).waitFor();
  assert.equal(imageRequests.filter(r=>!r.thumbnail).length,1);
  await page.getByRole('button',{name:'编辑图片标注',exact:true}).click();
  await page.getByRole('application',{name:'图片编辑画布'}).waitFor();await page.getByText('标注 0',{exact:true}).waitFor();
  assert.equal(imageRequests.filter(r=>!r.thumbnail).length,2);
  await page.getByRole('button',{name:'返回截图',exact:true}).click();await waitPage(0,49);
  assert.equal(hash(await readFile(join(workspace,target.originalImagePath))),immutableHash);
  report.checks.push('single click opens the knowledge card with one original request; explicit image-edit opens existing editor, source PNG stays unchanged');
  const thumbnailPath=join(workspace,target.thumbnailPath),thumbnail=await readFile(thumbnailPath);
  await writeFile(thumbnailPath,'corrupted acceptance-only PNG');
  // Remount the card list to force an actual native image read.
  await page.getByRole('combobox',{name:'素材排序'}).selectOption('createdAsc');await waitPage(0,49);
  await page.getByRole('combobox',{name:'素材排序'}).selectOption('createdDesc');await waitPage(0,49);
  await page.getByRole('button',{name:`重试缩略图 ${name}`,exact:true}).waitFor();assert.equal(await cards().count(),24);
  await until(async()=>await cards().locator('img').count()===23,'other thumbnails still load');
  assert.equal((await readPage()).total,49);await writeFile(thumbnailPath,thumbnail);
  await page.getByRole('button',{name:`重试缩略图 ${name}`,exact:true}).click();
  await page.getByRole('img',{name:`${name}缩略图`,exact:true}).waitFor();
  report.checks.push('one deliberately corrupt test thumbnail displays a per-card placeholder without crashing other cards; restoring its file and retrying works');
  await page.getByRole('button',{name:'列表视图',exact:true}).click();await until(()=>page.getByRole('region',{name:'截图素材库',exact:true}).getAttribute('data-view-mode').then(v=>v==='list'),'list layout');
  await page.getByRole('button',{name:'切换为浅色主题',exact:true}).click();await page.getByRole('button',{name:'切换为深色主题',exact:true}).waitFor();
  await cards().first().scrollIntoViewIfNeeded();await page.screenshot({path:join(output,'library-light-list.png')});
  await page.getByRole('button',{name:'网格视图',exact:true}).click();await page.getByRole('button',{name:'切换为深色主题',exact:true}).click();
  await page.getByRole('button',{name:'切换为浅色主题',exact:true}).waitFor();await cards().first().scrollIntoViewIfNeeded();await page.screenshot({path:join(output,'library-dark-grid.png')});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  report.checks.push('real native light/list and dark/grid views render the same records with no horizontal document overflow');
  const originalViewport=await page.evaluate(()=>({width:innerWidth,height:innerHeight}));
  let layouts=0;
  for(const theme of ['dark','light']){
    if(await page.getByRole('button',{name:theme==='dark'?'切换为深色主题':'切换为浅色主题',exact:true}).count())await page.getByRole('button',{name:theme==='dark'?'切换为深色主题':'切换为浅色主题',exact:true}).click();
    for(const mode of ['网格视图','列表视图']){
      await page.getByRole('button',{name:mode,exact:true}).click();
      for(const [width,height] of [[480,400],[640,500],[960,680],[1440,900]]){
        await page.setViewportSize({width,height});await cards().first().scrollIntoViewIfNeeded();
        await until(async()=>await page.evaluate(()=>innerWidth)===width,'emulated WebView viewport');
        assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,`${theme}/${mode}/${width}: document overflow`);
        const box=await cards().first().boundingBox();assert.ok(box.width>=200&&box.x>=0&&box.x+box.width<=width+1);
        assert.equal(await cards().count(),24);layouts++;
      }
    }
  }
  await page.setViewportSize(originalViewport);await page.getByRole('button',{name:'网格视图',exact:true}).click();
  await page.getByRole('button',{name:'切换为深色主题',exact:true}).click();
  report.checks.push(`${layouts} non-empty theme/grid/list layouts at 480x400, 640x500, 960x680, 1440x900 (CDP viewport emulation inside native WebView; not a hardware DPI test)`);
  await page.getByRole('button',{name:'下一页',exact:true}).click();await waitPage(1,49);
  await page.getByRole('button',{name:'下一页',exact:true}).click();const last=await waitPage(2,49),deleted=last.items[0];
  await page.getByRole('button',{name:`删除截图 ${deleted.title}`,exact:true}).click();await page.getByRole('button',{name:'保留截图',exact:true}).click();assert.equal((await readPage()).total,49);
  await page.getByRole('button',{name:`删除截图 ${deleted.title}`,exact:true}).click();await page.getByRole('button',{name:'确认删除截图',exact:true}).click();await waitPage(1,48);
  await assert.rejects(access(join(workspace,deleted.originalImagePath)));await assert.rejects(access(join(workspace,deleted.thumbnailPath)));
  assert.equal(hash(await readFile(join(workspace,target.originalImagePath))),immutableHash);
  report.checks.push('delete cancel keeps row/files; confirmed final-page deletion cleans only its files and automatically clamps to the preceding page');
  await close();await launch();await waitPage(0,48);
  const persisted=await call('get_screenshot',{id:target.id});assert.equal(persisted.title,name);assert.equal(persisted.isFavorite,true);
  await assert.rejects(()=>call('get_screenshot',{id:deleted.id}));assert.equal(hash(await readFile(join(workspace,target.originalImagePath))),immutableHash);
  await close();report.checks.push('actual close/restart preserves count, rename, favorite, deletion and immutable original');
  assert.equal(report.errors.length,0);report.result='passed';
} catch(error) {
  report.result='failed';report.failure=error.stack;process.exitCode=1;
  if(page&&!page.isClosed())await page.screenshot({path:join(output,'failure.png')}).catch(()=>{});
} finally {
  if(browser)await browser.close().catch(()=>{});if(app&&app.exitCode===null)app.kill();
  await writeFile(join(output,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
}
