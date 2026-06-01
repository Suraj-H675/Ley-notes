import { useEffect } from 'react';
import { useWorkspaceStore } from '@/store';

export function useThemeEffect() {
  const theme = useWorkspaceStore((s) => s.theme);

  useEffect(() => {
    const root = document.documentElement;
    const apply = (mode: 'light' | 'dark') => {
      root.classList.remove('light', 'dark');
      root.classList.add(mode);
    };

    if (theme === 'system') {
      const mq = window.matchMedia('(prefers-color-scheme: dark)');
      apply(mq.matches ? 'dark' : 'light');
      const handler = (e: MediaQueryListEvent) => apply(e.matches ? 'dark' : 'light');
      mq.addEventListener('change', handler);
      return () => mq.removeEventListener('change', handler);
    } else {
      apply(theme);
    }
  }, [theme]);
}
