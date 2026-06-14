import { useEffect, useCallback } from 'react';

interface KeyboardShortcut {
  key: string;
  ctrl?: boolean;
  meta?: boolean;
  shift?: boolean;
  alt?: boolean;
  action: () => void;
  preventDefault?: boolean;
}

export function useKeyboard(shortcuts: KeyboardShortcut[], enabled = true) {
  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    if (!enabled) return;

    for (const shortcut of shortcuts) {
      const keyMatch = event.key.toLowerCase() === shortcut.key.toLowerCase();
      const ctrlMatch = shortcut.ctrl ? (event.ctrlKey || event.metaKey) : !event.ctrlKey && !event.metaKey;
      const metaMatch = shortcut.meta ? event.metaKey : !event.metaKey;
      const shiftMatch = shortcut.shift ? event.shiftKey : !event.shiftKey;
      const altMatch = shortcut.alt ? event.altKey : !event.altKey;

      if (keyMatch && ctrlMatch && metaMatch && shiftMatch && altMatch) {
        if (shortcut.preventDefault !== false) {
          event.preventDefault();
        }
        shortcut.action();
        break;
      }
    }
  }, [shortcuts, enabled]);

  useEffect(() => {
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}

export function useEscapeKey(action: () => void, enabled = true) {
  useEffect(() => {
    if (!enabled) return;

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        action();
      }
    };

    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, [action, enabled]);
}

export function useGlobalShortcuts(actions: {
  onSearch?: () => void;
  onNewNode?: () => void;
  onSave?: () => void;
  onToggleSidebar?: () => void;
  onUniverse?: () => void;
}) {
  const shortcuts: KeyboardShortcut[] = [
    {
      key: 'k',
      ctrl: true,
      action: actions.onSearch || (() => {}),
    },
    {
      key: 'n',
      ctrl: true,
      action: actions.onNewNode || (() => {}),
    },
    {
      key: 's',
      ctrl: true,
      action: actions.onSave || (() => {}),
    },
    {
      key: 'b',
      ctrl: true,
      action: actions.onToggleSidebar || (() => {}),
    },
    {
      key: 'u',
      ctrl: true,
      action: actions.onUniverse || (() => {}),
    },
  ];

  useKeyboard(shortcuts);
}
