import { useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useWorkspaceStore, useSearchStore } from '@/store';
import { useNodes } from './useNodes';
import type { CommandAction } from '@/types';

export function useCommands(): CommandAction[] {
  const navigate = useNavigate();
  const { createNode } = useNodes();
  const { toggleSidebar } = useWorkspaceStore();
  const { openSearch } = useSearchStore();

  return useMemo(() => {
    const commands: CommandAction[] = [
      {
        id: 'search',
        label: 'Search',
        icon: 'Search',
        category: 'navigation',
        keywords: ['find', 'cmd+k', 'command palette'],
        execute: () => openSearch(),
      },
      {
        id: 'new-page',
        label: 'New Page',
        icon: 'FilePlus',
        category: 'create',
        keywords: ['create', 'add', 'page', 'document'],
        execute: async () => {
          const node = await createNode({
            type: 'document',
            title: 'Untitled',
          });
          navigate(`/page/${node.id}`);
        },
      },
      {
        id: 'new-task',
        label: 'New Task',
        icon: 'CheckSquare',
        category: 'create',
        keywords: ['create', 'add', 'task', 'todo'],
        execute: async () => {
          const node = await createNode({
            type: 'task',
            title: 'Untitled Task',
            taskStatus: 'pending',
          });
          navigate(`/page/${node.id}`);
        },
      },
      {
        id: 'new-project',
        label: 'New Project',
        icon: 'FolderPlus',
        category: 'create',
        keywords: ['create', 'add', 'project'],
        execute: async () => {
          const node = await createNode({
            type: 'project',
            title: 'Untitled Project',
          });
          navigate(`/page/${node.id}`);
        },
      },
      {
        id: 'new-concept',
        label: 'New Concept',
        icon: 'Lightbulb',
        category: 'create',
        keywords: ['create', 'add', 'concept', 'idea'],
        execute: async () => {
          const node = await createNode({
            type: 'concept',
            title: 'Untitled Concept',
          });
          navigate(`/page/${node.id}`);
        },
      },
      {
        id: 'toggle-sidebar',
        label: 'Toggle Sidebar',
        icon: 'Sidebar',
        category: 'action',
        keywords: ['sidebar', 'toggle', 'hide', 'show'],
        execute: () => toggleSidebar(),
      },
      {
        id: 'open-universe',
        label: 'Open Universe',
        icon: 'Globe',
        category: 'navigation',
        keywords: ['graph', 'map', 'universe', 'connections'],
        execute: () => navigate('/universe'),
      },
      {
        id: 'open-home',
        label: 'Go to Home',
        icon: 'Home',
        category: 'navigation',
        keywords: ['home', 'dashboard', 'start'],
        execute: () => navigate('/'),
      },
      {
        id: 'open-tasks',
        label: 'View Tasks',
        icon: 'ListTodo',
        category: 'navigation',
        keywords: ['tasks', 'todos', 'pending'],
        execute: () => navigate('/tasks'),
      },
    ];

    return commands;
  }, [navigate, createNode, openSearch, toggleSidebar]);
}
