import {OcrIndexControls} from '../ocr-index/OcrIndex';
import {useSelectionStore} from '../projects/selection';
import {useBatchStore} from '../projects/batch';
import {ProjectBar} from '../projects/ProjectBar';
import {BulkActions} from '../projects/BulkActions';
import {useImageDrop} from '../import/use-image-drop';
import { useEffect, useState } from 'react';
import { ChevronLeft, ChevronRight, Images, RefreshCw, Star } from 'lucide-react';
import { isTauri } from '@tauri-apps/api/core';
import { useUiStore } from '../../app/ui-store';
import { EmptyState } from '../../shared/ui/EmptyState';
import { Modal } from '../../shared/ui/Modal';
import { HomeView } from '../home/HomeView';
import { CapturePanel } from '../capture/CapturePanel';
import { useLibraryStore } from './store';
import { LibraryCard } from './LibraryCard';
import { LIBRARY_PAGE_SIZE, updateScreenshot } from '../../shared/api/library';
import type { LibrarySort, ScreenshotSummary } from '../../shared/api/library';
import { deleteLibraryScreenshot, openLibraryEditor } from '../../app/library-actions';
import { errorMessage } from '../../shared/api/errors';
import {openKnowledgeCard} from '../../app/card-actions';
import {useTagStore} from '../tags/store';
import {clearLibraryFilters} from './use-search';
import {useMarkdownExportStore} from '../export/store';

