import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium } from 'playwright-core';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const desktop = process.argv.includes('--desktop');
const output = join(root, 'artifacts');
const version = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8')).version;
const label = desktop ? 'init-desktop' : 'init-preview';
const report = { mode: desktop ? 'desktop' : 'preview', version, checks: [], errors: [] };
await mkdir(output, { recursive: true });
let currentPage;
let browser;
let server;

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
  page.on('console', (message) => { if (message.type() === 'error') report.errors.push(message.text()); });
}

async function checkUI(page, suffix = '') {
  currentPage = page;
  const prefix = suffix ? 'restart: ' : '';
  await page.getByRole('heading', { name: 'ResearchClip', exact: true }).waitFor();
  await page.getByRole('heading', { name: '欢迎使用 ResearchClip', exact: true }).waitFor();
  await page.locator('footer').getByText(`版本 ${version}${desktop ? '' : ' · 浏览器预览'}`, { exact: true }).waitFor();
  const button = page.getByRole('button', { name: '测试 Rust 通信', exact: true });
  if (desktop) {
    assert.equal(await button.isEnabled(), true);
    await button.click();
    await page.getByRole('status').getByText('ResearchClip：Rust 后端连接成功。', { exact: true }).waitFor();
    await button.click();
    await page.getByRole('status').getByText('ResearchClip：Rust 后端连接成功。', { exact: true }).waitFor();
    assert.equal(await button.isEnabled(), true);
    report.checks.push(`${prefix}real Rust command response and repeated click`);
  } else {
    assert.equal(await button.isDisabled(), true);
    await page.getByText('浏览器预览模式：请启动桌面应用测试 Rust 通信。', { exact: true }).waitFor();
    assert.equal(await page.getByText('ResearchClip：Rust 后端连接成功。', { exact: true }).count(), 0);
    for (const viewport of [{ width: 480, height: 400 }, { width: 960, height: 680 }]) {
      await page.setViewportSize(viewport);
      assert.equal(await page.evaluate(() => document.body.scrollWidth <= window.innerWidth), true);
    }
    report.checks.push('preview disables native action without a fake success message');
  }
  assert.equal(await page.locator('canvas').count(), 0);
  report.checks.push(`${prefix}header, welcome text, version, and initialization-only view`);
  await page.screenshot({ path: join(output, `${label}${suffix}.png`) });
}

async function nativeRun(suffix) {
  const endpoint = 'http://127.0.0.1:9333';
  let occupied = false;
  try { await fetch(`${endpoint}/json/version`); occupied = true; } catch { /* free */ }
  assert.equal(occupied, false, 'Debug port 9333 is occupied');
  const exe = join(root, 'src-tauri', 'target', 'release', 'researchclip.exe');
  assert.equal(existsSync(exe), true, 'Run npm run desktop:build first');
  const profile = join(output, 'init-webview-profile');
  const child = spawn(exe, [], {
    cwd: root, windowsHide: true, stdio: 'pipe',
    env: { ...process.env,
      WEBVIEW2_USER_DATA_FOLDER: profile,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9333 --remote-debugging-address=127.0.0.1',
    },
  });
  let startupError;
  child.on('error', (error) => { startupError = error; });
  child.stdout.on('data', (data) => { report.nativeStdout = `${report.nativeStdout ?? ''}${data}`; });
  child.stderr.on('data', (data) => { report.nativeStderr = `${report.nativeStderr ?? ''}${data}`; });
  let nativeBrowser;
  try {
    await until(async () => {
      if (startupError) throw startupError;
      if (child.exitCode !== null) throw new Error(`Native app exited with ${child.exitCode}`);
      return (await fetch(`${endpoint}/json/version`)).ok;
    }, 'WebView2 startup');
    nativeBrowser = await chromium.connectOverCDP(endpoint);
    const page = await until(async () => nativeBrowser.contexts().flatMap((context) => context.pages()).find((item) => item.url().includes('tauri.localhost') || item.url().startsWith('tauri://')), 'Tauri main webview');
    observe(page);
    await checkUI(page, suffix);
    assert.equal(existsSync(profile), true);
    report.checks.push(`${suffix ? 'restart: ' : ''}isolated WebView2 test profile`);
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
  if (server && server.exitCode === null) server.kill();
  await writeFile(join(output, `${label}-smoke.json`), JSON.stringify(report, null, 2));
}
