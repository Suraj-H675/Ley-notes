/**
 * Text input. Same shell as Obsidian: subtle border on idle, primary on focus.
 */

import { forwardRef, type InputHTMLAttributes } from 'react';
import { cn } from '@/shared/lib/classnames';

export type InputProps = InputHTMLAttributes<HTMLInputElement>;

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { className, ...rest },
  ref,
) {
  return (
    <input
      ref={ref}
      className={cn(
        'h-8 w-full rounded-md border border-border bg-surface-1 px-2 text-body text-foreground',
        'placeholder:text-subtle-foreground',
        'focus-visible:outline-none focus-visible:border-primary',
        'disabled:opacity-50',
        className,
      )}
      {...rest}
    />
  );
});