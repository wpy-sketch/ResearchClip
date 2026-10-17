import { BookOpen, FlaskConical, HardDrive, Images, Plus, Settings2, Star, Tags } from 'lucide-react';
import { useUiStore } from '../ui-store';
import { useAppStore } from '../store';

const items = [
  { id: 'screenshots', label: '截图素材', icon: Images },
  { id: 'favorites', label: '收藏', icon: Star },
  { id: 'tags', label: '标签', icon: Tags },
  { id: 'settings', label: '设置', icon: Settings2 },
] as const;

export function Sidebar() {
  const activeView = useUiStore((state) => state.activeView);
  const navigate = useUiStore((state) => state.navigate);
  const openCapture = useUiStore((state) => state.openCapture);
  const openGuide = useUiStore((state) => state.openGuide);
  const version = useAppStore((state) => state.info?.version);

  return (
    <aside className="app-sidebar">
      <div className="sidebar-brand"><div className="brand-icon"><FlaskConical size={20} strokeWidth={1.7} /></div>
        <div className="sidebar-label min-w-0"><h1 className="brand-title">ResearchClip</h1><p className="mt-1 text-[10px] text-muted">截图与研究笔记</p></div>
      </div>
      <button className="sidebar-create" aria-label="新建截图" title="新建截图" onClick={openCapture}><Plus size={24} strokeWidth={1.8}/><span className="sidebar-label">新建截图</span></button>
      <div className="workspace-label"><HardDrive size={14}/><span className="sidebar-label">本地工作区</span></div>
      <p className="sidebar-section-label">素材管理</p>
      <nav aria-label="主导航" className="space-y-1">
        {items.map(({ id, label, icon: Icon }) => <button key={id} className={`nav-button ${activeView === id ? 'is-active' : ''}`} aria-label={label} aria-current={activeView === id ? 'page' : undefined} title={label} onClick={() => navigate(id)}><Icon size={20} strokeWidth={1.7} /><span className="sidebar-label">{label}</span></button>)}
      </nav>
      <div className="sidebar-bottom">
        <button className="nav-button" aria-label="使用指南" title="使用指南" onClick={openGuide}><BookOpen size={17} strokeWidth={1.7} /><span className="sidebar-label">使用指南</span></button>
        <div className="sidebar-version"><span className="sidebar-label">ResearchClip</span><span>{version ? `v${version}` : '…'}</span></div>
      </div>
    </aside>
  );
}
