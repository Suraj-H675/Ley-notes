import * as React from 'react';
import { cn } from '@/lib/utils';

export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, type, ...props }, ref) => {
    return (
      <input
        type={type}
        ref={ref}
        className={cn(
          'flex h-8 w-full rounded-md border border-border/60 bg-background/60 px-2.5 text-[13px] text-foreground/95',
          'placeholder:text-muted-foreground/55',
          'transition-colors',
          'hover:border-border',
          'focus:outline-none focus:border-foreground/40 focus:bg-background',
          'disabled:cursor-not-allowed disabled:opacity-50',
          className
        )}
        {...props}
      />
    );
  }
);
Input.displayName = 'Input';

export { Input };
