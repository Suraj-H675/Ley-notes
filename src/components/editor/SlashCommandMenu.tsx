import { forwardRef, useEffect, useState, useImperativeHandle, useMemo } from 'react';
import {
  Heading1,
  Heading2,
  Heading3,
  List,
  ListOrdered,
  CheckSquare,
  Quote,
  Code2,
  Minus,
  Sparkles,
} from 'lucide-react';
import { cn } from '@/lib/utils';

interface SlashCommandItem {
  id: string;
  title: string;
  description: string;
  category?: string;
  command: (props: any) => void;
}

const ICON_MAP: Record<string, typeof Heading1> = {
  heading1: Heading1,
  heading2: Heading2,
  heading3: Heading3,
  bulletList: List,
  numberedList: ListOrdered,
  taskList: CheckSquare,
  blockquote: Quote,
  codeBlock: Code2,
  horizontalRule: Minus,
};

interface SlashCommandMenuProps {
  items: SlashCommandItem[];
  command: (item: SlashCommandItem) => void;
}

export const SlashCommandMenu = forwardRef<
  { onKeyDown: (props: { event: KeyboardEvent }) => boolean },
  SlashCommandMenuProps
>((props, ref) => {
  const [selectedIndex, setSelectedIndex] = useState(0);

  useEffect(() => {
    setSelectedIndex(0);
  }, [props.items]);

  const grouped = useMemo(() => {
    const groups: Record<string, SlashCommandItem[]> = {};
    for (const item of props.items) {
      const k = item.category || 'inline';
      (groups[k] ||= []).push(item);
    }
    return groups;
  }, [props.items]);

  const flat = useMemo(() => Object.values(grouped).flat(), [grouped]);

  useImperativeHandle(ref, () => ({
    onKeyDown: ({ event }) => {
      if (event.key === 'ArrowUp') {
        event.preventDefault();
        setSelectedIndex((prev) =>
          flat.length === 0 ? 0 : (prev - 1 + flat.length) % flat.length
        );
        return true;
      }
      if (event.key === 'ArrowDown') {
        event.preventDefault();
        setSelectedIndex((prev) => (flat.length === 0 ? 0 : (prev + 1) % flat.length));
        return true;
      }
      if (event.key === 'Enter') {
        event.preventDefault();
        const item = flat[selectedIndex];
        if (item) props.command(item);
        return true;
      }
      return false;
    },
  }));

  if (props.items.length === 0) {
    return (
      <div className="min-w-[260px] rounded-md border border-border/80 bg-popover p-3 text-[12.5px] text-muted-foreground/70 shadow-menu">
        No matching commands
      </div>
    );
  }

  let cursor = 0;
  return (
    <div className="min-w-[280px] overflow-hidden rounded-md border border-border/80 bg-popover shadow-menu">
      {Object.entries(grouped).map(([category, items]) => (
        <div key={category} className="p-1">
          <div className="px-2 py-1 text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground/60">
            {categoryLabel(category)}
          </div>
          {items.map((item) => {
            const Icon = ICON_MAP[item.id] || Sparkles;
            const index = cursor++;
            const active = index === selectedIndex;
            return (
              <button
                key={item.id}
                onClick={() => props.command(item)}
                className={cn(
                  'flex w-full items-center gap-2.5 rounded px-2 py-1.5 text-left text-[13px] transition-colors',
                  active
                    ? 'bg-primary/15 text-foreground'
                    : 'text-foreground/90 hover:bg-accent/40'
                )}
              >
                <span
                  className={cn(
                    'flex h-6 w-6 flex-shrink-0 items-center justify-center rounded text-muted-foreground/80',
                    active && 'text-foreground'
                  )}
                >
                  <Icon className="h-3.5 w-3.5" />
                </span>
                <span className="flex-1 truncate">{item.title}</span>
                <span className="hidden truncate text-[11.5px] text-muted-foreground/65 sm:block">
                  {item.description}
                </span>
              </button>
            );
          })}
        </div>
      ))}
    </div>
  );
});

function categoryLabel(c: string): string {
  return c === 'inline'
    ? 'Inline'
    : c === 'lists'
    ? 'Lists'
    : c === 'blocks'
    ? 'Blocks'
    : c.charAt(0).toUpperCase() + c.slice(1);
}

SlashCommandMenu.displayName = 'SlashCommandMenu';
