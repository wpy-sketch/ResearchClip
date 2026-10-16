import { describe, expect, it } from 'vitest';
import { selectionToPixels } from './geometry';

describe('overlay local CSS coordinates to frozen physical pixels', () => {
  it.each([1, 1.25, 1.5, 2])('supports scale %s and reverse dragging', (scale) => {
    const viewport = { width: 1920 / scale, height: 1080 / scale };
    const image = { width: 1920, height: 1080 };
    const start = { x: 10 / scale, y: 20 / scale }, end = { x: 110 / scale, y: 220 / scale };
    expect(selectionToPixels(start, end, viewport, image)).toEqual({ x: 10, y: 20, width: 100, height: 200 });
    expect(selectionToPixels(end, start, viewport, image)).toEqual(selectionToPixels(start, end, viewport, image));
  });
  it('rounds outward and clamps both ends to window boundaries', () => {
    expect(selectionToPixels({ x: -3, y: 2.2 }, { x: 2000, y: 30.1 }, { width: 800, height: 600 }, { width: 1000, height: 750 })).toEqual({ x: 0, y: 2, width: 1000, height: 36 });
  });
  it('maps non-integral logical viewport sizes using actual image ratios', () => {
    expect(selectionToPixels({ x: 0, y: 0 }, { x: 1707, y: 960 }, { width: 1707, height: 960 }, { width: 2560, height: 1440 })).toEqual({ x: 0, y: 0, width: 2560, height: 1440 });
  });
  it('does not add a virtual desktop origin to monitor-local coordinates', () => {
    // The native overlay may be at (-2560,-200). Its client coordinates still start at (0,0).
    expect(selectionToPixels({ x: 20, y: 30 }, { x: 80, y: 90 }, { width: 800, height: 600 }, { width: 1600, height: 1200 })).toEqual({ x: 40, y: 60, width: 120, height: 120 });
  });
  it('keeps zero-area gestures retryable, including fully out-of-window gestures', () => {
    const viewport = { width: 800, height: 600 };
    expect(selectionToPixels({ x: 1.1, y: 2 }, { x: 1.1, y: 30 }, viewport, viewport)).toBeNull();
    expect(selectionToPixels({ x: -10, y: 2 }, { x: -1, y: 30 }, viewport, viewport)).toBeNull();
  });
  it('rejects invalid coordinates, viewport and image limits', () => {
    const viewport = { width: 800, height: 600 };
    expect(() => selectionToPixels({ x: NaN, y: 0 }, { x: 10, y: 10 }, viewport, viewport)).toThrow();
    expect(() => selectionToPixels({ x: 0, y: 0 }, { x: 10, y: 10 }, { width: 0, height: 600 }, viewport)).toThrow();
    expect(() => selectionToPixels({ x: 0, y: 0 }, { x: 10, y: 10 }, viewport, { width: 8192, height: 8192 })).toThrow();
  });
});
