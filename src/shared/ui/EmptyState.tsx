import type { LucideIcon } from 'lucide-react';
import type { ReactNode } from 'react';

export function EmptyState({ icon: Icon, title, description, children }: { icon: LucideIcon; title: string; description: string; children?: ReactNode }) {
  return <div className="empty-state"><span className="empty-icon"><Icon size={27} strokeWidth={1.4} /></span><h3 className="mt-5 text-[15px] font-medium">{title}</h3><p className="mt-2 max-w-sm text-[13px] leading-6 text-muted">{description}</p>{children}</div>;
}
