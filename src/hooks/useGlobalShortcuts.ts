import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useSearchStore, useWorkspaceStore } from '@/store';
import { db } from '@/lib/db';
import { nanoid } from 'nanoid';

/**
 * Global keyboard shortcuts wired to the whole app.
 * Skips when the user is typing in an input/textarea/contenteditable.
 */
export function useGlobalShortcuts(createNode?: (input: any) => Promise<any>) {
  const navigate = useNavigate();
  const { openSearch, openQuickSwitcher } = useSearchStore();
  const { toggleSidebar } = useWorkspaceStore();

  useEffect(() => {
    const handler = async (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;

      if (mod && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        openSearch();
        return;
      }

      if (mod && e.key.toLowerCase() === 'n' && !e.shiftKey) {
        e.preventDefault();
        if (createNode) {
          const node = await createNode({ type: 'document', title: '' });
          navigate(`/page/${node.id}`);
        } else {
          const node = {
            id: nanoid(),
            type: 'document' as const,
            title: '',
            content: null,
            plainText: '',
            collections: [],
            tags: [],
            properties: {},
            isArchived: 0 as const,
            isPinned: 0 as const,
            createdAt: Date.now(),
            updatedAt: Date.now(),
          };
          await db.nodes.add(node);
          navigate(`/page/${node.id}`);
        }
        return;
      }

      if (mod && e.key === '/') {
        e.preventDefault();
        toggleSidebar();
        return;
      }

      if (mod && e.shiftKey && e.key.toLowerCase() === 'o') {
        e.preventDefault();
        openQuickSwitcher();
        return;
      }
    };

    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [navigate, openSearch, openQuickSwitcher, toggleSidebar, createNode]);
}
