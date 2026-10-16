import { describe, expect, it, vi } from 'vitest';
import { attachListener } from './use-workflow';
describe('native capture listener lifecycle', () => {
  it('unlistens even if disposed before subscription resolves; drops late notifications', async () => {
    let resolve!: (value: () => void) => void; let notify!: (v: unknown) => void;
    const cleanup = vi.fn(); const handler = vi.fn();
    const dispose = attachListener((h) => { notify = h; return new Promise((r) => { resolve = r; }); }, handler, vi.fn());
    dispose(); notify({ kind: 'ready' }); resolve(cleanup); await Promise.resolve();
    expect(cleanup).toHaveBeenCalledOnce(); expect(handler).not.toHaveBeenCalled(); dispose(); expect(cleanup).toHaveBeenCalledOnce();
  });
  it('cleans up once and only reports subscription errors while mounted', async () => {
    const cleanup = vi.fn(); const handler = vi.fn(); const failed = vi.fn();
    const dispose = attachListener(async (h) => { h({ kind: 'started' }); return cleanup; }, handler, failed);
    await Promise.resolve(); expect(handler).toHaveBeenCalledOnce(); dispose(); dispose(); expect(cleanup).toHaveBeenCalledOnce();
    attachListener(() => Promise.reject(new Error('unavailable')), handler, failed); await Promise.resolve(); await Promise.resolve(); expect(failed).toHaveBeenCalledOnce();
  });
});
