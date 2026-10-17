import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {mkdir,writeFile,readFile,readdir} from 'node:fs/promises';
import {join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {setTimeout as delay} from 'node:timers/promises';
import {createServer} from 'node:http';
import {chromium} from 'playwright-core';
const root=dirname(dirname(fileURLToPath(import.meta.url))),id=process.argv[2]??Date.now().toString();assert.match(id,/^[a-z0-9-]{1,64}$/);
const output=join(root,'artifacts',`pins-${id}`);await mkdir(output);const settings=join(output,'settings'),exportsDir=join(output,'exports');await mkdir(settings);await mkdir(exportsDir);
await writeFile(join(settings,'settings.json'),JSON.stringify({schemaVersion:1,theme:'dark',viewMode:'grid',fontFamily:'system',fontSize:16,captureShortcut:'Ctrl+Alt+Shift+F9',fullscreenShortcut:'Ctrl+Alt+Shift+F10',saveDirectory:exportsDir,closeToTray:true}));
const report={mode:'actual Windows native windows, Win32 styles/alpha/physical drag, system PNG dialog and separate Edge clipboard receiver; controlled PNG, NOT screen-capture verification',checks:[],errors:[]};
let app,browser,page,receiver,server;
const helper=join(root,'src-tauri/target/debug/examples/workflow_input.exe'),probe=join(root,'src-tauri/target/debug/examples/pin_probe.exe');
const input=(...args)=>execFileSync(helper,args.map(String),{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe'],encoding:'utf8'});
const pinProbe=(...args)=>execFileSync(probe,args.map(String),{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe'],encoding:'utf8'});
const inspect=()=>JSON.parse(pinProbe('inspect',app.pid));
const mainWindow=()=>JSON.parse(execFileSync(join(root,'src-tauri/target/debug/examples/desktop_probe.exe'),['inspect',String(app.pid)],{cwd:root,windowsHide:true,encoding:'utf8'}));
async function until(fn,name,timeout=25000){const start=Date.now();let last;while(Date.now()-start<timeout){try{const value=await fn();if(value)return value;}catch(e){last=e;}await delay(120);}throw new Error(`Timed out ${name}: ${last?.message??''}`);}
const status=()=>page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('pin_status'));
const pinPages=()=>browser.contexts().flatMap(c=>c.pages()).filter(p=>!p.isClosed()&&p.url().includes('desktopPin='));
async function waitPin(count){await until(()=>pinPages().length===count,'pin WebViews');await until(()=>inspect().filter(w=>w.visible).length===count,'native visible pins');return pinPages().at(-1);}
async function dialog(mode,path){await until(()=>{input(mode,app.pid,...(path?[path]:[]));return true;},'owned pin Save dialog');}
const menu=async p=>{await p.getByRole('img',{name:'桌面贴图',exact:true}).dispatchEvent('contextmenu');await p.getByRole('menu',{name:'贴图菜单'}).waitFor();};
try{
  assert.equal(await fetch('http://127.0.0.1:9345/json/version').then(()=>true).catch(()=>false),false,'CDP port occupied');
  app=spawn(join(root,'src-tauri/target/debug/researchclip.exe'),[],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe'],env:{...process.env,RESEARCHCLIP_TEST_SETTINGS_DIR:settings,RESEARCHCLIP_TEST_CAPTURE_FIXTURE:'1',WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9345 --remote-debugging-address=127.0.0.1'}});
  app.stderr.on('data',data=>{for(const line of data.toString().trim().split('\n'))if(line)report.errors.push(line);});
  await until(()=>fetch('http://127.0.0.1:9345/json/version').then(r=>r.ok).catch(()=>false),'native app');browser=await chromium.connectOverCDP('http://127.0.0.1:9345');
  page=await until(()=>browser.contexts().flatMap(c=>c.pages()).find(p=>!p.url().includes('desktopPin')&&!p.url().includes('captureOverlay')),'main');page.on('pageerror',e=>report.errors.push(e.message));
  await page.getByRole('button',{name:'贴图',exact:true}).waitFor();
  const original=await page.evaluate(async()=>{const invoke=window.__TAURI_INTERNALS__.invoke,info=await invoke('current_capture');return Array.from(new Uint8Array(await invoke('capture_result_png',{captureId:info.captureId})));});
  assert.deepEqual(await status(),{count:0,retainedBytes:0});
  await page.getByRole('button',{name:'贴图',exact:true}).click();const first=await waitPin(1);first.on('pageerror',e=>report.errors.push(e.message));
  const firstLabel=inspect()[0].label;
  assert.equal(await first.getByRole('img',{name:'桌面贴图'}).evaluate(el=>el.naturalWidth),320);assert.equal((await status()).count,1);
  await page.getByRole('button',{name:'贴图',exact:true}).click();const second=await waitPin(2);assert.equal((await status()).count,2);assert.equal(new Set(inspect().map(w=>w.label)).size,2);
  report.initialWindows=inspect();
  for(const w of inspect()){assert.equal(w.clientFillsWindow,true,'native client area must cover whole window: no nonclient title bar');assert.equal(w.topmost,true);assert.equal(w.visible,true);assert.equal(w.clickThrough,false);}
  report.checks.push('capture completion creates 2 independent visible frameless native HWNDs, each topmost, with unique labels; main remains alive');
  const denied=await first.evaluate(async()=>{const failures=[];for(const command of ['create_pin','pin_status','current_capture','recover_pins']){try{await window.__TAURI_INTERNALS__.invoke(command);failures.push(command);}catch{/* expected capability rejection */}}return failures;});assert.deepEqual(denied,[]);
  await assert.rejects(()=>page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('pin_image')));report.checks.push('actual capability enforcement prevents pin from reading main capture/creating windows and main from reading arbitrary pin resources');
  await assert.rejects(()=>page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('create_pin',new Uint8Array([1,2,3]).buffer)));assert.equal((await status()).count,2);
  for(const [action,value] of [['opacity',0],['opacity',101],['zoom',0],['zoom',3]])await assert.rejects(()=>first.evaluate(({action,value})=>window.__TAURI_INTERNALS__.invoke('pin_action',{action,value}),{action,value}));
  report.checks.push('actual Rust IPC rejects malformed PNG, invalid opacity and out-of-range zoom without losing existing pins');

  await second.getByRole('button',{name:'关闭贴图',exact:true}).click();await waitPin(1);
  const before=inspect().find(w=>w.label===firstLabel);
  await first.getByRole('img',{name:'桌面贴图'}).dispatchEvent('wheel',{deltaY:-100});await until(()=>inspect()[0].width>before.width,'native wheel enlargement');
  let dimensions=inspect()[0];assert.ok(Math.abs(dimensions.width/dimensions.height-1.6)<0.01);
  for(let i=0;i<20;i++)await first.evaluate(()=>window.__TAURI_INTERNALS__.invoke('pin_action',{action:'zoom',value:0.5}));dimensions=inspect()[0];assert.equal(dimensions.width,48);assert.equal(dimensions.height,30);
  for(let i=0;i<20;i++)await first.evaluate(()=>window.__TAURI_INTERNALS__.invoke('pin_action',{action:'zoom',value:2}));dimensions=inspect()[0];assert.ok(dimensions.width<=1280&&dimensions.height<=800);
  await first.evaluate(()=>window.__TAURI_INTERNALS__.invoke('pin_action',{action:'recover',value:null}));
  report.checks.push('actual wheel resizes native window with aspect ratio; repeated zoom reaches bounded minimum and maximum');
  const dragStart=inspect()[0];pinProbe('drag',app.pid,firstLabel);await until(()=>{const w=inspect()[0];return Math.abs(w.left-dragStart.left)>50&&Math.abs(w.top-dragStart.top)>40;},'physical native mouse drag');report.checks.push('real SendInput drag on test-owned foreground pin moves the actual HWND');
  await menu(first);const opacity=first.getByRole('slider',{name:'贴图不透明度'});await opacity.focus();await opacity.press('Home');await until(()=>inspect()[0].alpha===51,'native 20% window alpha');assert.equal(inspect()[0].layered,true);assert.equal(inspect()[0].clickThrough,false);await opacity.press('End');await until(()=>inspect()[0].alpha===255,'native full opacity');
  report.checks.push('right-click opacity slider changes actual Win32 layered-window alpha 51/255 and never enables click-through');
  await first.getByRole('menuitem',{name:'另存为 PNG',exact:true}).click();const saved=join(exportsDir,'贴图 原始.png');await dialog('dialog-confirm',saved);await until(()=>first.getByRole('menuitem',{name:'复制图片'}).isEnabled(),'save complete');assert.deepEqual(Array.from(await readFile(saved)),original);
  await first.getByRole('menuitem',{name:'另存为 PNG',exact:true}).click();await dialog('dialog-cancel');await until(()=>first.getByRole('menuitem',{name:'复制图片'}).isEnabled(),'cancel save');assert.equal((await readdir(exportsDir)).length,1);
  report.checks.push('real native Save As writes byte-identical original PNG; dialog cancellation creates no extra files');
  await first.getByRole('menuitem',{name:'复制图片',exact:true}).click();await first.getByRole('status').filter({hasText:'图片已复制'}).waitFor();
  server=createServer((req,res)=>{res.setHeader('Content-Type','text/html');res.end('<button id="paste">Paste pin</button><script>paste.onclick=async()=>{try{const items=await navigator.clipboard.read(),item=items.find(i=>i.types.includes("image/png")),blob=await item.getType("image/png"),bmp=await createImageBitmap(blob),c=new OffscreenCanvas(bmp.width,bmp.height),ctx=c.getContext("2d");ctx.drawImage(bmp,0,0);bmp.close();const a=ctx.getImageData(0,0,c.width,c.height).data;let exact=true;for(let y=0;y<c.height;y++)for(let x=0;x<c.width;x++){const i=(y*c.width+x)*4;if(a[i]!==x%256||a[i+1]!==y%256||a[i+2]!==180||a[i+3]!==255)exact=false;}window.result={width:c.width,height:c.height,exact};}catch(e){window.result={error:e.message};}}</script>');});await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));receiver=await chromium.launch({channel:'msedge',headless:false});const view=await receiver.newPage();await view.goto(`http://127.0.0.1:${server.address().port}`);await view.context().grantPermissions(['clipboard-read'],{origin:view.url()});await view.getByRole('button',{name:'Paste pin'}).click();assert.deepEqual(await until(()=>view.evaluate(()=>window.result),'actual clipboard image'),{width:320,height:200,exact:true});await receiver.close();receiver=undefined;
  report.checks.push('separate Edge application reads real Windows clipboard PNG; all controlled-image RGBA pixels match');
  const blocker=spawn(helper,['lock-clipboard'],{cwd:root,windowsHide:true,stdio:['pipe','pipe','pipe']});
  try{await new Promise((resolve,reject)=>{blocker.stdout.once('data',data=>data.toString().includes('CLIPBOARD_LOCKED')?resolve():reject(new Error('clipboard lock not established')));blocker.once('exit',code=>reject(new Error(`clipboard lock helper exited ${code}`)));});await first.getByRole('menuitem',{name:'复制图片',exact:true}).click();await first.getByRole('alert').filter({hasText:'剪贴板不可用'}).waitFor();assert.equal((await status()).count,1);}finally{blocker.stdin.end('\n');await until(()=>blocker.exitCode!==null,'clipboard lock released');}
  await first.getByRole('menuitem',{name:'复制图片',exact:true}).click();await first.getByRole('status').filter({hasText:'图片已复制'}).waitFor();
  report.checks.push('real clipboard lock produces a readable failure while retaining the pin; releasing lock permits retry');

  pinProbe('offscreen',app.pid,firstLabel);assert.ok(inspect()[0].left < -10000);await page.getByRole('button',{name:'恢复贴图位置'}).click();await until(()=>inspect()[0].left>=0,'recover all');
  await first.getByRole('menuitem',{name:'收起菜单'}).click();await first.screenshot({path:join(output,'pin.png')});
  input('close',app.pid);await until(()=>!mainWindow().visible,'main actually hidden to tray');await until(()=>page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('desktop_status')).then(s=>s.trayAvailable),'tray');assert.equal(app.exitCode,null);assert.equal(inspect()[0].visible,true);await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('desktop_action',{action:'open'}));await until(()=>mainWindow().visible,'main restored');
  report.checks.push('off-screen native position recovers to main monitor; closing main to tray retains the pin and reopening main retains both states');
  await first.getByRole('button',{name:'关闭贴图',exact:true}).click();await waitPin(0);assert.deepEqual(await status(),{count:0,retainedBytes:0});
  for(let i=0;i<10;i++){await page.getByRole('button',{name:'贴图',exact:true}).click();const p=await waitPin(1);await p.getByRole('button',{name:'关闭贴图',exact:true}).click();await waitPin(0);assert.deepEqual(await status(),{count:0,retainedBytes:0});}
  report.checks.push('10 real open/close cycles destroy each HWND/WebView; managed PNG/decoded-image budget returns to zero every cycle (not an OS heap leak proof)');
  await page.getByRole('button',{name:'打开编辑器',exact:true}).click();const canvas=page.getByRole('application',{name:'图片编辑画布'});await canvas.waitFor();await delay(300);const b=await canvas.boundingBox(),fit=Math.min(1,(b.width-48)/320,(b.height-48)/200),at=(x,y)=>({x:b.x+(b.width-320*fit)/2+x*fit,y:b.y+(b.height-200*fit)/2+y*fit});
  await page.getByRole('button',{name:'矩形工具',exact:true}).click();const a=at(40,30),z=at(140,100);await page.mouse.move(a.x,a.y);await page.mouse.down();await page.mouse.move(z.x,z.y,{steps:10});await page.mouse.up();await page.getByText('标注 1',{exact:true}).waitFor();
  await page.getByRole('button',{name:'贴图',exact:true}).click();const composite=await waitPin(1);const rendered=await composite.getByRole('img',{name:'桌面贴图'}).evaluate(el=>{const c=new OffscreenCanvas(el.naturalWidth,el.naturalHeight),ctx=c.getContext('2d');ctx.drawImage(el,0,0);const color=Array.from(ctx.getImageData(40,50,1,1).data),background=Array.from(ctx.getImageData(5,5,1,1).data);return {width:c.width,height:c.height,color,background};});assert.equal(rendered.width,320);assert.equal(rendered.height,200);assert.notDeepEqual(rendered.color,[40,50,180,255]);assert.deepEqual(rendered.background,[5,5,180,255]);
  const current=await page.evaluate(async()=>{const i=await window.__TAURI_INTERNALS__.invoke('current_capture');return Array.from(new Uint8Array(await window.__TAURI_INTERNALS__.invoke('capture_result_png',{captureId:i.captureId})));});assert.deepEqual(current,original);
  await page.evaluate(async()=>{const i=await window.__TAURI_INTERNALS__.invoke('current_capture');await window.__TAURI_INTERNALS__.invoke('discard_capture',{captureId:i.captureId});});assert.equal(await composite.getByRole('img',{name:'桌面贴图'}).evaluate(el=>el.complete&&el.naturalWidth===320),true);
  report.checks.push('editor button pins original-resolution composite containing the rectangle; background unchanged and original screenshot bytes immutable; discarding capture leaves independent pin alive');
  await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('desktop_action',{action:'exit'}));await page.getByRole('dialog').filter({hasText:'未保存'}).waitFor();await page.getByRole('button',{name:'返回保存',exact:true}).click();assert.equal((await status()).count,1);assert.equal(inspect().length,1);await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('desktop_action',{action:'exit'}));await page.getByRole('dialog').filter({hasText:'未保存'}).waitFor();const quit=page.getByRole('button',{name:/放弃.*退出/});await quit.click();await until(()=>app.exitCode!==null,'graceful true exit');assert.equal(app.exitCode,0);await browser.close();browser=undefined;assert.equal(inspect().length,0);
  report.checks.push('explicit exit confirms dirty editor, closes all pin HWNDs, and terminates main normally');
  assert.equal(report.errors.length,0);report.result='PASS';
}catch(error){report.result='FAIL';report.failure=error.stack;process.exitCode=1;if(page&&!page.isClosed())await page.screenshot({path:join(output,'failure.png')}).catch(()=>{});}
finally{if(receiver)await receiver.close().catch(()=>{});if(browser)await browser.close().catch(()=>{});if(app&&app.exitCode===null)app.kill();if(server)server.close();await writeFile(join(output,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));}
