/**
 * Button. Three variants: primary (action), ghost (low-emphasis), destructive.
 * Sizes: sm, md.
 */

import { forwardRef, type ButtonHTMLAttributes } from 'react';
import { cn } from '@/lib/classnames';

type Variant = 'primary' | 'ghost' | 'outline' | 'destructive';
type Size = 'sm' | 'md';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

const variantClasses: Record<Variant, string> = {
  primary:
    'bg-primary text-primary-foreground hover:bg-primary/90 active:bg-primary/80',
  ghost:
    'bg-transparent text-foreground hover:bg-surface-3 active:bg-surface-2',
  outline:
    'bg-transparent text-foreground border border-border hover:bg-surface-2 active:bg-surface-3',
  destructive:
    'bg-destructive text-destructive-foreground hover:bg-destructive/90 active:bg-destructive/80',
};

const sizeClasses: Record<Size, string> = {
  sm: 'h-7 px-2 text-meta',
  md: 'h-8 px-3 text-body',
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  function Button({ className, variant = 'ghost', size = 'md', type = 'button', ...rest }, ref) {
    return (
      <button
        ref={ref}
        type={type}
        className={cn(
          'inline-flex items-center justify-center gap-1.5 rounded-md font-medium transition-colors',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-background',
          'disabled:pointer-events-none disabled:opacity-50',
          variantClasses[variant],
          sizeClasses[size],
          className,
        )}
        {...rest}
      />
    );
  },
);