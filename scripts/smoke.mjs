import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { mkdir, writeFile, readdir } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium } from 'playwright-core';
import { checkOverlayRenderer } from './overlay-smoke.mjs';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const desktop = process.argv.includes('--desktop');
const regionOnly = process.argv.includes('--region-only');
assert.ok(!regionOnly || desktop, '--region-only requires --desktop');
const output = join(root, 'artifacts');
const version = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8')).version;
const runIdIndex = process.argv.indexOf('--run-id');
const runId = runIdIndex === -1 ? '' : process.argv[runIdIndex + 1];
if (runIdIndex !== -1) assert.match(runId ?? '', /^[a-z0-9-]{1,64}$/, 'Use a short lowercase run ID');
const label = `${desktop ? 'region-capture-desktop' : 'region-capture-preview'}${runId ? `-${runId}` : ''}`;
const nativeTemp = join(output, runId ? `${label}-native-temp` : 'region-capture-native-temp');
const report = { mode: regionOnly ? 'desktop-region' : desktop ? 'desktop' : 'preview', version, checks: [], errors: [] };
await mkdir(output, { recursive: true });
let currentPage;
let browser;
let server;
let fixtureBrowser;
let fixturePage;
let fixturePid, appPid;
function raiseFrozenFixture(){
  assert.ok(fixturePid&&appPid);
  execFileSync(join(root,'src-tauri/target/debug/examples/raise_capture_fixture.exe'),[String(fixturePid),String(appPid)],{cwd:root,windowsHide:true,stdio:'pipe'});
}
const expectedConsoleUrls = new Set();

async function until(action, description, limit = 30000) {
  const started = Date.now();
  let lastError;
  while (Date.now() - started < limit) {
    try { const result = await action(); if (result) return result; } catch (error) { lastError = error; }
    await delay(250);
  }
  throw new Error(`Timed out: ${description}${lastError ? ` (${lastError.message})` : ''}`);
}

function observe(page) {
  page.on('pageerror', (error) => report.errors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error' && !expectedConsoleUrls.has(message.location().url)) {
      report.errors.push(`${message.text()} (${message.location().url})`);
    }
  });
}

