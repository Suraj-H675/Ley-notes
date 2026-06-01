import * as React from 'react';
import { cn } from '@/lib/utils';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | 'link';
  size?: 'default' | 'sm' | 'lg' | 'icon';
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'default', size = 'default', ...props }, ref) => {
    return (
      <button
        ref={ref}
        className={cn(
          // base
          'inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-md text-[13px] font-medium',
          'transition-[background-color,color,border-color,transform,box-shadow] duration-100',
          // focus
          'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background',
          // press feedback
          'active:translate-y-px',
          // disabled
          'disabled:pointer-events-none disabled:opacity-50',
          {
            'bg-foreground text-background hover:opacity-90':
              variant === 'default',
            'bg-destructive text-destructive-foreground hover:bg-destructive/90':
              variant === 'destructive',
            'border border-border/80 bg-background/40 text-foreground/85 hover:bg-accent hover:border-border':
              variant === 'outline',
            'bg-secondary text-secondary-foreground hover:bg-secondary/80':
              variant === 'secondary',
            'text-foreground/80 hover:bg-accent hover:text-foreground':
              variant === 'ghost',
            'text-primary underline-offset-4 hover:underline': variant === 'link',
          },
          {
            'h-8 px-3': size === 'default',
            'h-7 px-2.5': size === 'sm',
            'h-9 px-4': size === 'lg',
            'h-7 w-7': size === 'icon',
          },
          className
        )}
        {...props}
      />
    );
  }
);
Button.displayName = 'Button';

export { Button };
