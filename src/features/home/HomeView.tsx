import { ArrowUpRight, Crop, Layers3, NotebookPen, X } from 'lucide-react';
import { useUiStore } from '../../app/ui-store';

const steps = [
  { icon: Crop, title: '捕捉研究中的灵感', description: '收集论文图表、实验记录和屏幕上的重要信息。' },
  { icon: Layers3, title: '让图像更有条理', description: '用标注突出重点，用收藏和标签组织素材。' },
  { icon: NotebookPen, title: '将发现留在笔记里', description: '连接图像与思考，整理成可导出的科研笔记。' },
];

export function HomeView() {
  const dismissWelcome = useUiStore((state) => state.dismissWelcome);
  const openCapture = useUiStore((state) => state.openCapture);

  return (
    <section className="welcome-panel" aria-labelledby="welcome-title">
      <button className="icon-button welcome-close" aria-label="收起欢迎引导" title="收起欢迎引导" onClick={dismissWelcome}><X size={16} /></button>
      <p className="mb-3 text-xs font-medium text-accent">从一张图像，开始一次发现</p>
      <h2 id="welcome-title" className="text-[25px] font-semibold tracking-tight">欢迎使用 ResearchClip</h2>
      <p className="mt-3 max-w-xl text-[13px] leading-6 text-secondary">为你的研究，留一个专注的空间。收集图像、记录思考，让有价值的发现触手可及。</p>
      <div className="guide-grid">
        {steps.map(({ icon: Icon, title, description }, index) => <div className="guide-step" key={title}><span className="guide-icon"><Icon size={18} strokeWidth={1.6} /></span><p className="mt-3 text-[13px] font-medium"><span className="mr-2 text-muted">0{index + 1}</span>{title}</p><p className="mt-2 text-xs leading-5 text-muted">{description}</p></div>)}
      </div>
      <div className="welcome-bottom"><span className="text-xs text-muted">截图后存入素材库，编辑标注后记得保存。</span><button className="text-button" onClick={openCapture}>打开截图入口<ArrowUpRight size={14} /></button></div>
    </section>
  );
}