async function checkNativeCapture(page, suffix) {
  const prefix = suffix ? 'restart: ' : '';
  const displays = await page.evaluate(() => window.__TAURI_INTERNALS__.invoke('list_displays'));
  assert.ok(displays.length > 0);
  report.displays = displays;
  const selected = displays.find((item) => item.isPrimary) ?? displays[0];
  await page.getByRole('button', { name: '截图素材', exact: true }).click();
  await page.getByRole('button', { name: '收起欢迎引导' }).click();
  const panel = page.getByRole('region', { name: '屏幕截图', exact: true });
  const select = panel.getByRole('combobox', { name: '目标显示器' });
  await select.selectOption(selected.id);
  const button = panel.getByRole('button', { name: '全屏截图', exact: true });
  const preview = panel.getByRole('img', { name: '全屏截图预览' });
  assert.equal(await preview.count(), 0, 'A new app session must not load a saved screenshot');
  const urls = [];
  for (const phase of [1, 2]) {
    await fixturePage.evaluate((value) => { document.body.dataset.phase = String(value); }, phase);
    await fixturePage.bringToFront();
    await fixturePage.waitForTimeout(250);
    const start = Date.now();
    await button.click();
    raiseFrozenFixture();
    await until(async () => {
      if (await panel.getByRole('alert').count()) throw new Error(await panel.getByRole('alert').innerText());
      return await preview.count() > 0 && await button.isEnabled();
    }, 'real native preview');
    const result = await preview.evaluate(async (img, value) => {
      await img.decode();
      const bitmap = await createImageBitmap(img);
      const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
      const context = canvas.getContext('2d');
      context.drawImage(bitmap, 0, 0);
      bitmap.close();
      const expected = value === 1 ? [[232, 76, 76], [75, 185, 132], [80, 120, 232], [232, 185, 65]] : [[75, 185, 132], [232, 76, 76], [232, 185, 65], [80, 120, 232]];
      // Inner points lie inside the main window's pre-hide position and reveal fade ghosts.
      const positions = [[0.2, 0.2], [0.8, 0.2], [0.2, 0.8], [0.8, 0.8], [0.3, 0.3], [0.7, 0.3], [0.3, 0.7], [0.7, 0.7], [0.5, 0.5]];
      expected.push(...expected.map((color) => [...color]), [245, 245, 245]);
      const matches = positions.map(([x, y], index) => {
        const pixel = context.getImageData(Math.floor(x * canvas.width), Math.floor(y * canvas.height), 1, 1).data;
        return expected[index].every((channel, c) => Math.abs(channel - pixel[c]) <= 2) && pixel[3] === 255;
      });
      return { width: img.naturalWidth, height: img.naturalHeight, url: img.src, markerMatches: matches };
    }, phase);
    assert.equal(result.width, selected.widthPx);
    assert.equal(result.height, selected.heightPx);
    assert.deepEqual(result.markerMatches, Array(9).fill(true), 'Actual screen fixture colors/orientation/self-exclusion mismatch');
    await panel.getByText(`${selected.widthPx} × ${selected.heightPx} 像素`, { exact: true }).waitFor();
    urls.push(result.url);
    report.checks.push(`${prefix}real capture ${phase}: ${result.width}x${result.height}, nine on-screen markers matched, elapsed ${Date.now() - start}ms`);
  }
  assert.notEqual(urls[0], urls[1]);
  expectedConsoleUrls.add(urls[0]); // Only this deliberately revoked URL may emit a resource error.
  assert.equal(await page.evaluate((url) => new Promise((resolve) => {
    const image = new Image();
    image.onload = () => resolve(false);
    image.onerror = () => resolve(true);
    image.src = url;
  }), urls[0]), true, 'Previous Blob URL must be revoked');

  const errors = await page.evaluate(async () => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    const result = [];
    for (const displayId of ['', 'missing-test-monitor']) {
      try { await invoke('capture_fullscreen', { displayId }); result.push('unexpected-success'); }
      catch (error) { result.push(error.code); }
    }
    return result;
  });
  assert.deepEqual(errors, ['INVALID_DISPLAY', 'DISPLAY_GONE']);
  const duplicate = await page.evaluate(async (displayId) => {
    const run = () => window.__TAURI_INTERNALS__.invoke('capture_fullscreen', { displayId }).then((bytes) => ({ ok: typeof bytes?.captureId === 'string' })).catch((error) => ({ code: error.code }));
    return Promise.all([run(), run()]);
  }, selected.id);
  assert.equal(duplicate.filter((item) => item.ok).length, 1);
  assert.equal(duplicate.filter((item) => item.code === 'BUSY').length, 1);
  await panel.getByRole('button', { name: '关闭截图预览' }).click();
  assert.equal(await preview.count(), 0);
  await fixturePage.bringToFront();
  await button.click();
  await until(async () => await preview.count() > 0 && await button.isEnabled(), 'capture after native errors');
  await preview.evaluate((img) => img.decode());
  await page.screenshot({ path: join(output, `${label}${suffix}-preview.png`) });
  await panel.getByRole('button',{name:'打开编辑器',exact:true}).click();
  const editorCanvas=page.getByRole('application',{name:'图片编辑画布'});await editorCanvas.waitFor();
  const imageExtent=()=>editorCanvas.evaluate(host=>{const c=host.querySelector('canvas');if(!c||!c.width||!c.height)return null;const p=c.getContext('2d').getImageData(0,0,c.width,c.height).data;let x=c.width,y=c.height,count=0;for(let i=0;i<p.length;i+=4)if(p[i+3]>0){x=Math.min(x,(i/4)%c.width);y=Math.min(y,Math.floor(i/4/c.width));count++;}return count?{x,y,ratioX:c.width/Math.floor(host.getBoundingClientRect().width),ratioY:c.height/Math.floor(host.getBoundingClientRect().height)}:null;});
  const imageBefore=await until(imageExtent,'native screen image rendered in editor');await until(()=>page.locator('.editor-coordinate-status').innerText().then(text=>text.startsWith(`原图 ${selected.widthPx} × ${selected.heightPx} 像素`)),'original dimensions retained in editor');
  await page.getByRole('button',{name:'平移工具',exact:true}).click();const host=await editorCanvas.boundingBox();await page.mouse.move(host.x+50,host.y+50);await page.mouse.down();await page.mouse.move(host.x+90,host.y+80,{steps:8});await page.mouse.up();
  const imageAfter=await imageExtent();assert.ok(Math.abs((imageAfter.x-imageBefore.x)/imageBefore.ratioX-40)<1);assert.ok(Math.abs((imageAfter.y-imageBefore.y)/imageBefore.ratioY-30)<1);
  await page.getByRole('button',{name:'返回截图',exact:true}).click();await panel.getByRole('button',{name:'关闭截图预览'}).click();
  await panel.getByRole('button',{name:'图片编辑器 / 打开项目',exact:true}).click();await editorCanvas.waitFor();assert.ok(await until(imageExtent,'editor owns native screenshot after capture preview cleared'));await until(()=>page.locator('.editor-coordinate-status').innerText().then(text=>text.startsWith(`原图 ${selected.widthPx} × ${selected.heightPx} 像素`)),'original dimensions retained in editor');await page.getByRole('button',{name:'返回截图',exact:true}).click();
  report.checks.push(`${prefix}actual native screenshot opens in editor, rendered image pans 40/30 CSS pixels, editor survives preview release`);
  report.checks.push(`${prefix}real invalid/missing display errors, BUSY gate, recovery, Blob replacement and clear`);
}

