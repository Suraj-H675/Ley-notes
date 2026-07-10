/**
 * useSearchHotkey — global Cmd+P / Ctrl+P binding. Returns [open, setOpen].
 */

import { useEffect, useState } from 'react';

export function useSearchHotkey(): [boolean, (b: boolean) => void] {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'p') {
        e.preventDefault();
        setOpen((v) => !v);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  return [open, setOpen];
}