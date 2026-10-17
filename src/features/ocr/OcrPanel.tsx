import {useRef,useState} from 'react';
import {Modal} from '../../shared/ui/Modal';
import {useDocumentStore} from '../document/store';
import {useOcrStore} from './store';
import {regionFromDrag} from './model';
export function OcrPanel(){const key=useOcrStore(s=>s.source?.url??'none');return <OcrSession key={key}/>;}
function OcrSession(){
  const s=useOcrStore(),document=useDocumentStore(d=>d.document),documentPhase=useDocumentStore(d=>d.phase);
  const [selecting,setSelecting]=useState(false),[replace,setReplace]=useState(false),[inserted,setInserted]=useState(false),[imageFailed,setImageFailed]=useState(false);
  const drag=useRef<{x:number;y:number}|null>(null),image=useRef<HTMLImageElement>(null);
  const busy=s.phase!=='idle';
  if(!s.source)return s.error?<p role="alert" className="error-banner">{s.error}</p>:s.phase==='preparing'?<p role="status" className="p-2 text-sm text-muted">正在准备 OCR 图片…</p>:null;
  const source=s.source,region=s.region;
  const point=(event:React.PointerEvent)=>{const b=image.current!.getBoundingClientRect();return{x:event.clientX-b.left,y:event.clientY-b.top};};
  const update=(event:React.PointerEvent)=>{if(!drag.current||!image.current)return;const b=image.current.getBoundingClientRect();s.setRegion(regionFromDrag(drag.current,point(event),{width:b.width,height:b.height},source));};
  const run=()=>{setReplace(false);setInserted(false);void s.run();};
  return <Modal title="离线文字识别" onClose={()=>{if(!busy)s.close();}}><div className="ocr-content p-5">
    <p className="mb-3 text-xs text-muted">{source.name} · 本地处理。结果仅保留在当前面板；关闭前请复制或插入。复杂公式、上下标与 LaTeX 不保证准确，务必对照原图校对。</p>
    <div className="ocr-grid"><div className="min-w-0"><div className="ocr-image" style={{width:`${source.widthPx/source.heightPx*30}vh`}} data-selecting={selecting} onPointerDown={event=>{if(!selecting||busy||event.button!==0||!image.current)return;event.preventDefault();drag.current=point(event);s.setRegion(null);event.currentTarget.setPointerCapture(event.pointerId);}} onPointerMove={update} onPointerUp={event=>{if(!drag.current)return;update(event);drag.current=null;event.currentTarget.releasePointerCapture(event.pointerId);}} onPointerCancel={()=>{drag.current=null;s.setRegion(null);}} onLostPointerCapture={()=>{drag.current=null;}}>
      <img ref={image} src={source.url} alt="OCR 待识别图片" draggable={false} onError={()=>setImageFailed(true)}/>
      {region&&<div className="ocr-selection" style={{left:`${region.x/source.widthPx*100}%`,top:`${region.y/source.heightPx*100}%`,width:`${region.width/source.widthPx*100}%`,height:`${region.height/source.heightPx*100}%`}}/>}
    </div><div className="mt-3 flex flex-wrap gap-2"><button className="secondary-button" disabled={busy} aria-pressed={!selecting} onClick={()=>{drag.current=null;setSelecting(false);s.setRegion(null);}}>整张图片</button><button className="secondary-button" disabled={busy||imageFailed} aria-pressed={selecting} onClick={()=>setSelecting(true)}>框选区域</button></div>
      <p className="mt-2 text-xs text-muted">{selecting?(region?`选区 ${region.width} × ${region.height} 原图像素`:'请在图片上拖动选区；零面积不会开始识别。'):`整图 ${source.widthPx} × ${source.heightPx} 像素`}</p>
      <label className="mt-4 block text-xs text-muted">识别语言<select aria-label="OCR 语言" className="mt-2 w-full rounded border border-edge bg-canvas p-2 text-primary" value={s.language} disabled={busy||!s.status?.languages.length} onChange={e=>s.setLanguage(e.target.value)}>{s.status?.languages.map(l=><option key={l.tag} value={l.tag}>{l.name} · {l.tag}</option>)}</select></label>
      <p className="mt-2 text-xs text-muted">仅列出本机已安装的 OCR 语言。英文组件缺失时可先尝试中文引擎；识别质量需校对。{s.status&&` 单边上限 ${s.status.maxDimension.toLocaleString()} 像素。`}</p>
      <button className="capture-button mt-4" disabled={busy||imageFailed||!s.language||!s.status?.languages.length||(selecting&&!region)} onClick={()=>{if(s.text!==s.recognizedText)setReplace(true);else run();}}>{s.phase==='recognizing'?'正在识别…':'开始识别'}</button>
      {replace&&<div className="mt-3 text-sm" role="alert">重新识别会替换已修改的结果。<div className="mt-2 flex gap-2"><button className="secondary-button" onClick={run}>确认重新识别</button><button className="secondary-button" onClick={()=>setReplace(false)}>保留编辑</button></div></div>}
    </div><div className="flex min-w-0 flex-col"><label className="text-sm" htmlFor="ocr-result">识别结果（可修改）</label><textarea id="ocr-result" aria-label="OCR 识别结果" className="ocr-result mt-2 rounded-lg border border-edge bg-canvas p-3 text-sm" value={s.text} spellCheck={false} disabled={busy} onChange={e=>{s.edit(e.target.value);setInserted(false);}} placeholder="识别结果会显示在这里…"/>
      <div className="mt-3 flex flex-wrap gap-2"><button className="secondary-button" disabled={busy||!s.text.trim()} onClick={()=>void s.copy()}>复制识别文字</button><button className="secondary-button" disabled={busy||!s.text.trim()||!document||documentPhase!=='idle'} onClick={()=>{if(useDocumentStore.getState().insertText(s.text)){setInserted(true);useOcrStore.setState({error:null});}else useOcrStore.setState({error:useDocumentStore.getState().error??'插入失败，请检查 Markdown 文档状态。'});}}>插入到 Markdown</button></div>
      <p className="mt-2 text-xs text-muted">{inserted?'已插入当前 Markdown 光标位置，可在文档中撤销。':document?'手动确认后插入，不会自动写入笔记；图文编排模式会在末尾新增文本块。':'请先新建或打开 Markdown 文档，再识别并插入。'}</p>
    </div></div>
    {imageFailed&&<p role="alert" className="error-banner mt-3">图片预览加载失败，请关闭面板后重试。</p>}{s.error&&<p role="alert" className="error-banner mt-3">{s.error}</p>}{s.notice&&<p role="status" className="mt-3 text-sm text-secondary">{s.notice}</p>}
    {busy&&<p role="status" className="mt-3 text-xs text-muted">正在本地处理，请等待完成后关闭面板。没有上传图片或识别文字。</p>}
  </div></Modal>;
}