async function checkNativeRegion(page, suffix) {
  const prefix = suffix ? 'restart: ' : '';
  if (!report.displays) report.displays = await page.evaluate(() => window.__TAURI_INTERNALS__.invoke('list_displays'));
  const welcome = page.getByRole('button', { name: '收起欢迎引导' });
  if (await welcome.count()) await welcome.click();
  const panel = page.getByRole('region', { name: '屏幕截图', exact: true });
  const button = panel.getByRole('button', { name: '区域截图', exact: true });
  const preview = panel.getByRole('img', { name: '区域截图预览' });
  const selectedId = await panel.getByRole('combobox', { name: '目标显示器' }).inputValue();
  const display = report.displays.find((item) => item.id === selectedId);
  const savedRoot = join(nativeTemp, 'ResearchClip', 'region-captures');
  const files = async () => {
    try { return (await readdir(savedRoot)).filter((name) => name.endsWith('.png')); }
    catch (error) { if (error.code === 'ENOENT') return []; throw error; }
  };
  const pages = () => page.context().browser().contexts().flatMap((context) => context.pages());
  let previousSession;
  const start = async () => {
    await fixturePage.bringToFront();
    await delay(150);
    await button.click();
    raiseFrozenFixture();
    const overlay = await until(() => pages().find((item) => item.url().includes('captureOverlay=')), 'region overlay created');
    currentPage = overlay;
    observe(overlay);
    await overlay.locator('.capture-overlay[data-ready="true"]').waitFor();
    const state = await overlay.evaluate(() => ({ width: innerWidth, height: innerHeight, dpr: devicePixelRatio, focused: document.hasFocus(), id: new URLSearchParams(location.search).get('captureOverlay') }));
    assert.equal(state.focused, true, 'Overlay must own keyboard focus');
    assert.ok(Math.abs(state.width * state.dpr - display.widthPx) <= 2);
    assert.ok(Math.abs(state.height * state.dpr - display.heightPx) <= 2);
    assert.notEqual(state.id, previousSession, 'Every overlay has a fresh session');
    previousSession = state.id;
    assert.equal(pages().filter((item) => item.url().includes('captureOverlay=')).length, 1);
    return { overlay, state };
  };
  const finish = async (overlay) => {
    await until(() => overlay.isClosed() && !pages().some((item) => item.url().includes('captureOverlay=')), 'overlay destroyed');
    await until(() => button.isEnabled(), 'main window restored');
    currentPage = page;
    assert.equal(await page.evaluate(() => document.hasFocus()), true, 'Main window must regain focus');
    assert.equal(await panel.getByRole('alert').count(), 0);
  };
  const saved = async (width, height) => {
    await preview.waitFor();
    await preview.evaluate((img) => img.decode());
    assert.deepEqual(await preview.evaluate((img) => [img.naturalWidth, img.naturalHeight]), [width, height]);
    const raw = await page.evaluate(async()=>{const info=await window.__TAURI_INTERNALS__.invoke('current_capture');return Array.from(new Uint8Array(await window.__TAURI_INTERNALS__.invoke('capture_result_png',{captureId:info.captureId})));});
    const bytes=Buffer.from(raw);
    assert.equal((await files()).length,0,'Capture completion must not automatically save files');
    assert.equal(bytes.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
    assert.equal(bytes.readUInt32BE(16), width);
    assert.equal(bytes.readUInt32BE(20), height);
    // CSP intentionally blocks fetch(blob:). Compare decoded pixels without widening app policy.
    const samePixels = await preview.evaluate(async (img, encoded) => {
      const fromDisk = await createImageBitmap(new Blob([Uint8Array.from(encoded)], { type: 'image/png' }));
      const displayed = await createImageBitmap(img);
      try {
        if (fromDisk.width !== displayed.width || fromDisk.height !== displayed.height) return false;
        const canvas = new OffscreenCanvas(fromDisk.width, fromDisk.height);
        const context = canvas.getContext('2d');
        context.drawImage(fromDisk, 0, 0);
        const diskPixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
        context.clearRect(0, 0, canvas.width, canvas.height);
        context.drawImage(displayed, 0, 0);
        const shownPixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
        return diskPixels.every((channel, index) => channel === shownPixels[index]);
      } finally { fromDisk.close(); displayed.close(); }
    }, Array.from(bytes));
    assert.equal(samePixels, true, 'Saved native PNG pixels must equal the displayed image');
    return preview.getAttribute('src');
  };

  await fixturePage.evaluate(() => { document.body.dataset.phase = '1'; });
  await delay(250);
  const first = await start();
  const frozenHash = await first.overlay.getByRole('img', { name: '原生冻结屏幕' }).evaluate(async (img) => {
    const bitmap = await createImageBitmap(img);
    const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
    const context = canvas.getContext('2d');
    context.drawImage(bitmap, 0, 0); bitmap.close();
    const pixels = context.getImageData(canvas.width / 4, canvas.height / 4, canvas.width / 2, canvas.height / 2).data;
    return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', pixels))).map((value) => value.toString(16).padStart(2, '0')).join('');
  });
  // Change the real underlying screen after the native frame has been frozen.
  await fixturePage.evaluate(() => { document.body.dataset.phase = '2'; });
  const { width, height } = first.state;
  await first.overlay.mouse.move(width / 4, height / 4);
  await first.overlay.mouse.down();
  await first.overlay.mouse.move(width * 3 / 4, height * 3 / 4, { steps: 12 });
  await first.overlay.locator('.selection-size').getByText(`${display.widthPx / 2} × ${display.heightPx / 2} px`, { exact: true }).waitFor();
  await first.overlay.screenshot({ path: join(output, `${label}${suffix}-selection.png`) });
  const permission = await first.overlay.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('get_workspace_status'); return 'unexpected-success'; }
    catch (error) { return String(error); }
  });
  assert.match(permission, /not allowed|denied/i, 'Overlay must not access the workspace');
  await first.overlay.mouse.up();
  await finish(first.overlay);
  await saved(display.widthPx / 2, display.heightPx / 2);
  const regionHash = await preview.evaluate(async (img) => {
    const bitmap = await createImageBitmap(img);
    const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
    const context = canvas.getContext('2d');
    context.drawImage(bitmap, 0, 0); bitmap.close();
    return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', context.getImageData(0, 0, canvas.width, canvas.height).data))).map((value) => value.toString(16).padStart(2, '0')).join('');
  });
  assert.equal(regionHash, frozenHash, 'Every cropped pixel must match the original native frozen frame');
  const matches = await preview.evaluate(async (img) => {
    const bitmap = await createImageBitmap(img);
    const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
    const context = canvas.getContext('2d');
    context.drawImage(bitmap, 0, 0); bitmap.close();
    const positions = [[.1, .1], [.9, .1], [.1, .9], [.9, .9], [.5, .5]];
    const expected = [[232, 76, 76], [75, 185, 132], [80, 120, 232], [232, 185, 65], [245, 245, 245]];
    return positions.map(([x, y], i) => {
      const pixel = context.getImageData(Math.floor(x * canvas.width), Math.floor(y * canvas.height), 1, 1).data;
      return pixel[3] === 255 && expected[i].every((v, c) => Math.abs(v - pixel[c]) <= 2);
    });
  });
  if (!regionOnly) assert.deepEqual(matches, Array(5).fill(true), 'Region must use the original native frame without overlay masks or live screen changes');
  report.checks.push(`${prefix}region overlay focus, physical boundaries, live size, frozen native pixels and exact PNG resource/preview match`);
  await page.screenshot({ path: join(output, `${label}${suffix}-region.png`) });

  const second = await start();
  const countBeforeZero = (await files()).length;
  await second.overlay.mouse.click(100, 100);
  await second.overlay.getByRole('status').getByText('选区没有面积', { exact: false }).waitFor();
  assert.equal(second.overlay.isClosed(), false);
  assert.equal((await files()).length, countBeforeZero);
  await second.overlay.mouse.move(240, 190);
  await second.overlay.mouse.down();
  await second.overlay.mouse.move(40, 30, { steps: 8 });
  await second.overlay.mouse.up();
  await finish(second.overlay);
  const reverseWidth = Math.ceil(240 * display.widthPx / width) - Math.floor(40 * display.widthPx / width);
  const reverseHeight = Math.ceil(190 * display.heightPx / height) - Math.floor(30 * display.heightPx / height);
  const priorUrl = await saved(reverseWidth, reverseHeight);
  const countBeforeCancel = (await files()).length;
  const cancelled = await start();
  await cancelled.overlay.mouse.move(10, 10);
  await cancelled.overlay.mouse.down();
  await cancelled.overlay.mouse.move(70, 80);
  await cancelled.overlay.keyboard.press('Escape');
  await finish(cancelled.overlay);
  assert.equal(await preview.getAttribute('src'), priorUrl, 'Esc must preserve the previous preview');
  assert.equal((await files()).length, countBeforeCancel, 'Esc must not save a PNG');
  report.checks.push(`${prefix}zero area remains selectable, reverse drag crops correctly, Esc closes/restores without new files`);

  const edge = await start();
  await edge.overlay.mouse.move(50, 50);
  await edge.overlay.mouse.down();
  await edge.overlay.mouse.move(-60, -60, { steps: 5 });
  await edge.overlay.mouse.up();
  await finish(edge.overlay);
  await saved(Math.ceil(50 * display.widthPx / width), Math.ceil(50 * display.heightPx / height));
  const mainPermission = await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('region_overlay_info', { sessionId: '1' }); return 'unexpected-success'; }
    catch (error) { return String(error); }
  });
  assert.match(mainPermission, /not allowed|denied/i, 'Main must not impersonate the overlay');
  const pathError = await page.evaluate(async () => {
    try { await window.__TAURI_INTERNALS__.invoke('region_result_png', { captureId: '../escape.png' }); return 'unexpected-success'; }
    catch (error) { return error.code; }
  });
  assert.equal(pathError, 'INVALID_SESSION');
  if (!suffix) {
    for (let index = 0; index < 20; index += 1) {
      const beforeCount = (await files()).length;
      const loop = await start();
      if (index % 2 === 0) await loop.overlay.keyboard.press('Escape');
      else {
        await loop.overlay.mouse.move(10, 10); await loop.overlay.mouse.down();
        await loop.overlay.mouse.move(26, 28); await loop.overlay.mouse.up();
      }
      await finish(loop.overlay);
      assert.equal((await files()).length, beforeCount, 'Selections and Esc must not automatically write files');
      if (index % 2) await saved(Math.ceil(26 * display.widthPx / width) - Math.floor(10 * display.widthPx / width), Math.ceil(28 * display.heightPx / height) - Math.floor(10 * display.heightPx / height));
    }
    report.checks.push('20 consecutive region sessions: 10 previews, 10 Esc cancellations; no automatic files, unique sessions, no remaining overlays, main focus restored');
  }
  await panel.getByRole('button', { name: '关闭截图预览' }).click();
  report.checks.push(`${prefix}pointer capture clamps outside top/left, repeated overlays fully destroyed, capability and resource-ID isolation`);
}

