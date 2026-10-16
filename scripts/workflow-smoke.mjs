import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile, readdir } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium } from 'playwright-core';
import { createServer } from 'node:http';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const id = process.argv[2] ?? new Date().toISOString().replaceAll(/[^0-9]/g, '');
assert.match(id, /^[a-z0-9-]{1,64}$/);
const output = join(root, 'artifacts', `workflow-${id}`);
await mkdir(output); // Never replace an earlier report or profile.
const settingsDir = join(output, 'settings'); const exportsDir = join(output, 'exports');
await mkdir(settingsDir); await mkdir(exportsDir);
const settingsPath = join(settingsDir, 'settings.json');
await writeFile(settingsPath, JSON.stringify({ schemaVersion: 1, theme: 'dark', viewMode: 'grid', fontFamily: 'system', fontSize: 16, captureShortcut: 'Ctrl+Shift+2', saveDirectory: exportsDir }));
const helper = join(root, 'src-tauri/target/debug/examples/workflow_input.exe');
const messageTrigger = process.argv.includes('--message-trigger');
const fixtureMode = process.argv.includes('--fixture');
const report = { mode: fixtureMode?'native-plugins-controlled-image-fixture':'native-windows-workflow', nativeCaptureVerified:false, shortcutTrigger: fixtureMode?'OS registration/conflict only':messageTrigger ? 'native WM_HOTKEY integration; physical keyboard unverified' : 'Windows SendInput keyboard', checks: [], errors: [] };
let app; let browser; let receiver; let holder; let clipboardLock; let page; let fixtureServer; let frontendServer;
const debugPort = 9334;
async function until(action, name, timeout = 15000) {
  const start = Date.now(); let last;
  while (Date.now() - start < timeout) { try { const value = await action(); if (value) return value; } catch (e) { last = e; } await delay(150); }
  throw new Error(`Timed out: ${name}${last ? ` (${last.message})` : ''}`);
}
function input(...args) { return execFileSync(helper, args.map(String), { cwd: root, windowsHide: true, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }); }
function ownedHelper(...args) { return spawn(helper, args, { cwd: root, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] }); }
async function stopChild(child) { if (child && child.exitCode === null) { const done = new Promise((resolve) => child.once('exit', resolve)); child.kill(); await Promise.race([done, delay(5000)]); } }
async function launch() {
  assert.equal(await fetch(`http://127.0.0.1:${debugPort}/json/version`).then(() => true).catch(() => false), false, 'Debug port is already in use');
  app = spawn(join(root, 'src-tauri/target/debug/researchclip.exe'), [], { cwd: root, windowsHide: true, stdio: ['ignore','pipe','pipe'], env: { ...process.env, RESEARCHCLIP_TEST_SETTINGS_DIR: settingsDir, ...(fixtureMode?{RESEARCHCLIP_TEST_CAPTURE_FIXTURE: '1'}:{}), WEBVIEW2_USER_DATA_FOLDER: join(output, 'webview-profile'), WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort} --remote-debugging-address=127.0.0.1` } });
  app.stderr.on('data', (chunk) => { for (const message of chunk.toString().trim().split('\n')) { if (!message) continue; if (message.startsWith('ResearchClip debug:')) { (report.debug ??= []).push(message); } else report.errors.push(message); } });
  await until(() => fetch(`http://127.0.0.1:${debugPort}/json/version`).then((r) => r.ok).catch(() => false), 'native WebView2');
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${debugPort}`);
  page = await until(() => browser.contexts().flatMap((c) => c.pages()).find((p) => !p.url().includes('captureOverlay')), 'main page');
  page.on('pageerror', (e) => report.errors.push(e.message));
  await page.getByRole('button', { name: '设置', exact: true }).waitFor();
  await page.evaluate(async () => {
    window.workflowEvents = [];
    const handler = window.__TAURI_INTERNALS__.transformCallback((event) => { window.workflowEvents.push(event.payload); });
    await window.__TAURI_INTERNALS__.invoke('plugin:event|listen', { event: 'capture-workflow', target: { kind: 'Any' }, handler });
  });
  const expectedTheme = JSON.parse(await readFile(settingsPath, 'utf8')).theme;
  await until(() => page.evaluate((theme) => document.documentElement.dataset.theme === theme, expectedTheme), 'appearance initialization');
}
async function prefs() { return page.evaluate(() => window.__TAURI_INTERNALS__.invoke('get_preferences')); }
async function current() { return page.evaluate(() => window.__TAURI_INTERNALS__.invoke('current_capture')); }
async function pngHash(locator) {
  return locator.evaluate(async (img) => {
    await img.decode(); const canvas = new OffscreenCanvas(img.naturalWidth, img.naturalHeight); const ctx = canvas.getContext('2d'); ctx.drawImage(img, 0, 0);
    const pixels = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    const hash = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', pixels))).map((v) => v.toString(16).padStart(2, '0')).join('');
    return { width: canvas.width, height: canvas.height, hash };
  });
}
async function nativeDialog(action) {
  if (action === 'cancel') { await until(() => { input('dialog-cancel', app.pid); return true; }, 'native dialog cancellation'); return; }
  if(process.argv.includes('--native-save-controls')){await until(()=>{input('dialog-save-native',app.pid,action);return true;},'scoped native file dialog controls');return;}
  const inspected = await until(() => execFileSync('powershell.exe',['-NoProfile','-File',join(root,'scripts/dialog-input.ps1'),'-TargetProcessId',String(app.pid),'-Action','inspect'],{cwd:root,windowsHide:true,encoding:'utf8',stdio:['ignore','pipe','pipe']}), 'native dialog control inventory', 10000);
  report.dialogControls ??= []; report.dialogControls.push(JSON.parse(inspected));
  const args = ['-NoProfile','-File',join(root,'scripts/dialog-input.ps1'),'-TargetProcessId',String(app.pid),'-Action',action === 'cancel' ? 'cancel' : 'save'];
  if (action !== 'cancel') args.push('-TestPath', action);
  await until(() => { execFileSync('powershell.exe',args,{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']}); return true; }, 'app-owned system dialog', 10000);
}
async function closeGracefully() {
  // CDP closes the app-owned main target through the actual Window close event.
  const exited = new Promise((resolve) => app.once('exit', resolve));
  input('close', app.pid); await Promise.race([exited, delay(8000).then(() => { throw new Error('Native close did not exit'); })]);
  await browser.close(); browser = undefined;
  await until(() => fetch(`http://127.0.0.1:${debugPort}/json/version`).then(() => false).catch(() => true), 'WebView2 cleanup');
}
try {
  if(fixtureMode){
    // The installed debug app embeds production assets; fixtures are explicit debug-only startup data.
  }
  await launch(); const initial = await prefs(); assert.ok(initial.registeredShortcut); report.initialShortcut = initial.registeredShortcut;
  await page.getByRole('button', { name: '设置', exact: true }).click();
  await page.getByRole('combobox', { name: '界面字体' }).selectOption('serif'); await until(async () => (await prefs()).preferences.fontFamily === 'serif', 'font persist');
  await page.getByRole('combobox', { name: '基础字号' }).selectOption('18'); await until(async () => (await prefs()).preferences.fontSize === 18, 'font size persist');
  await page.getByRole('button', { name: '浅色', exact: true }).click(); await until(async () => (await prefs()).preferences.theme === 'light', 'theme persist');
  assert.equal(await page.evaluate(() => getComputedStyle(document.documentElement).fontSize), '18px');
  report.checks.push('real Rust settings writes: serif, 18px, light; selected folder loaded');
  holder = ownedHelper('hold', 'new'); let held = ''; holder.stdout.on('data', (c) => { held += c; }); await until(() => held.includes('REGISTERED'), 'independent process owns Ctrl+Shift+3');
  await page.getByRole('textbox', { name: '区域截图快捷键' }).fill('Ctrl+Shift+3'); await page.getByRole('button', { name: '应用快捷键' }).click();
  await page.getByRole('alert').filter({ hasText: '占用' }).waitFor(); assert.equal((await prefs()).registeredShortcut, initial.registeredShortcut);
  assert.equal(JSON.parse(await readFile(settingsPath, 'utf8')).captureShortcut, 'Ctrl+Shift+2');
  await stopChild(holder); holder = undefined;
  await page.getByRole('button', { name: '应用快捷键' }).click(); await until(async () => (await prefs()).preferences.captureShortcut.includes('3'), 'new shortcut registered');
  assert.match(input('probe', 'default'), /REGISTERED/); assert.throws(() => input('probe', 'new'));
  report.checks.push('Windows registration conflict preserves old setting; changing binding releases old shortcut');
  await page.getByRole('button', { name: '选择保存目录' }).click(); await nativeDialog('cancel'); assert.equal((await prefs()).preferences.saveDirectory, exportsDir);
  report.checks.push('real system folder dialog cancellation preserves chosen directory');
  if(process.argv.includes('--settings-only')){
    await until(()=>page.getByRole('button',{name:'选择保存目录'}).isEnabled(),'folder cancellation completed');
    const beforeFull=await prefs();assert.ok(beforeFull.registeredFullscreenShortcut?.includes('1'));assert.throws(()=>input('probe','full-default'));
    const full=page.getByRole('textbox',{name:'全屏截图快捷键',exact:true}),applyFull=page.getByRole('button',{name:'应用全屏快捷键',exact:true});await full.fill('Ctrl+Shift+3');await applyFull.click();await page.getByRole('alert').filter({hasText:'相同'}).waitFor();assert.equal((await prefs()).registeredFullscreenShortcut,beforeFull.registeredFullscreenShortcut);assert.equal((await prefs()).registeredShortcut,beforeFull.registeredShortcut);report.checks.push('native settings reject shared region/fullscreen key and retain both registrations');
    holder=ownedHelper('hold','full-new');let fullHeld='';holder.stdout.on('data',c=>fullHeld+=c);await until(()=>fullHeld.includes('REGISTERED'),'independent full-screen key holder');await full.fill('Ctrl+Shift+4');await applyFull.click();await page.getByRole('alert').filter({hasText:'占用'}).waitFor();assert.equal((await prefs()).registeredFullscreenShortcut,beforeFull.registeredFullscreenShortcut);assert.equal((await prefs()).registeredShortcut,beforeFull.registeredShortcut);await stopChild(holder);holder=undefined;
    await applyFull.click();await until(async()=> (await prefs()).preferences.fullscreenShortcut.includes('4'),'full-screen setting persist');assert.match(input('probe','full-default'),/REGISTERED/);assert.throws(()=>input('probe','full-new'));assert.throws(()=>input('probe','new'));const pair=JSON.parse(await readFile(settingsPath,'utf8'));assert.ok(pair.captureShortcut.includes('3'));assert.ok(pair.fullscreenShortcut.includes('4'));report.checks.push('real OS conflict rollback and independent full-screen change release old key and preserve region');
    await closeGracefully();assert.match(input('probe','new'),/REGISTERED/);assert.match(input('probe','full-new'),/REGISTERED/);report.checks.push('actual main window close releases both registered shortcuts');
    await launch();const loaded=await prefs();assert.equal(loaded.preferences.theme,'light');assert.equal(loaded.preferences.fontFamily,'serif');assert.equal(loaded.preferences.fontSize,18);assert.equal(loaded.preferences.saveDirectory,exportsDir);assert.ok(loaded.registeredShortcut?.includes('3'));assert.ok(loaded.registeredFullscreenShortcut?.includes('4'));assert.throws(()=>input('probe','new'));assert.throws(()=>input('probe','full-new'));report.checks.push('restart restores actual persisted settings and registers both configured shortcuts');
    await closeGracefully();report.result=report.errors.length?'functional-checks-passed-with-shutdown-diagnostics':'passed';report.shortcutTrigger='OS registration/conflict/lifecycle only';
  }else{
  receiver = await chromium.launch({ channel: 'msedge', headless: false, args: ['--start-fullscreen'] });
  const receiverPage = await receiver.newPage({ viewport: null });
  const receiverHtml = `<html><body style="margin:0;background:#273e58;color:white;font:24px system-ui"><h1>ResearchClip clipboard acceptance image viewer</h1><p>Native screenshot, then Ctrl+V to paste the Windows clipboard image.</p><canvas id="image"></canvas><script>window.pasted=null;document.addEventListener('paste',async e=>{const file=Array.from(e.clipboardData.items).find(i=>i.type.startsWith('image/'))?.getAsFile();if(!file){window.pasted={error:'no image item'};return}const bitmap=await createImageBitmap(file);const canvas=document.querySelector('canvas');canvas.width=bitmap.width;canvas.height=bitmap.height;const ctx=canvas.getContext('2d');ctx.drawImage(bitmap,0,0);bitmap.close();const pixels=ctx.getImageData(0,0,canvas.width,canvas.height).data;const hash=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',pixels))).map(v=>v.toString(16).padStart(2,'0')).join('');window.pasted={width:canvas.width,height:canvas.height,hash,type:file.type};});</script></body></html>`;
  fixtureServer = createServer((req, res) => { res.setHeader("Content-Type", "text/html; charset=utf-8"); res.end(receiverHtml); });
  await new Promise((resolve) => fixtureServer.listen(0, "127.0.0.1", resolve));
  await receiverPage.goto(`http://127.0.0.1:${fixtureServer.address().port}`);
  if(fixtureMode){
    await receiverPage.context().grantPermissions(['clipboard-read','clipboard-write'],{origin:receiverPage.url()});
    await receiverPage.evaluate(()=>{const button=document.createElement('button');button.textContent='粘贴图片';document.body.prepend(button);button.onclick=async()=>{try{const items=await navigator.clipboard.read();const item=items.find(i=>i.types.some(t=>t.startsWith('image/')));if(!item)throw new Error('No OS clipboard image');const type=item.types.find(t=>t.startsWith('image/'));const blob=await item.getType(type);const bitmap=await createImageBitmap(blob);const canvas=document.querySelector('canvas');canvas.width=bitmap.width;canvas.height=bitmap.height;const ctx=canvas.getContext('2d');ctx.drawImage(bitmap,0,0);bitmap.close();const pixels=ctx.getImageData(0,0,canvas.width,canvas.height).data;const hash=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',pixels))).map(v=>v.toString(16).padStart(2,'0')).join('');window.pasted={width:canvas.width,height:canvas.height,hash,type};}catch(e){window.pasted={error:e.message};}};});
  }
  const receiverCdp = await receiver.newBrowserCDPSession(); const processInfo = await receiverCdp.send('SystemInfo.getProcessInfo'); const receiverPid = processInfo.processInfo.find((p) => p.type === 'browser').id;
  const startShortcut = async () => {
    input('minimize', app.pid); await receiverPage.bringToFront(); await delay(300); if(messageTrigger)input('dispatch-hotkey',app.pid);else input('send', receiverPid, 'new');
    const overlay = await until(() => browser.contexts().flatMap((c) => c.pages()).find((p) => p.url().includes('captureOverlay=')), 'global shortcut overlay');
    await until(() => overlay.evaluate(() => document.querySelector('.capture-overlay[data-ready="true"]') !== null && document.hasFocus()), 'focused overlay'); return overlay;
  };
  let expectedWidth=320,expectedHeight=200;
  if(!fixtureMode){
  let overlay = await startShortcut(); if(messageTrigger)input('dispatch-hotkey',app.pid);else input('send', app.pid, 'new'); await delay(200); assert.equal(browser.contexts().flatMap((c) => c.pages()).filter((p) => p.url().includes('captureOverlay')).length, 1);
  await overlay.keyboard.press('Escape'); await until(() => overlay.isClosed(), 'cancelled overlay destroyed'); assert.equal(await current(), null); assert.equal((await readdir(exportsDir)).length, 0);
  report.checks.push('background native shortcut: focused region layer, duplicate input ignored, Esc leaves no capture/file');
  overlay = await startShortcut();
  const overlayMetrics=await overlay.evaluate(()=>({width:innerWidth,height:innerHeight,dpr:devicePixelRatio,frameWidth:document.querySelector('img').naturalWidth,frameHeight:document.querySelector('img').naturalHeight}));report.overlayMetrics=overlayMetrics;
  expectedWidth=Math.ceil(420*overlayMetrics.frameWidth/overlayMetrics.width)-Math.floor(100*overlayMetrics.frameWidth/overlayMetrics.width);expectedHeight=Math.ceil(320*overlayMetrics.frameHeight/overlayMetrics.height)-Math.floor(120*overlayMetrics.frameHeight/overlayMetrics.height);
  await overlay.mouse.move(100, 120); await overlay.mouse.down(); await overlay.mouse.move(420, 320, { steps: 6 }); await overlay.mouse.up(); await until(() => overlay.isClosed(), 'committed overlay destroyed');
  }
  if(fixtureMode)await page.getByRole('button',{name:'截图素材',exact:true}).click();
  await page.getByRole('button', { name: '复制图片', exact: true }).waitFor(); const info = await current(); assert.equal(info.widthPx, expectedWidth); assert.equal(info.heightPx, expectedHeight);
  const preview = page.getByRole('img', { name: fixtureMode?'全屏截图预览':'区域截图预览' }); const before = await pngHash(preview); assert.equal((await readdir(exportsDir)).length, 0);
  await page.getByRole('button', { name: '打开编辑器', exact: true }).click(); await page.getByRole('application', {name:'图片编辑画布'}).waitFor();await page.getByRole('button',{name:'返回截图',exact:true}).click();
  await page.getByRole('button', { name: '复制图片', exact: true }).click(); await page.getByText('图片已复制，可粘贴到画图、Word 等应用。', { exact: true }).waitFor();
  await receiverPage.bringToFront(); await delay(150); if(fixtureMode){await receiverPage.getByRole('button',{name:'粘贴图片'}).click();}else input('send', receiverPid, 'paste'); const pasted = await until(() => receiverPage.evaluate(() => window.pasted), 'real clipboard image paste');
  assert.deepEqual({ width: pasted.width, height: pasted.height, hash: pasted.hash }, before); assert.match(pasted.type, /^image\//); await receiverPage.screenshot({ path: join(output, 'clipboard-pasted.png') });
  report.clipboard = { width: pasted.width, height: pasted.height, format: pasted.type, allPixelsMatched: true };
  report.checks.push(fixtureMode?'real Windows clipboard pasted via Clipboard API in separate Edge image viewer; all RGBA pixels matched controlled PNG':'real OS clipboard image pasted in separate Edge image viewer; all RGBA pixels matched native crop');
  if(!process.argv.includes('--skip-clipboard-lock')){
  clipboardLock = ownedHelper('lock-clipboard'); let locked = ''; clipboardLock.stdout.on('data', (c) => { locked += c; }); await until(() => locked.includes('CLIPBOARD_LOCKED'), 'clipboard locked by independent process');
  assert.match(input('probe-clipboard'),/CLIPBOARD_BUSY/,'Independent process must actually observe the held clipboard lock');
  await page.getByRole('button', { name: '复制图片', exact: true }).click(); await page.getByRole('alert').filter({ hasText: '剪贴板' }).waitFor(); assert.equal((await current()).captureId, info.captureId);
  await stopChild(clipboardLock); clipboardLock = undefined;
  report.checks.push('locked Windows clipboard produces readable error and preserves screenshot');
  }else report.skipped=['Windows clipboard contention: previous real lock scenario did not yield expected error; manual recheck required'];
  await page.getByRole('button', { name: '保存 PNG', exact: true }).click(); await nativeDialog('cancel'); await until(() => page.getByRole('button', { name: '保存 PNG', exact: true }).isEnabled(), 'save cancellation'); assert.equal((await readdir(exportsDir)).length, 0);
  const savePath = join(exportsDir, 'native.png'); await page.getByRole('button', { name: '保存 PNG', exact: true }).click(); await nativeDialog(savePath); await page.getByText('PNG 已保存。', { exact: true }).waitFor();
  const savedCaption=await page.getByRole('region',{name:'屏幕截图',exact:true}).locator('figcaption p').innerText();
  const actualSavePath=savedCaption.slice('保存位置：'.length);assert.equal(dirname(actualSavePath),exportsDir,'Native dialog result must stay in the selected test folder');
  const saved = await readFile(actualSavePath); assert.deepEqual([...saved.subarray(0, 8)], [137,80,78,71,13,10,26,10]); assert.equal(saved.readUInt32BE(16), expectedWidth); assert.equal(saved.readUInt32BE(20), expectedHeight);
  const filePixels = await page.evaluate(async (base64) => { const bytes=Uint8Array.from(atob(base64),c=>c.charCodeAt(0));const bitmap=await createImageBitmap(new Blob([bytes],{type:'image/png'}));const canvas=new OffscreenCanvas(bitmap.width,bitmap.height);const ctx=canvas.getContext('2d');ctx.drawImage(bitmap,0,0);bitmap.close();const data=ctx.getImageData(0,0,canvas.width,canvas.height).data;return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',data))).map(v=>v.toString(16).padStart(2,'0')).join(''); }, saved.toString('base64'));
  assert.equal(filePixels, before.hash); report.checks.push('native system Save As cancellation and PNG export; file pixels equal native preview');
  const rejected = await page.evaluate(async (captureId) => { try { await window.__TAURI_INTERNALS__.invoke('update_preferences', {patch:{saveDirectory:'C:/arbitrary'}}); return false; } catch { /* expected path rejection */ } try { await window.__TAURI_INTERNALS__.invoke('copy_capture_image', {captureId}); return false; } catch(e) { return e.code === 'INVALID_SESSION'; } }, '../escape.png'); assert.equal(rejected, true);
  await page.getByRole('button', { name: '取消', exact: true }).click(); await until(async () => await current() === null, 'discard resource'); assert.equal((await readdir(exportsDir)).length, 1); report.checks.push('opaque resource validation, no arbitrary settings paths; cancel releases resource and preserves exported file');
  await page.screenshot({ path: join(output, 'main.png') });
  await closeGracefully(); assert.match(input('probe', 'new'), /REGISTERED/); report.checks.push('actual app close exits and frees registered shortcut');
  await launch(); const restarted = await prefs(); assert.equal(restarted.preferences.theme, 'light'); assert.equal(restarted.preferences.fontFamily, 'serif'); assert.equal(restarted.preferences.fontSize, 18); assert.equal(restarted.preferences.saveDirectory, exportsDir); assert.ok(restarted.registeredShortcut?.includes('3')); assert.equal((await current())===null,!fixtureMode);
  report.checks.push('restart restores persisted appearance, shortcut and directory; no stale screenshot');
  await closeGracefully(); assert.equal(report.errors.length, 0); report.result = 'passed';
  }
} catch (error) {
  if (page && !page.isClosed()) report.events = await page.evaluate(() => window.workflowEvents).catch(() => []);
  report.result = 'failed'; report.failure = error.stack; if (page && !page.isClosed()) await page.screenshot({ path: join(output, 'failure.png') }).catch(() => {}); process.exitCode = 1;
} finally {
  await stopChild(holder); await stopChild(clipboardLock); if (browser) await browser.close().catch(() => {}); await stopChild(app); if (receiver) await receiver.close();
  await writeFile(join(output, 'report.json'), JSON.stringify(report, null, 2)); console.log(JSON.stringify(report, null, 2));
  if (fixtureServer) await new Promise((resolve) => fixtureServer.close(resolve));
  await stopChild(frontendServer);
}