export function LibraryView({favorites}: {favorites:boolean}) {
  const drop=useImageDrop('library');
  const exporting=useMarkdownExportStore(s=>s.busy);
  const selection=useSelectionStore(),selected=selection.items.map(i=>i.id),batchBusy=useBatchStore(s=>s.busy);
  const {projectId,unclassified}=useLibraryStore();
  const filterKey=JSON.stringify([favorites,projectId,unclassified,useLibraryStore(s=>s.query),useLibraryStore(s=>s.source),useLibraryStore(s=>s.tagIds),useLibraryStore(s=>s.favorite)]);
  useEffect(()=>{useSelectionStore.getState().clear();},[filterKey]);
  const {viewMode, showWelcome, openCapture} = useUiStore();
  const {items, status, error, loading, page, total, sort, query,tagIds,favorite,source, refresh, retryRecovery} = useLibraryStore();
  const {items:tags,error:tagError}=useTagStore();
  const [target, setTarget] = useState<{item:ScreenshotSummary; mode:'rename'|'delete'} | null>(null);
  const [working, setWorking] = useState(false);
  const [operationError, setOperationError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  useEffect(() => {void refresh(favorites);}, [favorites, refresh]);
  useEffect(()=>{void useTagStore.getState().refresh();},[]);
  const mutate = async (action:()=>Promise<unknown>, reload = true) => {
    if (working) return;
    setWorking(true); setOperationError(null); setNotice(null);
    try {await action(); if (reload) await refresh();}
    catch (error) {setOperationError(errorMessage(error, '资料操作失败，已有资料未重置。'));}
    finally {setWorking(false);}
  };
  const busy = working || loading || exporting || batchBusy;
  const select=(id:string,shift=false)=>selection.choose(items,id,shift);
  const pages = Math.max(1, Math.ceil(total / LIBRARY_PAGE_SIZE));
  const filtered=Boolean(source!=='all'||query||tagIds.length||favorite!==null);
  return <div className={`page-container library-page image-drop-target ${drop.over?'is-drag-over':''}`} data-image-drop-target="library" {...drop.handlers}>
    <div className="page-heading"><div><h2>{favorites ? '收藏' : '截图素材'}</h2>
      <p>{favorites ? '将值得反复回看的图像，留在这里。' : '图像和标注保存在本机应用数据目录中。'}</p>
    </div><span className="subtle-badge">{viewMode === 'grid' ? '网格视图' : '列表视图'}</span></div>
    <ProjectBar/>
    <OcrIndexControls/>
    <p className="image-drop-hint" title="每批最多10张、合计64 MiB；单张20 MiB、16,777,216像素。详见使用指南。">拖入图片或点击顶部剪贴板图标导入。支持 PNG、JPG / JPEG、WebP。</p>
    {!favorites && (items.length ? <details className="library-capture-tools"><summary>截图工具 · 全屏、区域与滚动长截图</summary><CapturePanel/></details> : <CapturePanel/>)}
    {!favorites && showWelcome && !items.length && !loading && !filtered && <HomeView/>}
    {(error || operationError) && <p role="alert" className="error-banner my-3">{operationError ?? error}</p>}
    {notice && <p role="status" className="my-3 text-xs text-muted">{notice}</p>}
    {status && (status.pendingWrites > 0 || status.pendingCleanup > 0) && <div className="error-banner my-3" role="alert">
      <p>未完成保存 {status.pendingWrites} 项 · 待清理图片 {status.pendingCleanup} 项。文件与任务记录保留。</p>
      {status.warnings.slice(0, 2).map((warning, i) => <p key={i}>{warning}</p>)}
      <button className="secondary-button mt-2" disabled={busy} onClick={() => void retryRecovery()}>重试恢复与清理</button>
    </div>}
    <section data-view-mode={viewMode} aria-label={favorites ? '收藏素材' : '截图素材库'} aria-busy={loading}>
      <div className="library-filters"><label>检索来源<select aria-label="检索来源" value={source} disabled={busy} onChange={e=>void refresh(favorites,{source:e.target.value as import('../../shared/api/library').SearchSource})}><option value="all">标题、笔记、标签与 OCR</option><option value="title">标题</option><option value="notes">笔记</option><option value="tags">标签名称</option><option value="ocr">OCR 全文</option></select></label>
        <label>收藏状态<select aria-label="收藏筛选" value={favorite===null?'all':favorite?'favorite':'unfavorite'} disabled={busy||!isTauri()} onChange={e=>void refresh(favorites,{favorite:e.target.value==='all'?null:e.target.value==='favorite'})}><option value="all">全部状态</option><option value="favorite">已收藏</option><option value="unfavorite">未收藏</option></select></label>
        <details><summary>标签筛选{tagIds.length?` · ${tagIds.length}`:''}</summary><div className="library-tag-options"><p>同时包含所有选中标签</p>{tags.map(tag=><label key={tag.id}><input type="checkbox" aria-label={`筛选标签 ${tag.name}`} checked={tagIds.includes(tag.id)} disabled={busy} onChange={()=>void refresh(favorites,{tagIds:tagIds.includes(tag.id)?tagIds.filter(id=>id!==tag.id):[...tagIds,tag.id]})}/>{tag.name}</label>)}{!tags.length&&<p>还没有标签，可在知识卡片中添加。</p>}</div></details>
        {(filtered||favorites)&&<button className="secondary-button" onClick={clearLibraryFilters}>清除全部筛选</button>}
      </div>
      {tagError&&<p role="alert" className="error-banner my-3">{tagError}<button className="secondary-button ml-2" onClick={()=>void useTagStore.getState().refresh()}>重试读取标签</button></p>}
      {filtered&&<div className="library-filter-chips">{query&&<span>关键词：{query}</span>}{tagIds.map(id=><button key={id} onClick={()=>void refresh(favorites,{tagIds:tagIds.filter(t=>t!==id)})}>{tags.find(t=>t.id===id)?.name??'已移除标签'} ×</button>)}</div>}
      <div className="library-controls">
        <p className="library-count">{loading ? '正在读取素材库…' : `共 ${total} 张${favorites ? '收藏' : '素材'}`}</p>
        <div className="flex flex-wrap items-center gap-2">
          <label className="library-sort">排序
            <select aria-label="素材排序" value={sort} disabled={busy || !isTauri()}
              onChange={event => void refresh(favorites, {sort: event.target.value as LibrarySort})}>
              <option value="createdDesc">创建时间 · 最新优先</option>
              <option value="createdAsc">创建时间 · 最早优先</option>
            </select>
          </label>
          {isTauri() && <button className="icon-button" aria-label="刷新素材库" title="刷新素材库" disabled={busy}
            onClick={() => void refresh(favorites)}><RefreshCw size={16}/></button>}
        </div>
      </div>
      {isTauri()&&<BulkActions disabled={busy}/>}
      {items.length ? <div className={`library-cards ${viewMode}`}>
        {items.map(item => <LibraryCard key={item.id} item={item} disabled={busy}
          selected={selected.includes(item.id)} onSelect={event=>select(item.id,event.shiftKey)}
          onOpen={event => {if(event.ctrlKey||event.metaKey||event.shiftKey)select(item.id,event.shiftKey);else openKnowledgeCard(item.id);}}
          onEdit={() => void mutate(() => openLibraryEditor(item.id), false)}
          onFavorite={() => void mutate(() => updateScreenshot(item.id, item.revision, {isFavorite:!item.isFavorite}))}
          onRename={() => {setOperationError(null); setTarget({item, mode:'rename'});}}
          onDelete={() => {setOperationError(null); setTarget({item, mode:'delete'});}}/>)}
      </div> : !loading && !error && <div className={`library-empty ${viewMode}`}>
        <EmptyState icon={favorites ? Star : Images} title={filtered?'没有找到匹配的科研素材':favorites ? '为重要的图像留个位置' : '你的第一张素材，从这里开始'}
          description={filtered?'试试更短的关键词，或减少标签和收藏条件。':isTauri() ? (favorites ? '点击素材卡片上的星标，将它收藏到这里。' : '截图后选择“存入素材库”，或在编辑器中保存图片和标注。') : '素材库需要桌面应用；浏览器预览不会持久化科研资料。'}>
          {filtered?<button className="secondary-button mt-5" onClick={clearLibraryFilters}>重置筛选条件</button>:!favorites && <button className="secondary-button mt-5" onClick={openCapture}>了解新建截图</button>}
        </EmptyState>
      </div>}
      {isTauri() && total > 0 && <nav className="library-pagination" aria-label="素材分页">
        <p aria-live="polite">第 {page + 1} / {pages} 页 · {page * LIBRARY_PAGE_SIZE + 1}–{Math.min((page + 1) * LIBRARY_PAGE_SIZE, total)} / {total} 张</p>
        <div className="flex gap-2">
          <button className="secondary-button" disabled={busy || page === 0} onClick={() => void refresh(favorites, {page:page - 1})}><ChevronLeft size={14}/>上一页</button>
          <button className="secondary-button" disabled={busy || page + 1 >= pages} onClick={() => void refresh(favorites, {page:page + 1})}>下一页<ChevronRight size={14}/></button>
        </div>
      </nav>}
    </section>
    {!favorites && <p className="library-hint">单击缩略图或标题打开科研知识卡片。“编辑标注”进入图片编辑器；图片标注仍需点击“保存到素材库”。</p>}
    {target && <Modal title={target.mode === 'delete' ? '删除截图' : '修改素材标题'} onClose={() => {if (!working) setTarget(null);}}>
      {target.mode === 'delete' ? <div className="p-5">
        <p className="text-sm leading-relaxed">删除“{target.item.title}”的记录、笔记、标签关联及应用目录中的原图和缩略图。无法撤销；外部导出文件不会删除。已打开的编辑草稿保留，再次保存会创建新资料。</p>
        <div className="mt-5 flex justify-end gap-2">
          <button className="secondary-button" disabled={working} onClick={() => setTarget(null)}>保留截图</button>
          <button className="secondary-button text-error" disabled={working} onClick={() => void mutate(async () => {
            const result = await deleteLibraryScreenshot(target.item.id, target.item.revision);
            useSelectionStore.setState({items:selection.items.filter(i=>i.id!==target.item.id)});
            setTarget(null);
            if (result.pendingCleanup) setNotice('记录已删除；部分图片清理失败，任务已保留供重试。');
          })}>确认删除截图</button>
        </div>
      </div> : <form className="p-5" onSubmit={event => {
        event.preventDefault(); const name = String(new FormData(event.currentTarget).get('title') ?? '');
        void mutate(async () => {await updateScreenshot(target.item.id, target.item.revision, {title:name}); setTarget(null);});
      }}>
        <label className="block text-sm">标题<input name="title" aria-label="素材标题" defaultValue={target.item.title} required maxLength={200} data-initial-focus className="mt-3 w-full rounded border border-edge bg-canvas p-3"/></label>
        <div className="mt-5 flex justify-end gap-2"><button type="button" className="secondary-button" disabled={working} onClick={() => setTarget(null)}>取消</button><button className="secondary-button text-accent" disabled={working}>保存标题</button></div>
      </form>}
      {operationError && <p className="error-banner m-3" role="alert">{operationError}</p>}
    </Modal>}
  </div>;
}