async function checkUI(page, suffix = '') {
  currentPage = page;
  const prefix = suffix ? 'restart: ' : '';
  const nav = page.getByRole('navigation', { name: '主导航' });
  const toolbar = page.locator('.app-toolbar');
  const theme = () => page.evaluate(() => document.documentElement.dataset.theme);
  const screenshot = async (name) => {
    await page.waitForTimeout(150); // Let the 120ms color transitions settle for evidence.
    await page.screenshot({ path: join(output, name) });
  };
  const sessionStorageBefore = await page.evaluate(() => JSON.stringify({ local: { ...localStorage }, session: { ...sessionStorage } }));
  const commands = [];
  if (desktop) page.on('request', (request) => {
    const url = new URL(request.url());
    if (url.hostname === 'ipc.localhost') commands.push(decodeURIComponent(url.pathname.slice(1)));
  });
  await page.getByRole('heading', { name: 'ResearchClip', exact: true }).waitFor();
  if(desktop){const preferences=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('get_preferences'));await until(()=>theme().then(value=>value===preferences.preferences.theme),'native appearance initialized');if(suffix)assert.equal(preferences.preferences.theme,'light','previous run theme persists on restart');if(preferences.preferences.theme==='light'){await toolbar.getByRole('button',{name:'切换为深色主题'}).click();await until(()=>theme().then(value=>value==='dark'),'restore dark through UI before shared visual assertions');}}
  await page.getByRole('heading', { name: '欢迎使用 ResearchClip', exact: true }).waitFor();
  await page.locator('.sidebar-version').getByText(`v${version}`, { exact: true }).waitFor();
  assert.equal(await theme(), 'dark');
  assert.equal(await nav.getByRole('button', { name: '截图素材', exact: true }).getAttribute('aria-current'), 'page');
  assert.equal(await page.getByRole('region',{name:'截图素材库',exact:true}).getAttribute('data-view-mode'), 'grid');
  await screenshot(`${label}${suffix}-dark.png`);

  await toolbar.getByRole('button', { name: '列表视图', exact: true }).click();
  await until(async()=>await page.getByRole('region',{name:'截图素材库',exact:true}).getAttribute('data-view-mode')==='list','view preference committed by Rust');
  assert.equal(await page.getByRole('region',{name:'截图素材库',exact:true}).getAttribute('data-view-mode'), 'list');
  assert.equal(await toolbar.getByRole('button', { name: '列表视图' }).getAttribute('aria-pressed'), 'true');
  await page.getByRole('button', { name: '收起欢迎引导' }).click();
  assert.equal(await page.getByRole('heading', { name: '欢迎使用 ResearchClip' }).count(), 0);
  await nav.getByRole('button', { name: '收藏', exact: true }).click();
  await page.getByRole('heading', { name: '为重要的图像留个位置' }).waitFor();
  await nav.getByRole('button', { name: '标签', exact: true }).click();
  await page.getByRole('heading', { name: '让素材形成自己的脉络' }).waitFor();
  await page.getByRole('button', { name: '使用指南', exact: true }).click();
  await page.getByRole('heading', { name: '欢迎使用 ResearchClip' }).waitFor();
  report.checks.push(`${prefix}navigation, list/grid state, dismiss and reopen guide`);

  await toolbar.getByRole('button', { name: '新建截图', exact: true }).click();
  const capture = page.getByRole('dialog', { name: '新建截图', exact: true });
  await capture.waitFor();
  await capture.getByRole('button', { name: '区域截图', exact: true }).waitFor();
  if (desktop) await until(async () => await capture.getByRole('button', { name: '全屏截图', exact: true }).isEnabled(), 'display enumeration in capture dialog');
  else assert.equal(await capture.getByRole('button', { name: '全屏截图', exact: true }).isDisabled(), true);
  assert.equal(await capture.getByRole('button', { name: '区域截图', exact: true }).isEnabled(), desktop);
  assert.equal(await page.evaluate(() => !!document.activeElement?.closest('dialog')), true);
  await page.keyboard.press('Escape');
  assert.equal(await page.getByRole('dialog').count(), 0);
  assert.equal(await toolbar.getByRole('button', { name: '新建截图', exact: true }).evaluate((el) => el === document.activeElement), true);
  await toolbar.getByRole('button', { name: '搜索素材', exact: true }).click();
  const search = page.getByRole('dialog', { name: '搜索素材与笔记' });
  await search.waitFor();
  const input = search.getByRole('textbox', { name: '搜索关键词' });
  assert.equal(await input.evaluate((el) => el === document.activeElement), true);
  await input.fill('蛋白质组');
  if(desktop)await search.getByRole('status').waitFor();
  else await search.getByText('请在桌面应用中搜索真实素材，浏览器预览不创建示例资料。', { exact: false }).waitFor();
  await search.getByRole('button', { name: '清空搜索' }).click();
  assert.equal(await input.inputValue(), '');
  await page.keyboard.press('Escape');
  await page.keyboard.press('Control+k');
  await search.waitFor();
  // Native dialog should keep keyboard focus within its contents.
  for (let i = 0; i < 5; i += 1) await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(() => !!document.activeElement?.closest('dialog')), true);
  await search.getByRole('button', { name: '关闭对话框' }).click();
  report.checks.push(`${prefix}capture controls, search input/clear, Ctrl+K, Escape and modal focus`);

  await toolbar.getByRole('button', { name: '切换为浅色主题' }).click();
  await until(async () => await theme() === 'light', 'light theme');
  await nav.getByRole('button', { name: '设置', exact: true }).click();
  assert.equal(await page.getByRole('group', { name: '主题选择' }).getByRole('button', { name: '浅色' }).getAttribute('aria-pressed'), 'true');
  await page.getByRole('group', { name: '主题选择' }).getByRole('button', { name: '深色' }).click();
  await until(async()=>await theme()==='dark','persisted dark theme');
  await page.getByRole('group', { name: '默认素材视图' }).getByRole('button', { name: '网格', exact: true }).click();
  await until(async()=>await toolbar.getByRole('button',{name:'网格视图'}).getAttribute('aria-pressed')==='true','persisted grid view');
  const button = page.getByRole('button', { name: '检查桌面连接', exact: true });
  if (desktop) {
    assert.equal(await button.isEnabled(), true);
    await button.click();
    await page.getByRole('status').getByText('ResearchClip：Rust 后端连接成功。', { exact: true }).waitFor();
    await button.click();
    await page.getByRole('status').getByText('ResearchClip：Rust 后端连接成功。', { exact: true }).waitFor();
    assert.equal(await button.isEnabled(), true);
    await page.waitForTimeout(300);
    assert.equal(await page.getByText('系统标题栏主题同步失败，应用内主题仍可使用。', { exact: true }).count(), 0);
    report.checks.push(`${prefix}real Rust command response, repeated click and window theme permission`);
  } else {
    assert.equal(await button.isDisabled(), true);
    await page.getByText('请在桌面应用中检查后端连接。', { exact: true }).waitFor();
    assert.equal(await page.getByText('ResearchClip：Rust 后端连接成功。', { exact: true }).count(), 0);
    report.checks.push('preview disables native action without a fake success message');
  }
  await page.getByRole('button', { name: '使用指南', exact: true }).click();
  await toolbar.getByRole('button', { name: '切换为浅色主题' }).click();
  await screenshot(`${label}${suffix}-light.png`);
  report.checks.push(`${prefix}light/dark and settings keep toolbar view selection in sync`);

  if (desktop) {
    if (!regionOnly) await checkNativeCapture(page, suffix);
    await checkNativeRegion(page, suffix);
  }

  if (!desktop) {
    for (const viewport of [{ width: 480, height: 400 }, { width: 640, height: 500 }, { width: 960, height: 680 }, { width: 1440, height: 900 }]) {
      await page.setViewportSize(viewport);
      for (const targetTheme of ['light', 'dark']) {
        if (await theme() !== targetTheme) await toolbar.getByRole('button', { name: targetTheme === 'dark' ? '切换为深色主题' : '切换为浅色主题' }).click();
        for (const view of ['截图素材', '收藏', '标签', '设置']) {
          await nav.getByRole('button', { name: view, exact: true }).click();
          assert.equal(await page.evaluate(() => {
            const panels = ['.app-sidebar', '.app-toolbar', '.workspace-scroll', '.workspace-footer'];
            return document.body.scrollWidth <= innerWidth && document.body.scrollHeight <= innerHeight && panels.every((selector) => {
              const box = document.querySelector(selector).getBoundingClientRect();
              return box.x >= 0 && box.y >= 0 && box.right <= innerWidth + 1 && box.bottom <= innerHeight + 1;
            }) && document.querySelector('.workspace-scroll').scrollWidth <= document.querySelector('.workspace-scroll').clientWidth;
          }), true, `Layout overflow at ${viewport.width}x${viewport.height}, ${targetTheme}, ${view}`);
        }
      }
      await nav.getByRole('button', { name: '截图素材', exact: true }).click();
      await screenshot(`${label}-${viewport.width}x${viewport.height}.png`);
    }
    report.checks.push('32 responsive page/theme cases at 480x400, 640x500, 960x680 and 1440x900');
  }
  assert.equal(await page.locator('canvas').count(), 0);
  assert.equal(await page.evaluate(() => JSON.stringify({ local: { ...localStorage }, session: { ...sessionStorage } })), sessionStorageBefore);
  report.checks.push(`${prefix}no leaked canvas on idle pages or browser storage writes`);
  if (desktop) {
    assert.equal(commands.includes('test_connection'), true, 'IPC observation must capture real test calls');
    assert.equal(commands.includes('plugin:window|set_theme'), true, 'IPC observation must capture theme calls');
    assert.equal(commands.includes('get_workspace_status'), false);
    const unexpected = commands.filter((command) => !['plugin:app|version', 'plugin:window|set_theme', 'test_connection', 'list_displays', 'capture_fullscreen', 'capture_region', 'region_result_png','capture_result_png','current_capture','discard_capture','get_preferences','update_preferences','plugin:event|listen','plugin:event|unlisten','library_status','list_screenshots','screenshot_page','screenshot_image','list_tags'].includes(command));
    assert.deepEqual(unexpected, ['region_overlay_info'], 'Only the explicitly rejected main-to-overlay security probe may be outside the normal UI command list');
    report.checks.push(`${prefix}UI interactions do not call database diagnostics; observed: ${[...new Set(commands)].join(', ')}`);
  }
}

