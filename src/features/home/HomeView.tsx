import { useAppStore } from '../../app/store';

export function HomeView() {
  const info = useAppStore((state) => state.info);
  const phase = useAppStore((state) => state.connectionPhase);
  const message = useAppStore((state) => state.message);
  const error = useAppStore((state) => state.connectionError);
  const testConnection = useAppStore((state) => state.testConnection);
  const preview = info?.mode === 'preview';

  return (
    <section className="w-full max-w-lg text-center" aria-labelledby="welcome-title">
      <h2 id="welcome-title" className="text-3xl font-semibold tracking-tight sm:text-4xl">欢迎使用 ResearchClip</h2>
      <p className="mt-4 text-sm leading-6 text-slate-500">你的本地科研截图与知识管理工具</p>
      <button
        className="mt-8 rounded-lg bg-teal-700 px-6 py-3 text-sm font-medium text-white transition-colors hover:bg-teal-800 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-teal-700 disabled:cursor-not-allowed disabled:bg-slate-300"
        disabled={info?.mode !== 'desktop' || phase === 'loading'}
        onClick={() => void testConnection()}
      >{phase === 'loading' ? '正在连接…' : '测试 Rust 通信'}</button>
      <div className="mt-5 min-h-12 text-sm leading-6" aria-live="polite">
        {preview ? <p className="text-slate-500">浏览器预览模式：请启动桌面应用测试 Rust 通信。</p>
          : error ? <p className="text-red-700" role="alert">{error}</p>
            : <p className={message ? 'text-teal-700' : 'text-slate-400'} role="status">{message ?? '点击按钮验证与 Rust 后端的连接。'}</p>}
      </div>
    </section>
  );
}
