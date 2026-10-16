import assert from 'node:assert/strict';
import { join } from 'node:path';

// Renderer-only tests with explicit mock IPC and a generated test PNG.
// These checks cannot verify native capture, window placement/focus/closing, or disk saving.
export async function checkOverlayRenderer(browser, observe, output, label) {
  const checks = [];
  for (const scale of [1, 1.25, 1.5, 2]) {
    const viewport = { width: Math.round(1600 / scale), height: Math.round(900 / scale) };
    const context = await browser.newContext({ viewport, deviceScaleFactor: scale });
    try {
      const page = await context.newPage();
      observe(page);
      const png = await page.evaluate(async () => {
        const canvas = new OffscreenCanvas(1600, 900);
        const drawing = canvas.getContext('2d');
        drawing.fillStyle = '#4d6b9e'; drawing.fillRect(0, 0, 1600, 900);
        const bytes = new Uint8Array(await (await canvas.convertToBlob({ type: 'image/png' })).arrayBuffer());
        return Array.from(bytes);
      });
      await page.addInitScript(({ bytes }) => {
        window.isTauri = true;
        window.__overlayTestCalls = [];
        window.__TAURI_INTERNALS__ = {
          invoke: async (command, args) => {
            window.__overlayTestCalls.push({ command, args });
            if (command === 'region_overlay_info') return { sessionId: 'renderer-test', display: { id: 'mock-display', name: 'mock-display', widthPx: 1600, heightPx: 900, xPx: -1600, yPx: -200, isPrimary: true } };
            if (command === 'region_overlay_image') return Uint8Array.from(bytes).buffer;
            if (['ready_region_overlay', 'commit_region_selection', 'cancel_region_selection'].includes(command)) return;
            throw new Error(`Unexpected mock IPC: ${command}`);
          },
        };
      }, { bytes: png });
      const url = 'http://127.0.0.1:1421/?captureOverlay=renderer-test';
      await page.goto(url);
      await page.locator('.capture-overlay[data-ready="true"]').waitFor();
      assert.equal(await page.getByRole('navigation').count(), 0, 'Overlay entry must not initialize the main app');
      assert.deepEqual(await page.evaluate(() => window.__overlayTestCalls.map((call) => call.command)), ['region_overlay_info', 'region_overlay_image', 'ready_region_overlay']);
      assert.equal(await page.evaluate(() => document.body.scrollWidth <= innerWidth && document.body.scrollHeight <= innerHeight), true);
      await page.mouse.click(20, 20);
      await page.getByRole('status').getByText('选区没有面积', { exact: false }).waitFor();
      const start = { x: Math.floor(viewport.width * .75), y: Math.floor(viewport.height * .75) };
      const end = { x: Math.floor(viewport.width * .25), y: Math.floor(viewport.height * .25) };
      await page.mouse.move(start.x, start.y); await page.mouse.down();
      await page.mouse.move(end.x, end.y, { steps: 5 });
      const width = Math.ceil(start.x * 1600 / viewport.width) - Math.floor(end.x * 1600 / viewport.width);
      const height = Math.ceil(start.y * 900 / viewport.height) - Math.floor(end.y * 900 / viewport.height);
      assert.equal(await page.locator('.selection-size').innerText(), `${width} × ${height} px`);
      assert.equal(await page.locator('.selection-dim').count(), 4);
      if (scale === 1.25) await page.screenshot({ path: join(output, `${label}-overlay-ui-mock.png`) });
      await page.mouse.up();
      await page.locator('.capture-overlay[data-pending="true"]').waitFor();
      const commit = await page.evaluate(() => window.__overlayTestCalls.filter((call) => call.command === 'commit_region_selection'));
      assert.equal(commit.length, 1);
      assert.deepEqual(commit[0].args, { sessionId: 'renderer-test', selection: { start, end, viewportWidth: viewport.width, viewportHeight: viewport.height } });
      await page.keyboard.press('Escape');
      assert.equal(await page.evaluate(() => window.__overlayTestCalls.filter((call) => call.command === 'cancel_region_selection').length), 0, 'A submitted selection must not be cancelled late');
      await page.reload();
      await page.locator('.capture-overlay[data-ready="true"]').waitFor();
      await page.keyboard.press('Escape');
      await page.locator('.capture-overlay[data-pending="true"]').waitFor();
      assert.deepEqual(await page.evaluate(() => window.__overlayTestCalls.at(-1)), { command: 'cancel_region_selection', args: { sessionId: 'renderer-test', reason: 'user' } });
      checks.push(`mock IPC only, scale ${scale}: isolated entry, one-shot frame, zero area, reverse drag, live pixels, four masks, single commit, Esc`);
    } finally { await context.close(); }
  }
  return { mode: 'renderer-only-mock', nativeCaptureVerified: false, checks };
}
