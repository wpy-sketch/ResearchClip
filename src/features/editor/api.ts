import { invoke, isTauri } from '@tauri-apps/api/core';
import { parsePngSize } from '../../shared/api/capture';
import { renderComposite } from './export';
import { packImage, unpackProject } from './serialization';
import type { EditorSession } from './types';
function desktop() { if (!isTauri()) throw new Error('项目文件和图片剪贴板操作需要在桌面应用中使用。'); }
export async function saveProject(session: EditorSession): Promise<string | null> {
  desktop(); return invoke<string | null>('save_editor_project', await packImage(session.image.blob, session.document));
}
export async function openProject() {
  desktop(); const data = unpackProject(await invoke<ArrayBuffer>('open_editor_project')); if (!data) return null;
  const size = parsePngSize(await data.blob.arrayBuffer());
  if (size.widthPx !== data.project.document.image.widthPx || size.heightPx !== data.project.document.image.heightPx) throw new Error('项目原图尺寸与标注不一致。');
  const bitmap = await createImageBitmap(data.blob);
  try { if (bitmap.width !== size.widthPx || bitmap.height !== size.heightPx) throw new Error('项目原图解码失败。'); } finally { bitmap.close(); }
  return { ...data, ...size };
}
export async function exportImage(session: EditorSession, action: 'save' | 'copy'): Promise<string | null> {
  desktop(); const composite = await renderComposite(session.image.blob, session.document);
  const bytes = await packImage(composite, session.document);
  if (action === 'copy') { await invoke('copy_editor_png', bytes); return '合成图片已复制，可粘贴到图像应用。'; }
  return invoke<string | null>('export_editor_png', bytes);
}
