import { ChevronRight, LayoutGrid, List, Moon, Plus, Sun } from 'lucide-react';
import {QuickSearch} from '../../features/library/QuickSearch';
import { useUiStore } from '../ui-store';
import { useSettingsStore } from '../../features/settings/store';

const titles = { screenshots: '截图素材', favorites: '收藏', tags: '标签', settings: '设置', editor: '图片编辑器',card:'科研知识卡片' };

export function Toolbar() {
  const { activeView, theme, viewMode, openCapture } = useUiStore();
  const { change } = useSettingsStore();
  const setViewMode = (viewMode: 'grid' | 'list') => { void change({ viewMode }); };
  const toggleTheme = () => { void change({ theme: theme === 'dark' ? 'light' : 'dark' }); };
  return (
    <header className="app-toolbar">
      <div className="toolbar-breadcrumb"><span className="breadcrumb-parent">工作区</span><ChevronRight className="breadcrumb-parent text-faint" size={13} /><span>{titles[activeView]}</span></div>
      <div className="toolbar-actions">
        <button className="capture-button" aria-label="新建截图" title="新建截图" onClick={openCapture}><Plus size={16} strokeWidth={2} /><span>新建截图</span></button>
        <QuickSearch/>
        <span className="toolbar-divider" aria-hidden="true" />
        <div className="view-toggle" role="group" aria-label="素材视图">
          <button className={`icon-button ${viewMode === 'grid' ? 'is-selected' : ''}`} aria-label="网格视图" aria-pressed={viewMode === 'grid'} title="网格视图" onClick={() => setViewMode('grid')}><LayoutGrid size={16} /></button>
          <button className={`icon-button ${viewMode === 'list' ? 'is-selected' : ''}`} aria-label="列表视图" aria-pressed={viewMode === 'list'} title="列表视图" onClick={() => setViewMode('list')}><List size={16} /></button>
        </div>
        <button className="icon-button" aria-label={theme === 'dark' ? '切换为浅色主题' : '切换为深色主题'} title={theme === 'dark' ? '切换为浅色主题' : '切换为深色主题'} onClick={toggleTheme}>{theme === 'dark' ? <Sun size={17} /> : <Moon size={17} />}</button>
      </div>
    </header>
  );
}
