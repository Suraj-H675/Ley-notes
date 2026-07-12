/**
 * Keyboard hint chip. Renders something like [⌘ P] in the UI.
 */

import type { ReactNode } from 'react';
import { cn } from '@/shared/lib/classnames';

export interface KbdProps {
  children: ReactNode;
  className?: string;
}

export function Kbd({ children, className }: KbdProps) {
  return (
    <kbd
      className={cn(
        'inline-flex h-5 min-w-5 items-center justify-center rounded-sm border border-border bg-surface-2 px-1.5',
        'font-mono text-[11px] font-medium text-muted-foreground-strong',
        className,
      )}
    >
      {children}
    </kbd>
  );
}