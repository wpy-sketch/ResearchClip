export interface Point { x: number; y: number }
export interface Size { width: number; height: number }
export interface RectPx { x: number; y: number; width: number; height: number }

export function selectionToPixels(start: Point, end: Point, viewport: Size, image: Size): RectPx | null {
  const values = [start.x, start.y, end.x, end.y, viewport.width, viewport.height, image.width, image.height];
  if (!values.every(Number.isFinite) || viewport.width <= 0 || viewport.height <= 0 || !Number.isInteger(image.width) || !Number.isInteger(image.height) || image.width <= 0 || image.height <= 0 || image.width * image.height > 16_777_216) throw new Error('选区坐标或图像尺寸无效。');
  const clamp = (n: number, max: number) => Math.min(max, Math.max(0, n));
  const x1 = clamp(start.x, viewport.width), x2 = clamp(end.x, viewport.width);
  const y1 = clamp(start.y, viewport.height), y2 = clamp(end.y, viewport.height);
  if (x1 === x2 || y1 === y2) return null;
  const sx = image.width / viewport.width, sy = image.height / viewport.height;
  const x = clamp(Math.floor(Math.min(x1, x2) * sx), image.width);
  const y = clamp(Math.floor(Math.min(y1, y2) * sy), image.height);
  const right = clamp(Math.ceil(Math.max(x1, x2) * sx), image.width);
  const bottom = clamp(Math.ceil(Math.max(y1, y2) * sy), image.height);
  return right === x || bottom === y ? null : { x, y, width: right - x, height: bottom - y };
}
