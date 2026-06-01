import { useNavigate } from 'react-router-dom';
import { ArrowLeft } from 'lucide-react';
import type { ReactNode } from 'react';
import { cn } from '@/lib/utils';

interface PageHeaderProps {
  title: string;
  subtitle?: string;
  back?: boolean;
  actions?: ReactNode;
  className?: string;
}

export function PageHeader({ title, subtitle, back = true, actions, className }: PageHeaderProps) {
  const navigate = useNavigate();

  return (
    <div className={cn('mx-auto flex w-full max-w-3xl items-center gap-2 px-8 pt-4 pb-2', className)}>
      {back && (
        <button
          onClick={() => navigate(-1)}
          aria-label="Back"
          className="flex h-6 w-6 flex-shrink-0 items-center justify-center rounded text-muted-foreground/70 transition-colors hover:bg-accent hover:text-foreground"
        >
          <ArrowLeft className="h-3.5 w-3.5" />
        </button>
      )}
      <div className="min-w-0 flex-1">
        <h1 className="truncate text-[20px] font-semibold tracking-[-0.01em] text-foreground/95">
          {title}
        </h1>
        {subtitle && (
          <p className="truncate text-[12px] text-muted-foreground/70">{subtitle}</p>
        )}
      </div>
      {actions && <div className="flex items-center gap-1">{actions}</div>}
    </div>
  );
}
