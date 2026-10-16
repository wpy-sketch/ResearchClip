import type { Point, Viewport } from './types';
export const MAX_IMAGE_PIXELS = 16_777_216;
// Even the widest accepted image must fit in a one-CSS-pixel host with half-pixel margins.
export const MIN_SCALE = 1 / (2 * MAX_IMAGE_PIXELS); export const MAX_SCALE = 8;
export function validateImageSize(width: number, height: number) {
  if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0 || width * height > MAX_IMAGE_PIXELS) throw new Error('图片尺寸无效或超出 16,777,216 像素上限。');
}
export function fitViewport(width: number, height: number, availableWidth: number, availableHeight: number): Viewport {
  validateImageSize(width, height);
  if (!Number.isFinite(availableWidth) || !Number.isFinite(availableHeight) || availableWidth <= 0 || availableHeight <= 0) throw new Error('画布窗口尺寸无效。');
  const scale = Math.min(MAX_SCALE, Math.max(MIN_SCALE, Math.min((availableWidth - Math.min(48, availableWidth / 2)) / width, (availableHeight - Math.min(48, availableHeight / 2)) / height, 1)));
  return { scale, x: (availableWidth - width * scale) / 2, y: (availableHeight - height * scale) / 2, mode: 'fit' };
}
function validateTransform(point: Point, view: Viewport) {
  if (!Number.isFinite(view.scale) || view.scale <= 0 || ![point.x, point.y, view.x, view.y].every(Number.isFinite)) throw new Error('画布坐标无效。');
}
export function screenToImage(point: Point, view: Viewport): Point {
  validateTransform(point, view);
  return { x: (point.x - view.x) / view.scale, y: (point.y - view.y) / view.scale };
}
export function imageToScreen(point: Point, view: Viewport): Point { validateTransform(point, view); return { x: point.x * view.scale + view.x, y: point.y * view.scale + view.y }; }
export function zoomViewport(view: Viewport, scale: number, anchor: Point): Viewport {
  if (!Number.isFinite(scale)) throw new Error('缩放值无效。');
  const next = Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale)); const image = screenToImage(anchor, view);
  return { x: anchor.x - image.x * next, y: anchor.y - image.y * next, scale: next, mode: 'manual' };
}
export function clampPoint(point: Point, width: number, height: number): Point { return { x: Math.max(0, Math.min(width, point.x)), y: Math.max(0, Math.min(height, point.y)) }; }
