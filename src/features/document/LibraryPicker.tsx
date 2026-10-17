import {useEffect,useState} from 'react';
import {Modal} from '../../shared/ui/Modal';
import {readScreenshotPage,screenshotImage} from '../../shared/api/library';
import type {ScreenshotPage,ScreenshotSummary} from '../../shared/api/library';
function Thumbnail({item}:{item:ScreenshotSummary}){
  const [url,setUrl]=useState<string|null>(null),[failed,setFailed]=useState(false);
  useEffect(()=>{let active=true,owned:string|null=null;void screenshotImage(item.id,true).then(blob=>{if(active){owned=URL.createObjectURL(blob);setUrl(owned);}},()=>{if(active)setFailed(true);});return()=>{active=false;if(owned)URL.revokeObjectURL(owned);};},[item.id]);
  return url&&!failed?<img className="h-12 w-16 rounded object-contain" src={url} alt="" onError={()=>setFailed(true)}/>:<span className="w-16 text-xs text-muted">{failed?'图片不可用':'载入中'}</span>;
}
export function LibraryPicker({busy,onClose,onAdd,operationError,maxSelection=32}:{maxSelection?:number;busy:boolean;onClose:()=>void;onAdd:(ids:string[])=>Promise<boolean>;operationError:string|null}){
  const [page,setPage]=useState(0),[resource,setResource]=useState<{page:number;data:ScreenshotPage|null;error:string|null}|null>(null),[selected,setSelected]=useState<string[]>([]),[error,setError]=useState<string|null>(null);
  const loading=resource?.page!==page,data=loading?null:resource?.data;
  useEffect(()=>{let active=true;void readScreenshotPage(page).then(data=>{if(active)setResource({page,data,error:null});},()=>{if(active)setResource({page,data:null,error:'素材读取失败，请重试。'});});return()=>{active=false;};},[page]);
  return <Modal title="从素材库添加图片" onClose={()=>{if(!busy)onClose();}}><div className="p-5">
    <p className="mb-3 text-xs text-muted">按勾选顺序添加，使用已保存标注的合成图。可跨页选择，每次最多{maxSelection}张。</p>
    {(error||operationError||resource?.error)&&<p role="alert">{error??operationError??resource?.error}</p>}
    <fieldset disabled={busy||loading} className="max-h-80 space-y-2 overflow-auto">
      {data?.items.map(item=><label key={item.id} className="flex cursor-pointer items-center gap-3 rounded border border-edge p-2"><input type="checkbox" aria-label={`添加图片 ${item.title}`} checked={selected.includes(item.id)} onChange={event=>{if(event.target.checked){if(selected.length>=maxSelection){setError(`最多选择${maxSelection}张图片。`);return;}setSelected([...selected,item.id]);}else setSelected(selected.filter(id=>id!==item.id));}}/><Thumbnail item={item}/><span className="min-w-0 flex-1 truncate text-sm">{item.title}</span>{selected.includes(item.id)&&<span className="text-xs text-muted">第{selected.indexOf(item.id)+1}张</span>}</label>)}
      {data?.total===0&&<p className="py-5 text-sm text-muted">素材库为空，请先截图并保存标注到素材库。</p>}
    </fieldset>
    <div className="mt-4 flex flex-wrap items-center justify-between gap-2"><span className="text-xs text-muted">第{page+1}页 · 已选{selected.length}张</span><div className="flex gap-2"><button className="secondary-button" disabled={busy||loading||page===0} onClick={()=>setPage(page-1)}>上一页</button><button className="secondary-button" disabled={busy||loading||!data||(page+1)*24>=data.total} onClick={()=>setPage(page+1)}>下一页</button></div></div>
    <div className="mt-4 flex justify-end gap-2"><button className="secondary-button" disabled={busy} onClick={onClose}>取消</button><button className="capture-button" disabled={busy||!selected.length} onClick={()=>void onAdd(selected).then(ok=>{if(ok)onClose();})}>{busy?'正在合成…':`添加所选 ${selected.length} 张`}</button></div>
  </div></Modal>;
}
