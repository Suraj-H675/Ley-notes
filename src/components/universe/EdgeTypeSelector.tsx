import { useState, useRef, useEffect } from 'react';
import { cn } from '@/lib/utils';
import type { EdgeType } from '@/types';
import { ChevronDown, Check } from 'lucide-react';

const EDGE_TYPES: { value: EdgeType; label: string; color: string }[] = [
  { value: 'wiki-link', label: 'Wiki Link', color: '#8b5cf6' },
  { value: 'explicit', label: 'Explicit', color: '#06b6d4' },
  { value: 'task-dependency', label: 'Task Dependency', color: '#22c55e' },
  { value: 'project-member', label: 'Project Member', color: '#f59e0b' },
  { value: 'depends-on', label: 'Depends On', color: '#ef4444' },
  { value: 'part-of', label: 'Part Of', color: '#3b82f6' },
  { value: 'related-to', label: 'Related To', color: '#6b7280' },
  { value: 'contradicts', label: 'Contradicts', color: '#f97316' },
  { value: 'extends', label: 'Extends', color: '#ec4899' },
  { value: 'uses', label: 'Uses', color: '#14b8a6' },
  { value: 'created-by', label: 'Created By', color: '#a855f7' },
];

interface EdgeTypeSelectorProps {
  value: EdgeType;
  onChange: (type: EdgeType) => void;
  className?: string;
}

export function EdgeTypeSelector({ value, onChange, className }: EdgeTypeSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  const selectedType = EDGE_TYPES.find((t) => t.value === value) || EDGE_TYPES[0];

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  return (
    <div ref={ref} className={cn('relative', className)}>
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className={cn(
          'flex items-center gap-2 px-3 py-1.5 rounded-md border text-sm transition-colors',
          'hover:bg-accent',
          isOpen && 'bg-accent'
        )}
        style={{ borderColor: selectedType.color }}
      >
        <span
          className="w-3 h-3 rounded-full"
          style={{ backgroundColor: selectedType.color }}
        />
        <span>{selectedType.label}</span>
        <ChevronDown className="h-4 w-4 text-muted-foreground" />
      </button>

      {isOpen && (
        <div className="absolute z-50 top-full left-0 mt-1 w-48 rounded-md border bg-popover shadow-lg overflow-hidden">
          {EDGE_TYPES.map((type) => (
            <button
              key={type.value}
              onClick={() => {
                onChange(type.value);
                setIsOpen(false);
              }}
              className={cn(
                'w-full flex items-center gap-2 px-3 py-2 text-sm text-left hover:bg-accent transition-colors',
                value === type.value && 'bg-accent'
              )}
            >
              <span
                className="w-3 h-3 rounded-full"
                style={{ backgroundColor: type.color }}
              />
              <span className="flex-1">{type.label}</span>
              {value === type.value && (
                <Check className="h-4 w-4 text-muted-foreground" />
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
