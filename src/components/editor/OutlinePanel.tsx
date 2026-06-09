import { useEffect, useRef, useState } from 'react';
import { cn } from '@/lib/utils';
import type { MarkdownHeading } from '@/lib/markdown/extract-headings';

interface OutlinePanelProps {
  headings: MarkdownHeading[];
  onHeadingClick: (offset: number) => void;
  className?: string;
}

const LEVEL_INDENT: Record<number, string> = {
  1: 'pl-0',
  2: 'pl-3',
  3: 'pl-6',
};

export function OutlinePanel({ headings, onHeadingClick, className }: OutlinePanelProps) {
  const [activeIdx, setActiveIdx] = useState(0);
  const observerRef = useRef<IntersectionObserver | null>(null);
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);

  // Set up Intersection Observer to track which heading is at the top of viewport.
  useEffect(() => {
    if (observerRef.current) observerRef.current.disconnect();
    observerRef.current = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .map((e, i) => ({ entry: e, index: i }))
          .filter(({ entry }) => entry.isIntersecting);
        if (visible.length > 0) {
          // Pick the topmost visible heading
          setActiveIdx(visible[0].index);
        }
      },
      { rootMargin: '-60px 0px -70% 0px', threshold: 0 }
    );

    itemRefs.current.forEach((el) => {
      if (el) observerRef.current?.observe(el);
    });

    return () => observerRef.current?.disconnect();
  }, [headings]);

  if (headings.length === 0) {
    return (
      <div className={cn('flex flex-col gap-3 px-4 py-4', className)}>
        <p className="text-[12px] text-muted-foreground/60">No headings in this document</p>
      </div>
    );
  }

  return (
    <nav className={cn('flex flex-col py-2', className)} aria-label="Document outline">
      <p className="mb-2 px-4 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/50">
        Outline
      </p>
      {headings.map((h, i) => (
        <button
          key={i}
          ref={(el) => { itemRefs.current[i] = el; }}
          onClick={() => onHeadingClick(h.offset)}
          className={cn(
            'group flex items-start gap-1.5 rounded-sm px-3 py-1 text-left',
            'text-[12px] leading-snug transition-colors',
            'hover:bg-accent/60 hover:text-foreground',
            LEVEL_INDENT[h.level] ?? 'pl-6',
            i === activeIdx
              ? 'font-medium text-foreground'
              : 'text-muted-foreground/70 font-normal'
          )}
        >
          {/* Level indicator dot */}
          <span
            className={cn(
              'mt-1.5 h-1 w-1 flex-shrink-0 rounded-full transition-colors',
              i === activeIdx ? 'bg-primary' : 'bg-muted-foreground/40'
            )}
          />
          <span className="line-clamp-2">{h.text}</span>
        </button>
      ))}
    </nav>
  );
}