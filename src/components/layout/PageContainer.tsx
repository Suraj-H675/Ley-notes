import type { ReactNode } from 'react';
import { cn } from '@/lib/utils';

interface PageContainerProps {
  children: ReactNode;
  className?: string;
}

export function PageContainer({ children, className }: PageContainerProps) {
  return (
    <div className="h-full overflow-auto">
      <div className={cn('mx-auto max-w-3xl px-8 pb-16', className)}>
        {children}
      </div>
    </div>
  );
}

interface ListSectionProps {
  title: string;
  count?: number | string;
  total?: number | string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}

export function ListSection({ title, count, total, action, children, className }: ListSectionProps) {
  return (
    <section className={cn('space-y-1', className)}>
      <div className="flex items-center justify-between px-1 pb-1">
        <h2 className="flex items-center gap-2 text-[12px] font-medium text-muted-foreground/80">
          {title}
          {count !== undefined && total !== undefined ? (
            <span className="font-normal tabular-nums text-muted-foreground/50">
              {count}<span className="opacity-50">/{total}</span>
            </span>
          ) : count !== undefined ? (
            <span className="font-normal tabular-nums text-muted-foreground/50">{count}</span>
          ) : null}
        </h2>
        {action}
      </div>
      <div>{children}</div>
    </section>
  );
}
