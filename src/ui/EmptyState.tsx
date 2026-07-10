/**
 * Empty-state placeholder for panels with no content yet.
 */

import type { ReactNode } from 'react';
import { cn } from '@/lib/classnames';

export interface EmptyStateProps {
  icon?: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({ icon, title, description, action, className }: EmptyStateProps) {
  return (
    <div
      className={cn(
        'flex flex-col items-center justify-center gap-3 px-6 py-10 text-center',
        className,
      )}
    >
      {icon ? <div className="text-subtle-foreground">{icon}</div> : null}
      <div className="space-y-1">
        <div className="text-meta font-medium text-foreground">{title}</div>
        {description ? (
          <div className="text-meta text-muted-foreground">{description}</div>
        ) : null}
      </div>
      {action}
    </div>
  );
}