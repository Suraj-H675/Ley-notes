/**
 * useDailyNoteHotkey — global Cmd+D / Ctrl+D binding that opens or creates
 * today's daily note.
 */

import { useEffect } from 'react';
import { getOrCreateDailyNote } from '@/core/vault/daily-notes';
import { useNavStore } from '@/store/nav';

export function useDailyNoteHotkey(): void {
  useEffect(() => {
    const onKey = async (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'd') {
        e.preventDefault();
        const note = await getOrCreateDailyNote();
        const nav = useNavStore.getState();
        nav.openPage(note.pageId);
        nav.pushRecent(note.pageId);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
}