async function nativeRun(suffix) {
  const endpoint = 'http://127.0.0.1:9333';
  let occupied = false;
  try { await fetch(`${endpoint}/json/version`); occupied = true; } catch { /* free */ }
  assert.equal(occupied, false, 'Debug port 9333 is occupied');
  const exe = join(root, 'src-tauri', 'target', 'debug', 'researchclip.exe');
  assert.equal(existsSync(exe), true, 'Run npm run tauri -- build --debug --no-bundle first');
  const profile = join(output, runId ? `${label}-webview-profile` : 'region-capture-webview-profile');
  await mkdir(nativeTemp, { recursive: true });
  const child = spawn(exe, [], {
    cwd: root, windowsHide: true, stdio: 'pipe',
    env: { ...process.env,
      TEMP: nativeTemp, TMP: nativeTemp, RESEARCHCLIP_TEST_SETTINGS_DIR: join(nativeTemp, 'settings'),
      WEBVIEW2_USER_DATA_FOLDER: profile,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9333 --remote-debugging-address=127.0.0.1',
    },
  });
  appPid=child.pid;
  let startupError;
  child.on('error', (error) => { startupError = error; });
  child.stdout.on('data', (data) => { report.nativeStdout = `${report.nativeStdout ?? ''}${data}`; });
  child.stderr.on('data', (data) => { report.nativeStderr = `${report.nativeStderr ?? ''}${data}`; });
  let nativeBrowser;
  let mainPage;
  try {
    await until(async () => {
      if (startupError) throw startupError;
      if (child.exitCode !== null) throw new Error(`Native app exited with ${child.exitCode}`);
      return (await fetch(`${endpoint}/json/version`)).ok;
    }, 'WebView2 startup');
    nativeBrowser = await chromium.connectOverCDP(endpoint);
    const page = await until(async () => {
      if (child.exitCode !== null) throw new Error(`Native app exited with ${child.exitCode}`);
      return nativeBrowser.contexts().flatMap((context) => context.pages()).find((item) => item.url().includes('tauri.localhost') || item.url().startsWith('tauri://'));
    }, 'Tauri main webview');
    mainPage = page;
    observe(page);
    await checkUI(page, suffix);
    assert.equal(existsSync(profile), true);
    report.checks.push(`${suffix ? 'restart: ' : ''}isolated WebView2 test profile`);
  } catch (error) {
    if (mainPage && !mainPage.isClosed()) {
      if (!currentPage || currentPage.isClosed()) currentPage = mainPage;
    }
    if (currentPage && !currentPage.isClosed()) {
      report.rendererFailure = await currentPage.locator('body').innerText();
      await currentPage.screenshot({ path: join(output, `${label}-failure.png`) });
      // Diagnostic only: leave the native acceptance FAILED, but exercise region failure cleanup.
      if (report.rendererFailure.includes('Windows 拒绝访问')) {
        report.regionAccessDiagnostic = await currentPage.evaluate(async () => {
          const invoke = window.__TAURI_INTERNALS__.invoke;
          const displays = await invoke('list_displays');
          const displayId = (displays.find((d) => d.isPrimary) ?? displays[0]).id;
          try { await invoke('capture_region', { displayId }); return { result: 'unexpected-success' }; }
          catch (failure) { return { code: failure.code, message: failure.message, mainFocused: document.hasFocus() }; }
        });
        report.regionAccessDiagnostic.overlayCount = nativeBrowser.contexts().flatMap((context) => context.pages()).filter((item) => item.url().includes('captureOverlay=')).length;
      }
    }
    throw error;
  } finally {
    if (nativeBrowser) await nativeBrowser.close();
    if (child.exitCode === null) {
      const stopped = new Promise((resolve) => child.once('exit', resolve));
      child.kill();
      await Promise.race([stopped, delay(5000)]);
    }
    await until(async () => {
      try { await fetch(`${endpoint}/json/version`); return false; } catch { return true; }
    }, 'WebView2 shutdown', 10000);
  }
}

