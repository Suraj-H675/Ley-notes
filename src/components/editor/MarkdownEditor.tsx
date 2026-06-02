import { useEffect, useRef } from 'react';
import { cn } from '@/lib/utils';

export interface MarkdownEditorProps {
  content: string;
  onChange: (markdown: string) => void;
  onSave?: () => void;
  placeholder?: string;
  className?: string;
  autoFocus?: boolean;
}

/**
 * Phase 8a placeholder editor: a styled textarea that loads and saves
 * the markdown content directly. In Phase 8b this is replaced by a
 * CodeMirror 6 + live-preview editor.
 */
export function MarkdownEditor({
  content,
  onChange,
  onSave,
  placeholder = "Type '/' for commands, or '[[' to link another page",
  className,
  autoFocus,
}: MarkdownEditorProps) {
  const ref = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (autoFocus && ref.current) {
      ref.current.focus();
    }
  }, [autoFocus]);

  return (
    <textarea
      ref={ref}
      defaultValue={content}
      onChange={(e) => onChange(e.target.value)}
      onKeyDown={(e) => {
        if ((e.metaKey || e.ctrlKey) && e.key === 's') {
          e.preventDefault();
          onSave?.();
        }
      }}
      placeholder={placeholder}
      spellCheck
      className={cn(
        'w-full resize-none border-0 bg-transparent font-mono text-[14px] leading-relaxed',
        'text-foreground/90 outline-none placeholder:text-muted-foreground/40',
        'focus:outline-none',
        className
      )}
      rows={20}
    />
  );
}
