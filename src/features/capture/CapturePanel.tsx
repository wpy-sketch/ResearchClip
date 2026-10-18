import { createPin } from '../pins/api';
import { errorMessage } from '../../shared/api/errors';
import { Camera, Pin, ClipboardCopy, Download, Pencil, Save, X } from 'lucide-react';
import { CaptureControls } from './CaptureControls';
import { useCaptureStore } from './store';
import { openCaptureEditor } from '../../app/open-editor';
import { useUiStore } from '../../app/ui-store';
import { saveCaptureToLibrary } from '../../app/library-actions';
import {OcrButton} from '../ocr/OcrButton';

export function CapturePanel() {
  const { preview, displays, phase, notice, clearPreview, previewFailed, copyImage, savePng } = useCaptureStore();
  const sourceIndex = displays.findIndex((display) => display.id === preview?.displayId);
  const busy = phase !== 'idle';
  return <section className={`capture-panel ${!preview?'is-compact':''}`} aria-labelledby="capture-title">
    <div className="capture-panel-heading mb-5 flex items-start justify-between gap-3">
      <div><h2 id="capture-title" className="text-sm font-medium">屏幕截图</h2><p className="mt-2 text-xs text-muted">截图预览为临时内容；点击“存入素材库”才会长期保留。</p></div>
      {preview && <button className="icon-button" aria-label="关闭截图预览" title="关闭截图预览" disabled={busy} onClick={clearPreview}><X size={16} /></button>}
    </div>
    <CaptureControls compact={!preview} />
    <button className="secondary-button mt-3" onClick={() => useUiStore.getState().navigate('editor')}><Pencil size={15} />图片编辑器 / 打开项目</button>
    {preview ? <>
      <figure className="mt-5 overflow-hidden rounded-lg border border-edge bg-canvas">
        <div className="capture-image-container"><img className="capture-image" src={preview.url} alt={preview.mode === 'scroll' ? '滚动长截图预览' : preview.mode === 'region' ? '区域截图预览' : '全屏截图预览'} onError={() => previewFailed(preview.url)} /></div>
        <figcaption className="border-t border-edge px-4 py-3 text-xs text-muted"><div className="flex flex-wrap justify-between gap-2"><span>{preview.widthPx} × {preview.heightPx} 像素</span><span>{sourceIndex >= 0 ? `显示器 ${sourceIndex + 1}` : '截图预览'} · {preview.savedPath ? 'PNG 已保存' : '临时预览 · 未保存'}</span></div>{preview.savedPath && <p className="mt-2 break-all select-text">保存位置：{preview.savedPath}</p>}</figcaption>
      </figure>
      <div className="mt-4 flex flex-wrap gap-2" role="group" aria-label="截图完成操作">
        <OcrButton disabled={busy||!preview.blob} name="截图预览" image={async()=>{if(!preview.blob)throw new Error('截图已失效，请重新截图。');return preview.blob;}}/>
        <button className="capture-button" disabled={busy||Boolean(preview.library)} onClick={()=>void saveCaptureToLibrary()}><Save size={15}/>{preview.library?'已存入素材库':'存入素材库'}</button>
        <button className="secondary-button" disabled={busy||!preview.blob} onClick={()=>{if(!preview.blob||phase!=='idle')return;useCaptureStore.setState({phase:'working',error:null,notice:null});void createPin(preview.blob).then(()=>useCaptureStore.setState({notice:'桌面贴图已创建，可拖动、滚轮缩放或右键操作。'})).catch(e=>useCaptureStore.setState({error:errorMessage(e,'创建贴图失败。')})).finally(()=>useCaptureStore.setState({phase:'idle'}));}}><Pin size={15}/>贴图</button>
        <button className="secondary-button" disabled={busy} onClick={() => void copyImage()}><ClipboardCopy size={15} />复制图片</button>
        <button className="capture-button" disabled={busy} onClick={() => void savePng()}><Download size={15} />保存 PNG</button>
        <button className="secondary-button" disabled={busy} onClick={() => { try { openCaptureEditor(preview); } catch (error) { useCaptureStore.setState({ error: error instanceof Error ? error.message : '编辑器无法打开。' }); } }}><Pencil size={15} />打开编辑器</button>
        <button className="secondary-button" disabled={busy} onClick={clearPreview}>取消</button>
      </div>
      {notice && <p className="mt-3 text-xs text-secondary" role="status">{notice}</p>}

    </> : <div className="capture-preview-empty"><Camera size={18} strokeWidth={1.5} /><p>选择显示器，开始全屏或区域截图</p></div>}
  </section>;
}
