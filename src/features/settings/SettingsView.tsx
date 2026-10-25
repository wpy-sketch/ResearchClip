import {ZoteroSetup} from '../pdf/ZoteroSetup';
import {SyncSettings} from '../sync/SyncSettings';
import {MathOcrSettings} from '../math-ocr/MathOcr';
import {useEffect} from 'react';
import {ClipperSettings} from '../clipper/Clipper';
import { Check, FolderOpen, LayoutGrid, List, Moon, Sun,Monitor } from 'lucide-react';
import { useUiStore } from '../../app/ui-store';
import { useAppStore } from '../../app/store';
import { useSettingsStore } from './store';
import {useDesktopStore} from '../desktop/store';
import {useCommandUi} from '../commands/runtime';

export function SettingsView() {
  const { theme, viewMode, fontFamily, fontSize, windowThemeError } = useUiStore();
  const { snapshot, phase, error, change, chooseDirectory } = useSettingsStore();
  const desktopState=useDesktopStore();
  const { info, connectionPhase, message, connectionError, testConnection } = useAppStore();
  const desktop = info?.mode === 'desktop'; const busy = phase !== 'idle';
  useEffect(()=>{void useSettingsStore.getState().refresh();},[]);
  return <div className="page-container settings-page">
    <div className="page-heading"><div><h2>设置</h2><p>个性化工作区，管理采集、文件与设备间同步。</p></div></div>
    <nav className="settings-shortcuts" aria-label="设置分类">{[{id:'zotero-setup-title',name:'Zotero与PDF',description:'外部阅读器 · 本机API'},{id:'appearance-title',name:'外观与字体',description:'主题 · 阅读密度'},{id:'shortcut-title',name:'截图与文件',description:'快捷键 · 保存位置'},{id:'webdav-title',name:'跨设备同步',description:'项目范围 · 冲突处理'},{id:'clipper-title',name:'浏览器剪藏',description:'本地连接 · 安装指南'}].map(item=><button key={item.id} onClick={()=>{const node=document.getElementById(item.id);if(node){node.tabIndex=-1;node.scrollIntoView({block:'start'});node.focus({preventScroll:true});}}}><strong>{item.name}</strong><span>{item.description}</span></button>)}</nav>
    <ZoteroSetup/><ClipperSettings/><SyncSettings/><MathOcrSettings/>
    <section className="settings-section"><h3>命令与工作流</h3><p>Ctrl+Shift+P打开命令面板。此组合保留给应用内命令，不注册为全局截图键。</p><button className="secondary-button" onClick={()=>useCommandUi.setState({open:true,tab:'workflows',error:null})}>管理自定义工作流</button></section>
    {(error || snapshot.storageError) && <p className="error-banner mb-5" role="alert">{error ?? snapshot.storageError}</p>}
    <section className="settings-section" aria-labelledby="appearance-title"><h3 id="appearance-title">外观</h3><p className="settings-description">选择舒适的主题与字体。</p><div className="theme-options" role="group" aria-label="主题选择">
      {([{ id: 'dark', label: '深色', icon: Moon }, { id: 'light', label: '浅色', icon: Sun },{id:'system',label:'跟随系统',icon:Monitor}] as const).map(({ id, label, icon: Icon }) => <button key={id} className={`theme-option ${snapshot.preferences.theme === id ? 'is-selected' : ''}`} disabled={busy} aria-pressed={snapshot.preferences.theme === id} onClick={() => void change({ theme: id })}><span className={`theme-preview ${id==='system'?theme:id}`} aria-hidden="true"><span /><span><i /><i /><i /></span></span><span className="flex items-center gap-2 text-[13px]"><Icon size={15} />{label}{snapshot.preferences.theme === id && <Check className="ml-auto text-accent" size={15} />}</span></button>)}
    </div><div className="mt-5 flex flex-wrap gap-4">
      <label className="min-w-40 flex-1 text-xs text-muted">界面字体<select className="display-select mt-2" aria-label="界面字体" value={fontFamily} disabled={busy} onChange={(e) => void change({ fontFamily: e.target.value as typeof fontFamily })}><option value="system">系统默认</option><option value="sans">无衬线</option><option value="serif">衬线</option></select></label>
      <label className="min-w-32 flex-1 text-xs text-muted">基础字号<select className="display-select mt-2" aria-label="基础字号" value={fontSize} disabled={busy} onChange={(e) => void change({ fontSize: Number(e.target.value) })}>{[12,14,16,18,20].map((v) => <option value={v} key={v}>{v} px</option>)}</select></label>
    </div>{windowThemeError && <p className="error-banner mt-4" role="alert">{windowThemeError}</p>}</section>
    <section className="settings-section" aria-labelledby="shortcut-title"><h3 id="shortcut-title">区域截图快捷键</h3><p className="settings-description">在其他应用中直接进入主显示器区域截图。按 Esc 取消。</p>
      <form className="mt-4 flex flex-wrap gap-3" onSubmit={(e) => { e.preventDefault(); const value = String(new FormData(e.currentTarget).get('shortcut') ?? '').trim(); void change({ captureShortcut: value }); }}>
        <input key={snapshot.preferences.captureShortcut} name="shortcut" className="display-select min-w-40 flex-1" aria-label="区域截图快捷键" defaultValue={snapshot.preferences.captureShortcut} placeholder="Ctrl+Shift+2" maxLength={128} disabled={!desktop || busy} autoComplete="off" spellCheck={false} />
        <button className="secondary-button" disabled={!desktop || busy} type="submit">应用快捷键</button>
      </form><p className="mt-3 text-xs text-muted" role="status">{snapshot.registeredShortcut ? `已注册：${snapshot.registeredShortcut}` : desktop ? '尚未注册，请设置可用的快捷键。' : '全局快捷键仅在桌面应用中可用。'}</p>
      {snapshot.shortcutError && <p className="error-banner mt-3" role="alert">{snapshot.shortcutError}</p>}
    </section>
    <section className="settings-section" aria-labelledby="fullscreen-shortcut-title"><h3 id="fullscreen-shortcut-title">全屏截图快捷键</h3><p className="settings-description">在其他应用中截取主显示器完整画面，完成后显示预览。</p>
      <form className="mt-4 flex flex-wrap gap-3" onSubmit={(e) => { e.preventDefault(); const value=String(new FormData(e.currentTarget).get('fullscreenShortcut')??'').trim(); void change({fullscreenShortcut:value}); }}>
        <input key={snapshot.preferences.fullscreenShortcut} name="fullscreenShortcut" className="display-select min-w-40 flex-1" aria-label="全屏截图快捷键" defaultValue={snapshot.preferences.fullscreenShortcut} placeholder="Ctrl+Shift+1" maxLength={128} disabled={!desktop||busy} autoComplete="off" spellCheck={false}/>
        <button className="secondary-button" disabled={!desktop||busy} type="submit">应用全屏快捷键</button>
      </form><p className="mt-3 text-xs text-muted" role="status">{snapshot.registeredFullscreenShortcut?`已注册：${snapshot.registeredFullscreenShortcut}`:desktop?'尚未注册，请设置可用的快捷键。':'全局快捷键仅在桌面应用中可用。'}</p>
      {snapshot.fullscreenShortcutError&&<p className="error-banner mt-3" role="alert">{snapshot.fullscreenShortcutError}</p>}
      <p className="mt-3 text-xs text-muted">全屏与区域截图需使用不同的快捷键。</p>
    </section>
    <section className="settings-section" aria-labelledby="save-title"><h3 id="save-title">截图保存位置</h3><p className="settings-description">作为“保存 PNG”系统对话框的默认目录，每次保存仍由你确认文件名。</p>
      {snapshot.directoryError&&<p className="error-banner mt-3" role="alert">{snapshot.directoryError}</p>}
      <p className="mt-4 break-all rounded-md border border-edge bg-canvas p-3 text-xs text-secondary select-text" data-testid="save-directory">{snapshot.preferences.saveDirectory ?? '系统图片文件夹（未指定自定义目录）'}</p>
      <button className="secondary-button mt-4" disabled={!desktop || busy} onClick={() => void chooseDirectory()}><FolderOpen size={15} />选择保存目录</button>
      <p className="mt-3 text-xs text-muted">同名文件不会被覆盖；请选择新的文件名。</p>
    </section>
    <section className="settings-section" aria-labelledby="annotation-title"><h3 id="annotation-title">标注默认值</h3><p className="settings-description">用于新图片和之后创建的对象，已有标注保持原样。</p><form key={`${snapshot.preferences.annotationColor}:${snapshot.preferences.annotationWidth}`} className="mt-4 flex flex-wrap items-end gap-4" onSubmit={event=>{event.preventDefault();const data=new FormData(event.currentTarget);void change({annotationColor:String(data.get('color')),annotationWidth:Number(data.get('width'))});}}><label className="text-xs text-muted">默认颜色<input className="mt-2 block h-9 w-16" name="color" type="color" aria-label="默认标注颜色" defaultValue={snapshot.preferences.annotationColor} disabled={busy}/></label><label className="text-xs text-muted">默认线宽（原图像素）<input className="display-select mt-2" name="width" type="number" aria-label="默认标注线宽" min={1} max={32} step={1} defaultValue={snapshot.preferences.annotationWidth} disabled={busy}/></label><button className="secondary-button" disabled={busy}>保存标注默认值</button></form></section>
    <section className="settings-section" aria-labelledby="desktop-title"><h3 id="desktop-title">窗口与系统托盘</h3><p className="settings-description">{desktopState.snapshot?.trayAvailable?'托盘已启用：可打开主窗口、设置，或开始截图。':'托盘状态将在桌面应用中显示；托盘不可用时不会隐藏主窗口。'}</p><label className="mt-4 flex items-center gap-2 text-sm"><input type="checkbox" aria-label="关闭窗口时隐藏到托盘" checked={snapshot.preferences.closeToTray} disabled={!desktop||busy} onChange={event=>void change({closeToTray:event.target.checked})}/>关闭窗口时隐藏到托盘</label><p className="mt-3 text-xs text-muted">{snapshot.preferences.closeToTray?'关闭按钮隐藏窗口并保留会话，截图快捷键继续工作；托盘“退出”会结束程序。':'关闭按钮和“退出”都会结束程序；最小化只收起窗口。'}退出前自动保存笔记，未保存标注会再次确认。</p><div className="mt-4 flex flex-wrap gap-2"><button className="secondary-button" disabled={!desktop||!desktopState.snapshot?.trayAvailable||desktopState.phase!=='idle'} onClick={()=>void desktopState.action('hide')}>隐藏到托盘</button><button className="secondary-button" disabled={!desktop||desktopState.phase!=='idle'} onClick={()=>void desktopState.action('exit')}>退出 ResearchClip</button></div></section>
    <section className="settings-section" aria-labelledby="view-title"><div className="settings-row"><div><h3 id="view-title">素材视图</h3><p className="settings-description">选择素材库的浏览方式。</p></div><div className="segmented-control" role="group" aria-label="默认素材视图"><button disabled={busy} aria-pressed={viewMode === 'grid'} className={viewMode === 'grid' ? 'is-selected' : ''} onClick={() => void change({ viewMode: 'grid' })}><LayoutGrid size={14} />网格</button><button disabled={busy} aria-pressed={viewMode === 'list'} className={viewMode === 'list' ? 'is-selected' : ''} onClick={() => void change({ viewMode: 'list' })}><List size={14} />列表</button></div></div>
      <p className="mt-5 text-xs text-muted">{desktop ? '设置保存在本机，重新启动后继续生效。' : '浏览器预览中的外观设置只在本次会话中生效。'}</p></section>
    <section className="settings-section" aria-labelledby="about-title"><div className="settings-row"><div><h3 id="about-title">关于 ResearchClip</h3><p className="settings-description">本地科研截图与知识管理工具</p></div><span className="subtle-badge">{info ? `v${info.version}` : '读取版本…'}</span></div>
      <div className="mt-6 flex flex-wrap items-center justify-between gap-3"><span className="text-xs text-muted">{desktop ? '桌面应用' : '浏览器预览'}</span><button className="secondary-button" disabled={!desktop || connectionPhase === 'loading'} onClick={() => void testConnection()}>{connectionPhase === 'loading' ? '正在检查…' : '检查桌面连接'}</button></div>
      <div className="mt-3 text-xs leading-5 text-muted" aria-live="polite">{info?.mode === 'preview' ? <p>请在桌面应用中检查后端连接。</p> : connectionError ? <p className="text-error" role="alert">{connectionError}</p> : message ? <p role="status">{message}</p> : null}</div>
    </section>
  </div>;
}
