import { lazy, Suspense, useEffect } from 'react';
import { useAppStore } from './store';
import { useUiStore } from './ui-store';
import { useAppearance } from './use-appearance';
import { Sidebar } from './layout/Sidebar';
import { Toolbar } from './layout/Toolbar';
import { LibraryView } from '../features/library/LibraryView';
import { TagsView } from '../features/tags/TagsView';
import { SettingsView } from '../features/settings/SettingsView';
import { WorkspaceDialogs } from './layout/WorkspaceDialogs';
import { useSettingsStore } from '../features/settings/store';
import { useCaptureWorkflow } from '../features/capture/use-workflow';
import {KnowledgeCardView} from '../features/notes/KnowledgeCardView';
import {PendingCardNotice} from '../features/notes/PendingCardNotice';
import {useCardWorkflow} from './use-card-workflow';
import {useCardStore} from '../features/notes/store';
import {MarkdownExportStatus} from '../features/export/MarkdownExport';
import {DesktopExperience} from '../features/desktop/DesktopExperience';
import {useDesktopExperience} from '../features/desktop/use-desktop-experience';
const EditorView = lazy(() => import('../features/editor/EditorView').then((module) => ({ default: module.EditorView })));
import {WorkspaceSplit} from '../features/document/WorkspaceSplit';

export function App() {
  const initialize = useAppStore((state) => state.initialize);
  const error = useAppStore((state) => state.initializationError);
  const activeView = useUiStore((state) => state.activeView);
  const cardId=useCardStore(s=>s.activeId);
  const settingsError = useSettingsStore((state) => state.error ?? state.snapshot.storageError ?? state.snapshot.shortcutError ?? state.snapshot.fullscreenShortcutError);
  useAppearance();
  useCaptureWorkflow();
  useCardWorkflow();
  useDesktopExperience();
  useEffect(() => { void useSettingsStore.getState().initialize(); }, []);
  useEffect(() => { void initialize(); }, [initialize]);
  useEffect(() => {
    const handleKey = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        useUiStore.getState().openSearch();
      }
      if(event.defaultPrevented||(event.target instanceof Element&&event.target.closest('[data-markdown-panel]')))return;
      if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='s'&&useUiStore.getState().activeView==='card'){
        event.preventDefault();const id=useCardStore.getState().activeId;if(id)void useCardStore.getState().flush(id);
      }
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, []);

  return (
    <div className="app-shell">
      <a className="skip-link" href="#workspace-content">跳到主内容</a>
      <Sidebar />
      <div className="app-main">
        <Toolbar />
        <DesktopExperience/>
        <PendingCardNotice/>
        <MarkdownExportStatus/>
        <WorkspaceSplit><main id="workspace-content" className="workspace-scroll" tabIndex={-1}>
          {error && <div className="error-banner" role="alert">{error}</div>}
          {settingsError && activeView !== 'settings' && <div className="error-banner" role="alert">{settingsError}<button className="secondary-button ml-3" onClick={() => useUiStore.getState().navigate('settings')}>打开设置</button></div>}
          {activeView === 'editor' ? <Suspense fallback={<p className="p-6 text-sm text-muted" role="status">正在加载编辑器…</p>}><EditorView /></Suspense> : activeView==='card'?<KnowledgeCardView key={cardId}/>:activeView === 'settings' ? <SettingsView /> : activeView === 'tags' ? <TagsView /> : <LibraryView favorites={activeView === 'favorites'} />}
        </main></WorkspaceSplit>
        <footer className="workspace-footer"><span className="flex items-center gap-2"><span className="h-1.5 w-1.5 rounded-full bg-faint" />本地工作区</span><span>图片与标注保存在本机</span></footer>
      </div>
      <WorkspaceDialogs />
    </div>
  );
}