try {
  if (desktop) {
    assert.equal(process.platform, 'win32', 'Native smoke is currently Windows-only');
    // This pattern is a real, visible Windows browser surface. It is never passed to the capture API.
    fixtureBrowser = await chromium.launch({ channel: 'msedge', headless: false, args: ['--window-position=0,0'] });
    fixturePage = await fixtureBrowser.newPage({ viewport: null });
    const fixtureCdp = await fixturePage.context().newCDPSession(fixturePage);
    const { windowId } = await fixtureCdp.send('Browser.getWindowForTarget');
    const size = await fixturePage.evaluate(() => ({ width: screen.width, height: screen.height }));
    await fixtureCdp.send('Browser.setWindowBounds', { windowId, bounds: { windowState: 'normal' } });
    await fixtureCdp.send('Browser.setWindowBounds', { windowId, bounds: { left: 0, top: 0, ...size } });
    await fixtureCdp.send('Browser.setWindowBounds', { windowId, bounds: { windowState: 'fullscreen' } });
    await until(() => fixturePage.evaluate(() => innerWidth === screen.width && innerHeight === screen.height), 'test fixture covers the full primary screen');
    await fixturePage.setContent(`<html><head><title>ResearchClip capture acceptance fixture</title><style>
      html,body{margin:0;width:100%;height:100%;overflow:hidden}body{display:grid;grid-template-columns:1fr 1fr;grid-template-rows:1fr 1fr}
      .a{background:rgb(232,76,76)}.b{background:rgb(75,185,132)}.c{background:rgb(80,120,232)}.d{background:rgb(232,185,65)}
      body[data-phase="2"] .a{background:rgb(75,185,132)}body[data-phase="2"] .b{background:rgb(232,76,76)}body[data-phase="2"] .c{background:rgb(232,185,65)}body[data-phase="2"] .d{background:rgb(80,120,232)}
      .center{position:fixed;left:40%;top:40%;width:20%;height:20%;background:rgb(245,245,245);display:flex;align-items:center;justify-content:center;font:20px sans-serif;color:#202020}
    </style></head><body data-phase="1"><div class="a"></div><div class="b"></div><div class="c"></div><div class="d"></div><div class="center"><span style="transform:translateY(50px)">ResearchClip · real screen test</span></div></body></html>`);
    const browserCdp = await fixtureBrowser.newBrowserCDPSession();
    const { processInfo } = await browserCdp.send('SystemInfo.getProcessInfo');
    const fixtureProcess = processInfo.find((item) => item.type === 'browser');
    assert.ok(fixtureProcess && Number.isInteger(fixtureProcess.id) && fixtureProcess.id > 0);
    fixturePid=fixtureProcess.id;
    const raiseFixture = join(root, 'src-tauri', 'target', 'debug', 'examples', 'raise_capture_fixture.exe');
    assert.ok(existsSync(raiseFixture), 'Build the test helper with cargo build --manifest-path src-tauri/Cargo.toml --locked --example raise_capture_fixture');
    await until(() => {
      execFileSync(raiseFixture, [String(fixtureProcess.id)], { cwd: root, windowsHide: true, stdio: 'pipe' });
      return true;
    }, 'native fixture title ready', 5000);
    report.fixtureTopmost = 'test browser PID and title verified; no other windows changed';
    report.fixtureViewport = await fixturePage.evaluate(() => ({ width: innerWidth, height: innerHeight, dpr: devicePixelRatio, screenWidth: screen.width, screenHeight: screen.height }));
    await nativeRun('');
    await nativeRun('-restart');
  } else {
    server = spawn(process.execPath, [join(root, 'node_modules', 'vite', 'bin', 'vite.js'), 'preview', '--host', '127.0.0.1', '--port', '1421', '--strictPort'], { cwd: root, stdio: 'pipe', windowsHide: true });
    let serverError;
    server.on('error', (error) => { serverError = error; });
    server.stdout.on('data', () => {});
    server.stderr.on('data', () => {});
    await until(async () => {
      if (serverError) throw serverError;
      if (server.exitCode !== null) throw new Error(`Preview server exited with ${server.exitCode}`);
      return (await fetch('http://127.0.0.1:1421')).ok;
    }, 'Vite production preview');
    browser = await chromium.launch({ channel: 'msedge', headless: true });
    const page = await browser.newPage({ viewport: { width: 960, height: 680 } });
    observe(page);
    await page.goto('http://127.0.0.1:1421');
    await checkUI(page);
    report.overlayRenderer = await checkOverlayRenderer(browser, observe, output, label);
  }
  assert.deepEqual(report.errors, [], 'Renderer emitted console/page errors');
  report.result = 'passed';
  console.log(JSON.stringify(report, null, 2));
} catch (error) {
  report.result = 'failed';
  report.failure = error.stack ?? String(error);
  if (currentPage && !currentPage.isClosed()) await currentPage.screenshot({ path: join(output, `${label}-failure.png`) }).catch(() => {});
  console.error(JSON.stringify(report, null, 2));
  process.exitCode = 1;
} finally {
  if (browser) await browser.close();
  if (fixtureBrowser) await fixtureBrowser.close();
  if (server && server.exitCode === null) server.kill();
  await writeFile(join(output, `${label}-smoke.json`), JSON.stringify(report, null, 2));
}
