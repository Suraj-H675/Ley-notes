import { useCallback, useRef, useState } from 'react';
import { Plus, X } from 'lucide-react';
import { cn } from '@/lib/utils';

interface PropertiesEditorProps {
  properties: Record<string, string>;
  onChange: (properties: Record<string, string>) => void;
  className?: string;
}

/** Inline key-value editor for YAML frontmatter properties.
 * Rendered below the document title and above the editor body. */
export function PropertiesEditor({ properties, onChange, className }: PropertiesEditorProps) {
  const [rows, setRows] = useState<[string, string][]>(() =>
    Object.entries(properties).filter(([, v]) => v !== '' && v != null)
  );
  const inputRefs = useRef<(HTMLInputElement | null)[]>([]);

  const syncToProps = useCallback(
    (newRows: [string, string][]) => {
      // Build a clean properties object: trim keys, reject empty keys,
      // and keep only the last value per deduplicated key.
      const seen = new Map<string, number>();
      newRows.forEach(([k], i) => {
        const key = k.trim();
        if (key) seen.set(key, i);
      });
      const props: Record<string, string> = {};
      for (const [key, idx] of seen) {
        props[key] = newRows[idx][1];
      }
      onChange(props);
    },
    [onChange]
  );

  const handleKeyChange = useCallback(
    (idx: number, field: 0 | 1, value: string) => {
      setRows((prev) => {
        const next: [string, string][] = prev.map((r, i) =>
          i === idx ? (field === 0 ? [value, r[1]] : [r[0], value]) : r
        );
        return next;
      });
    },
    []
  );

  const handleKeyBlur = useCallback(
    (idx: number) => {
      setRows((prev) => {
        // Trim keys and drop rows with empty keys.
        const trimmed: [string, string][] = prev.map((r, i) =>
          i === idx ? [r[0].trim(), r[1]] : r
        );
        const filtered = trimmed.filter(([k]) => k !== '');
        if (filtered.length === prev.length && trimmed[idx][0] === prev[idx][0]) return prev;
        return filtered;
      });
    },
    []
  );

  const handleValueBlur = useCallback(
    (idx: number) => {
      setRows((prev) => {
        const trimmed: [string, string][] = prev.map((r, i) =>
          i === idx ? [r[0], r[1].trim()] : r
        );
        return trimmed;
      });
      // Sync to parent on blur so the last edit is committed.
      setRows((prev) => {
        syncToProps(prev);
        return prev;
      });
    },
    [syncToProps]
  );

  const handleDelete = useCallback(
    (idx: number) => {
      setRows((prev) => {
        const next: [string, string][] = prev.filter((_, i) => i !== idx);
        syncToProps(next);
        return next;
      });
    },
    [syncToProps]
  );

  const handleAdd = useCallback(() => {
    const idx = rows.length;
    setRows((prev) => {
      const next: [string, string][] = [...prev, ['', '']];
      return next;
    });
    // Focus the new key input on next tick.
    setTimeout(() => {
      inputRefs.current[idx]?.focus();
    }, 0);
  }, [rows.length]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent, idx: number, field: 0 | 1) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        if (field === 0) {
          // Move focus to value input on Enter in key field
          (inputRefs.current[idx + 0.5] as HTMLInputElement | null)?.focus();
        } else {
          // Enter on value: blur to commit
          (e.currentTarget as HTMLElement).blur();
        }
      }
      if (e.key === 'Backspace' && field === 0) {
        const [k] = rows[idx];
        if (k === '' && rows.length > 0) {
          e.preventDefault();
          const newRows: [string, string][] = rows.filter((_, i) => i !== idx);
          setRows(newRows);
          syncToProps(newRows);
          const focusIdx = Math.max(0, idx - 1);
          setTimeout(() => inputRefs.current[focusIdx]?.focus(), 0);
        }
      }
    },
    [rows, syncToProps]
  );

  const isEmpty = rows.length === 0;

  return (
    <div className={cn('group/props', className)}>
      {/* Collapsed hint showing number of properties */}
      {isEmpty && (
        <button
          onClick={handleAdd}
          className="mt-0.5 flex items-center gap-1.5 rounded-sm px-1.5 py-0.5
                     text-[11px] text-muted-foreground/50 opacity-0 group-hover/props:opacity-100
                     transition-opacity hover:text-muted-foreground/80 hover:bg-accent/50"
        >
          <Plus size={11} />
          Add properties
        </button>
      )}

      {/* Properties table */}
      {!isEmpty && (
        <div className="mt-1.5 rounded-sm border border-border/40 bg-accent/20 py-1 pl-1.5 pr-2">
          {rows.map(([key, value], idx) => (
            <div key={idx} className="flex items-center gap-2 py-0.5">
              {/* Key */}
              <input
                ref={(el) => { inputRefs.current[idx] = el; }}
                value={key}
                onChange={(e) => handleKeyChange(idx, 0, e.target.value)}
                onBlur={() => handleKeyBlur(idx)}
                onKeyDown={(e) => handleKeyDown(e, idx, 0)}
                placeholder="key"
                className="min-w-0 flex-1 bg-transparent text-[12px] font-medium
                           text-foreground/80 outline-none placeholder:text-muted-foreground/40"
              />
              <span className="text-muted-foreground/30 text-[11px]">:</span>
              {/* Value — stored at idx + 0.5 to allow Tab from key → value */}
              <input
                ref={(el) => { inputRefs.current[idx + 0.5] = el; }}
                value={value}
                onChange={(e) => handleKeyChange(idx, 1, e.target.value)}
                onBlur={() => handleValueBlur(idx)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    (e.currentTarget as HTMLElement).blur();
                  }
                }}
                placeholder="value"
                className="min-w-0 flex-[2] bg-transparent text-[12px]
                           text-foreground/70 outline-none placeholder:text-muted-foreground/40"
              />
              {/* Delete */}
              <button
                onClick={() => handleDelete(idx)}
                className="flex-shrink-0 rounded-sm p-0.5 text-muted-foreground/40
                           hover:text-destructive/80 hover:bg-destructive/10 transition-colors"
              >
                <X size={11} />
              </button>
            </div>
          ))}

          {/* Add row */}
          <button
            onClick={handleAdd}
            className="mt-0.5 flex items-center gap-1 rounded-sm px-1 py-0.5
                       text-[11px] text-muted-foreground/50 hover:text-muted-foreground/80
                       hover:bg-accent/50 transition-colors"
          >
            <Plus size={11} />
            Add property
          </button>
        </div>
      )}
    </div>
  );
}