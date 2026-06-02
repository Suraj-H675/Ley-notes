import { useEffect, useRef, useCallback, useState } from 'react';

interface AutoSaveOptions {
  delay?: number;
  onSave: (content: string) => Promise<void>;
  enabled?: boolean;
}

export function useAutoSave(
  content: string,
  options: AutoSaveOptions
) {
  const { delay = 1000, onSave, enabled = true } = options;
  const [isSaving, setIsSaving] = useState(false);
  const [lastSaved, setLastSaved] = useState<Date | null>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const contentRef = useRef(content);
  const lastSavedContentRef = useRef<string | null>(null);

  contentRef.current = content;

  const save = useCallback(async () => {
    const currentContent = contentRef.current;
    if (currentContent == null) return;

    if (currentContent === lastSavedContentRef.current) return;

    setIsSaving(true);
    try {
      await onSave(currentContent);
      lastSavedContentRef.current = currentContent;
      setLastSaved(new Date());
    } finally {
      setIsSaving(false);
    }
  }, [onSave]);

  useEffect(() => {
    if (!enabled) return;

    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }

    timeoutRef.current = setTimeout(() => {
      save();
    }, delay);

    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, [content, delay, enabled, save]);

  const saveNow = useCallback(async () => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }
    await save();
  }, [save]);

  return {
    isSaving,
    lastSaved,
    saveNow,
  };
}
