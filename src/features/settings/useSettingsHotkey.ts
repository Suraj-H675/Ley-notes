/**
 * useSettingsHotkey — Cmd+, binding for opening settings.
 */

import { useEffect, useState } from 'react';

export function useSettingsHotkey(): [boolean, (b: boolean) => void] {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === ',') {
        e.preventDefault();
        setOpen((v) => !v);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  return [open, setOpen];
}