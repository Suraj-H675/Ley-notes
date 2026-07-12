/**
 * useSearchHotkey — global Cmd+O / Ctrl+O quick-switcher binding.
 */

import { useEffect, useState } from 'react';

export function useSearchHotkey(): [boolean, (b: boolean) => void] {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'o') {
        e.preventDefault();
        setOpen((v) => !v);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  return [open, setOpen];
}
