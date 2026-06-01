import * as React from 'react';
import { cn } from '@/lib/utils';

interface BadgeProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: 'default' | 'secondary' | 'destructive' | 'outline';
}

function Badge({ className, variant = 'default', ...props }: BadgeProps) {
  return (
    <div
      className={cn(
        'inline-flex items-center gap-1 rounded border px-1.5 py-0.5 text-[11px] font-medium transition-colors',
        {
          'border-transparent bg-foreground/95 text-background': variant === 'default',
          'border-border/60 bg-accent/50 text-foreground/85': variant === 'secondary',
          'border-transparent bg-destructive/90 text-destructive-foreground':
            variant === 'destructive',
          'border-border/60 text-foreground/80': variant === 'outline',
        },
        className
      )}
      {...props}
    />
  );
}

export { Badge };
