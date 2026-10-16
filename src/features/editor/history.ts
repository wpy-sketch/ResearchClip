import type { Annotation, EditHistory } from './types';
export const HISTORY_LIMIT = 100;
export const HISTORY_BYTES = 8 * 1024 * 1024;
// Only vectors are retained; original images and view state never enter history.
export function boundHistory(history: EditHistory): EditHistory {
  const past = [...history.past], future = [...history.future];
  const bytes = (items: Annotation[][]) => items.reduce((sum, item) => sum + new TextEncoder().encode(JSON.stringify(item)).length, 0);
  while (past.length + future.length > HISTORY_LIMIT || bytes(past) + bytes(future) > HISTORY_BYTES) {
    if (past.length) past.shift(); else if (future.length) future.shift(); else break;
  }
  return { past, future };
}
export function recordEdit(history: EditHistory, before: Annotation[], after: Annotation[]): EditHistory {
  return JSON.stringify(before) === JSON.stringify(after) ? history : boundHistory({ past: [...history.past, before], future: [] });
}
