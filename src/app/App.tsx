import { useEffect } from 'react';
import { useAppStore } from './store';
import { HomeView } from '../features/home/HomeView';

export function App() {
  const info = useAppStore((state) => state.info);
  const initialize = useAppStore((state) => state.initialize);
  const initializationError = useAppStore((state) => state.initializationError);
  useEffect(() => { void initialize(); }, [initialize]);

  return (
    <div className="flex min-h-dvh flex-col bg-slate-50 text-slate-800">
      <header className="border-b border-slate-200 bg-white px-8 py-5">
        <h1 className="text-lg font-semibold tracking-tight">ResearchClip</h1>
      </header>
      <main className="flex flex-1 items-center justify-center px-6 py-12"><HomeView /></main>
      <footer className="border-t border-slate-200 bg-white px-6 py-4 text-center text-xs text-slate-500">
        {initializationError ? <span role="alert">{initializationError}</span>
          : <span>版本 {info ? info.version : '读取中…'}{info?.mode === 'preview' ? ' · 浏览器预览' : ''}</span>}
      </footer>
    </div>
  );
}
