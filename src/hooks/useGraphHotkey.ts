/**
 * useGraphHotkey — global Cmd+G / Ctrl+G binding for the full-screen graph.
 */

import { useEffect, useState } from 'react';

export function useGraphHotkey(): [boolean, (b: boolean) => void] {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'g') {
        e.preventDefault();
        setOpen((v) => !v);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  return [open, setOpen];
}