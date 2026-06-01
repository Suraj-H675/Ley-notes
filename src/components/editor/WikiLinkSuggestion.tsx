import { forwardRef, useEffect, useState, useImperativeHandle } from 'react';
import { cn } from '@/lib/utils';

interface WikiLinkItem {
  id: string;
  title: string;
  type: string;
  emoji?: string;
}

interface WikiLinkSuggestionListProps {
  items: WikiLinkItem[];
  command: (item: WikiLinkItem) => void;
}

export const WikiLinkSuggestionList = forwardRef<
  { onKeyDown: (props: { event: KeyboardEvent }) => boolean },
  WikiLinkSuggestionListProps
>((props, ref) => {
  const [selectedIndex, setSelectedIndex] = useState(0);

  useEffect(() => {
    setSelectedIndex(0);
  }, [props.items]);

  useImperativeHandle(ref, () => ({
    onKeyDown: ({ event }) => {
      if (event.key === 'ArrowUp') {
        event.preventDefault();
        setSelectedIndex((prev) =>
          props.items.length === 0 ? 0 : (prev - 1 + props.items.length) % props.items.length
        );
        return true;
      }
      if (event.key === 'ArrowDown') {
        event.preventDefault();
        setSelectedIndex((prev) =>
          props.items.length === 0 ? 0 : (prev + 1) % props.items.length
        );
        return true;
      }
      if (event.key === 'Enter') {
        event.preventDefault();
        const item = props.items[selectedIndex];
        if (item) props.command(item);
        return true;
      }
      return false;
    },
  }));

  if (props.items.length === 0) {
    return (
      <div className="min-w-[220px] rounded-md border border-border/80 bg-popover p-2 text-[12.5px] text-muted-foreground/70 shadow-menu">
        No matching pages
      </div>
    );
  }

  return (
    <div className="min-w-[260px] overflow-hidden rounded-md border border-border/80 bg-popover p-1 shadow-menu">
      <div className="px-2 py-1 text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground/60">
        Pages
      </div>
      {props.items.map((item, index) => (
        <button
          key={item.id}
          onClick={() => props.command(item)}
          className={cn(
            'flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-[13px] text-foreground/90 transition-colors',
            index === selectedIndex
              ? 'bg-primary/15 text-foreground'
              : 'hover:bg-accent/40'
          )}
        >
          <span className="flex h-4 w-4 flex-shrink-0 items-center justify-center text-[12px] leading-none">
            {item.emoji || (
              <span className="block h-1 w-1 rounded-full bg-muted-foreground/40" />
            )}
          </span>
          <span className="flex-1 truncate">{item.title || 'Untitled'}</span>
          <span className="ml-auto text-[10.5px] capitalize text-muted-foreground/55">
            {item.type}
          </span>
        </button>
      ))}
    </div>
  );
});

WikiLinkSuggestionList.displayName = 'WikiLinkSuggestionList';
