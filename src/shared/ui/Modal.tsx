import { useEffect, useRef } from 'react';
import type { ReactNode } from 'react';
import { X } from 'lucide-react';

export function Modal({ title, children, onClose, fitViewport=false }: { title: string; children: ReactNode; onClose: () => void; fitViewport?:boolean }) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(()=>{
    const dialog=ref.current;if(!fitViewport||!dialog)return;
    // Top-layer dialogs inherit CSS content zoom while vw/vh remain unscaled.
    // Bound only opted-in dialogs in their measured local CSS coordinate space.
    let frame=0,settle=0;
    const update=()=>{
      frame=0;if(!dialog.isConnected||!dialog.open||!dialog.offsetWidth)return;
      const scale=dialog.getBoundingClientRect().width/dialog.offsetWidth;if(!Number.isFinite(scale)||scale<=0)return;
      const width=`${Math.max(1,innerWidth/scale-32)}px`,height=`${Math.max(1,innerHeight/scale-40)}px`;
      if(dialog.style.maxWidth!==width)dialog.style.maxWidth=width;
      if(dialog.style.maxHeight!==height)dialog.style.maxHeight=height;
      // WebView2 can settle inherited zoom across several layout frames.
      if(settle-->0)frame=requestAnimationFrame(update);
    };
    const schedule=()=>{settle=3;if(!frame)frame=requestAnimationFrame(update);};
    const resize=new ResizeObserver(schedule),style=new MutationObserver(schedule);resize.observe(dialog);style.observe(document.body,{attributes:true,attributeFilter:['style','class']});style.observe(document.documentElement,{attributes:true,attributeFilter:['style','class']});window.addEventListener('resize',schedule);window.visualViewport?.addEventListener('resize',schedule);schedule();
    return()=>{cancelAnimationFrame(frame);resize.disconnect();style.disconnect();window.removeEventListener('resize',schedule);window.visualViewport?.removeEventListener('resize',schedule);};
  },[fitViewport]);
  useEffect(() => {
    const dialog = ref.current;
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    if (dialog && !dialog.open) dialog.showModal();
    dialog?.querySelector<HTMLElement>('[data-initial-focus]')?.focus();
    return () => {
      dialog?.close();
      if (previousFocus?.isConnected) previousFocus.focus();
    };
  }, []);

  return (
    <dialog ref={ref} className="app-dialog" aria-label={title}
      onCancel={(event) => { event.preventDefault(); onClose(); }}
      onClick={(event) => {
        if (event.target !== event.currentTarget) return;
        const bounds = event.currentTarget.getBoundingClientRect();
        if (event.clientX < bounds.left || event.clientX > bounds.right || event.clientY < bounds.top || event.clientY > bounds.bottom) onClose();
      }}>
      <div className="flex items-center justify-between border-b border-edge px-5 py-4">
        <h2 className="text-sm font-medium">{title}</h2>
        <button className="icon-button" aria-label="关闭对话框" onClick={onClose}><X size={17} /></button>
      </div>
      {children}
    </dialog>
  );
}
