import { BookTemplate, ChevronRight, FilePlus2, FileText, FolderOpen, LayoutGrid, List, Moon, Sun } from 'lucide-react';
import {ClipboardImportButton} from '../../features/import/ImportControls';
import { RecoverPinsButton } from '../../features/pins/RecoverPinsButton';
import {QuickSearch} from '../../features/library/QuickSearch';
import { useUiStore } from '../ui-store';
import { useSettingsStore } from '../../features/settings/store';
import {useDocumentStore} from '../../features/document/store';
import {CommandButton} from '../../features/commands/CommandCenter';

const titles = { screenshots: '截图素材', favorites: '收藏', tags: '标签', settings: '设置', editor: '图片编辑器',card:'科研知识卡片',compare:'图片对比',guide:'使用指南',graph:'知识图谱' };

export function Toolbar() {
  const { activeView, theme, viewMode } = useUiStore();
  const { change } = useSettingsStore();
  const setViewMode = (viewMode: 'grid' | 'list') => { void change({ viewMode }); };
  const toggleTheme = () => { void change({ theme: theme === 'dark' ? 'light' : 'dark' }); };
  return (
    <header className="app-toolbar">
      <div className="toolbar-primary-row">
        <QuickSearch/>
        <div className="toolbar-utilities">
          <CommandButton/>
          <ClipboardImportButton/>
          <RecoverPinsButton/>
          <button className="icon-button" aria-label={theme === 'dark' ? '切换为浅色主题' : '切换为深色主题'} title={theme === 'dark' ? '切换为浅色主题' : '切换为深色主题'} onClick={toggleTheme}>{theme === 'dark' ? <Sun size={19} /> : <Moon size={19} />}</button>
        </div>
      </div>
      <div className="toolbar-secondary-row">
        <div className="toolbar-breadcrumb"><span className="breadcrumb-parent">工作区</span><ChevronRight className="breadcrumb-parent text-faint" size={14}/><span>{titles[activeView]}</span></div>
        <div className="toolbar-actions">
        <button className="secondary-button" aria-label="新建 Markdown" title="新建图文 Markdown" onClick={()=>void useDocumentStore.getState().request('new')}><FileText size={15}/><span className="markdown-action-label">新建 Markdown</span></button>
        <button className="secondary-button" aria-label="新建空白 Markdown" title="新建空白 Markdown" onClick={()=>void useDocumentStore.getState().request('blank')}><FilePlus2 size={15}/><span className="markdown-action-label">新建空白 Markdown</span></button>
        <button className="secondary-button" aria-label="打开 Markdown" title="打开 Markdown" onClick={()=>void useDocumentStore.getState().request('open')}><FolderOpen size={15}/><span className="markdown-action-label">打开 Markdown</span></button>
        <button className="secondary-button" aria-label="新建模板文档" title="从模板新建 Markdown" onClick={()=>useDocumentStore.getState().setTemplateDialog('choose')}><BookTemplate size={15}/><span className="markdown-action-label">模板</span></button>
        <span className="toolbar-divider" aria-hidden="true" />
        <div className="view-toggle" role="group" aria-label="素材视图">
          <button className={`icon-button ${viewMode === 'grid' ? 'is-selected' : ''}`} aria-label="网格视图" aria-pressed={viewMode === 'grid'} title="网格视图" onClick={() => setViewMode('grid')}><LayoutGrid size={16} /></button>
          <button className={`icon-button ${viewMode === 'list' ? 'is-selected' : ''}`} aria-label="列表视图" aria-pressed={viewMode === 'list'} title="列表视图" onClick={() => setViewMode('list')}><List size={16} /></button>
        </div>
        </div>
      </div>
    </header>
  );
}
