import {highlightParts} from '../ocr-index/highlight';
import type {MouseEvent} from 'react';
import { useEffect, useState } from 'react';
import { ImageOff, Pencil, RotateCw, Star, Trash2 } from 'lucide-react';
import { screenshotImage } from '../../shared/api/library';
import type { ScreenshotSummary } from '../../shared/api/library';
import { errorMessage } from '../../shared/api/errors';

import {InsertButton} from '../document/InsertButton';
import {SimilarityButton} from '../similarity/SimilarityPanel';

function Thumbnail({item, disabled, onOpen}: {item:ScreenshotSummary; disabled:boolean; onOpen:(event:MouseEvent)=>void}) {
  const [attempt, setAttempt] = useState(0);
  const key = `${item.id}:${attempt}`;
  const [resource, setResource] = useState<{key:string; url?:string; error?:string} | null>(null);
  useEffect(() => {
    let active = true, url: string | undefined;
    void screenshotImage(item.id, true).then(blob => {
      if (!active) return;
      url = URL.createObjectURL(blob);
      setResource({key, url});
    }).catch(error => {
      if (active) setResource({key, error: errorMessage(error, '缩略图读取失败。')});
    });
    return () => {active = false; if (url) URL.revokeObjectURL(url);};
  }, [item.id, key]);
  const current = resource?.key === key ? resource : null;
  return <div className="library-thumbnail">
    <button className="library-image-button" aria-label={`打开截图 ${item.title}`} disabled={disabled} onClick={onOpen}>
      {current?.url && !current.error
        ? <img src={current.url} alt={`${item.title}缩略图`} loading="lazy" decoding="async"
          onError={() => setResource({key, error: '缩略图无法显示。'})}/>
        : <span className="library-image-placeholder">
          <ImageOff size={22} strokeWidth={1.4}/>
          <span>{current?.error ? '缩略图读取失败' : '正在加载缩略图…'}</span>
        </span>}
    </button>
    {current?.error && <button className="library-thumbnail-retry" title={current.error}
      aria-label={`重试缩略图 ${item.title}`} disabled={disabled} onClick={() => setAttempt(n => n + 1)}>
      <RotateCw size={12}/>重试
    </button>}
  </div>;
}

interface Props {
  item: ScreenshotSummary;
  disabled: boolean;
  onOpen: (event:MouseEvent) => void;
  onEdit: () => void;
  onFavorite: () => void;
  onRename: () => void;
  onDelete: () => void;
  selected:boolean;
  onSelect:(event:MouseEvent)=>void;
}
const formatDate = new Intl.DateTimeFormat('zh-CN', {year:'numeric', month:'2-digit', day:'2-digit', hour:'2-digit', minute:'2-digit', hour12:false});

export function LibraryCard({item, disabled, onOpen, onEdit, onFavorite, onRename, onDelete,selected,onSelect}: Props) {
  return <article data-screenshot-id={item.id} className={`library-card ${selected?'is-selected':''}`}>
    <Thumbnail item={item} disabled={disabled} onOpen={onOpen}/>
    <div className="library-card-content">
      <label className="library-export-selection"><input type="checkbox" aria-label={`选择导出 ${item.title}`} checked={selected} disabled={disabled} onChange={()=>{}} onClick={onSelect}/>选择素材</label>
      <button className="library-title-button" disabled={disabled} onClick={onOpen} aria-label={`打开素材 ${item.title}`}>
        <h3 title={item.title}>{item.title}</h3>
      </button>
      <time dateTime={new Date(item.createdAt).toISOString()} title="创建时间">{formatDate.format(item.createdAt)}</time>
      <p className="library-dimensions">{item.widthPx} × {item.heightPx} px</p>
      {!item.imageAvailable && <p className="library-file-warning">原图缺失或不可访问；资料记录已保留。</p>}
      {item.matchExcerpt&&<p className="search-excerpt"><span>{({title:'标题',notes:'笔记',tags:'标签',ocr:'OCR'} as Record<string,string>)[item.matchExcerpt.source]} · </span>{highlightParts(item.matchExcerpt.text,item.matchExcerpt.keyword).map((part,i)=>part.match?<mark key={i}>{part.text}</mark>:<span key={i}>{part.text}</span>)}</p>}
      <InsertButton id={item.id} disabled={disabled}/>
      <div className="library-card-actions">
        <button className="library-edit-button" disabled={disabled} onClick={onEdit}>编辑标注</button>
        <div className="flex gap-1">
          <SimilarityButton id={item.id} title={item.title}/>
          <button className={`icon-button ${item.isFavorite ? 'text-accent' : ''}`} aria-pressed={item.isFavorite}
            aria-label={`${item.isFavorite ? '取消收藏' : '收藏'} ${item.title}`} title={item.isFavorite ? '取消收藏' : '收藏'} disabled={disabled} onClick={onFavorite}>
            <Star size={15} fill={item.isFavorite ? 'currentColor' : 'none'}/>
          </button>
          <button className="icon-button" aria-label={`修改标题 ${item.title}`} title="重命名" disabled={disabled} onClick={onRename}><Pencil size={15}/></button>
          <button className="icon-button" aria-label={`删除截图 ${item.title}`} title="删除" disabled={disabled} onClick={onDelete}><Trash2 size={15}/></button>
        </div>
      </div>
    </div>
  </article>;
}
