import { forwardRef, useEffect, useState } from 'react';
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
  HTMLDivElement,
  WikiLinkSuggestionListProps
>((props, ref) => {
  const [selectedIndex, setSelectedIndex] = useState(0);

  useEffect(() => {
    setSelectedIndex(0);
  }, [props.items]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex((prev) => (prev - 1 + props.items.length) % props.items.length);
        return true;
      }

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex((prev) => (prev + 1) % props.items.length);
        return true;
      }

      if (e.key === 'Enter') {
        e.preventDefault();
        const item = props.items[selectedIndex];
        if (item) {
          props.command(item);
        }
        return true;
      }

      return false;
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [props.items, selectedIndex, props.command]);

  if (props.items.length === 0) {
    return (
      <div
        ref={ref}
        className="bg-popover border rounded-md shadow-lg p-2 text-sm text-muted-foreground"
      >
        No results found.
      </div>
    );
  }

  return (
    <div
      ref={ref}
      className="bg-popover border rounded-md shadow-lg overflow-hidden max-h-[300px] overflow-y-auto"
    >
      {props.items.map((item, index) => (
        <button
          key={item.id}
          onClick={() => props.command(item)}
          className={cn(
            'w-full flex items-center gap-2 px-3 py-2 text-sm text-left hover:bg-accent transition-colors',
            index === selectedIndex && 'bg-accent'
          )}
        >
          <span className="text-lg">{item.emoji || '📄'}</span>
          <div className="flex-1 min-w-0">
            <p className="truncate">{item.title || 'Untitled'}</p>
            <p className="text-xs text-muted-foreground">{item.type}</p>
          </div>
        </button>
      ))}
    </div>
  );
});

WikiLinkSuggestionList.displayName = 'WikiLinkSuggestionList';
