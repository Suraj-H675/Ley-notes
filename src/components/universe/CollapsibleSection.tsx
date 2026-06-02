import { ChevronDown } from 'lucide-react';
import { cn } from '@/lib/utils';

export interface CollapsibleSectionProps {
  title: string;
  open: boolean;
  onToggle: () => void;
  children: React.ReactNode;
  className?: string;
}

export function CollapsibleSection({
  title,
  open,
  onToggle,
  children,
  className,
}: CollapsibleSectionProps) {
  return (
    <div className={cn('border-b border-foreground/[0.06] last:border-b-0', className)}>
      <button
        type="button"
        onClick={onToggle}
        className="flex w-full items-center justify-between px-4 py-2.5 text-[12px] font-medium uppercase tracking-wider text-foreground/85 hover:bg-foreground/[0.03]"
      >
        <span>{title}</span>
        <ChevronDown
          className={cn(
            'h-3.5 w-3.5 text-muted-foreground/70 transition-transform',
            !open && '-rotate-90'
          )}
        />
      </button>
      {open && <div className="flex flex-col gap-3 px-4 pb-4 pt-1">{children}</div>}
    </div>
  );
}
