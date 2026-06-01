import { forwardRef, useEffect, useState } from 'react';
import { cn } from '@/lib/utils';

interface SlashCommandItem {
  id: string;
  title: string;
  description: string;
  command: (props: any) => void;
}

function getCommandIcon(id: string): string {
  const icons: Record<string, string> = {
    heading1: 'H1',
    heading2: 'H2',
    heading3: 'H3',
    bulletList: '•',
    numberedList: '1.',
    taskList: '☑',
    blockquote: '"',
    codeBlock: '<>',
    horizontalRule: 'HR',
  };
  return icons[id] || '▸';
}

interface SlashCommandMenuProps {
  items: SlashCommandItem[];
  command: (item: SlashCommandItem) => void;
}

export const SlashCommandMenu = forwardRef<HTMLDivElement, SlashCommandMenuProps>(
  (props, ref) => {
    const [selectedIndex, setSelectedIndex] = useState(0);

    useEffect(() => {
      setSelectedIndex(0);
    }, [props.items]);

    useEffect(() => {
      const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === 'ArrowUp') {
          e.preventDefault();
          setSelectedIndex((prev) =>
            (prev - 1 + props.items.length) % props.items.length
          );
          return true;
        }

        if (e.key === 'ArrowDown') {
          e.preventDefault();
          setSelectedIndex(
            (prev) => (prev + 1) % props.items.length
          );
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
          No commands found.
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
              'w-full flex items-center gap-3 px-3 py-2 text-sm text-left hover:bg-accent transition-colors',
              index === selectedIndex && 'bg-accent'
            )}
          >
            <span className="w-8 h-8 flex items-center justify-center rounded bg-accent text-sm font-mono font-medium">
              {getCommandIcon(item.id)}
            </span>
            <div className="flex-1 min-w-0">
              <p className="font-medium">{item.title}</p>
              <p className="text-xs text-muted-foreground truncate">
                {item.description}
              </p>
            </div>
          </button>
        ))}
      </div>
    );
  }
);

SlashCommandMenu.displayName = 'SlashCommandMenu';
