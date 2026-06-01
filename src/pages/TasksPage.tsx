import { useState, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useNodes } from '@/hooks';
import { PageHeader, PageContainer, ListSection } from '@/components/layout';
import { Check, Plus } from 'lucide-react';
import { formatRelative } from '@/lib/utils';
import type { TaskStatus } from '@/types';
import { cn } from '@/lib/utils';

const FILTERS: { value: TaskStatus | 'all'; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'pending', label: 'Pending' },
  { value: 'in-progress', label: 'In progress' },
  { value: 'completed', label: 'Completed' },
];

export function TasksPage() {
  const navigate = useNavigate();
  const { nodes, updateNode, createNode } = useNodes();
  const [filter, setFilter] = useState<TaskStatus | 'all'>('all');

  const tasks = useMemo(() => {
    return nodes
      .filter((n) => n.type === 'task')
      .filter((n) => filter === 'all' || n.taskStatus === filter)
      .sort((a, b) => {
        if (a.taskStatus === 'completed' && b.taskStatus !== 'completed') return 1;
        if (a.taskStatus !== 'completed' && b.taskStatus === 'completed') return -1;
        return b.updatedAt - a.updatedAt;
      });
  }, [nodes, filter]);

  const counts = useMemo(() => {
    const taskNodes = nodes.filter((n) => n.type === 'task');
    return {
      all: taskNodes.length,
      pending: taskNodes.filter((n) => n.taskStatus === 'pending').length,
      'in-progress': taskNodes.filter((n) => n.taskStatus === 'in-progress').length,
      completed: taskNodes.filter((n) => n.taskStatus === 'completed').length,
    };
  }, [nodes]);

  const handleStatusToggle = async (taskId: string, currentStatus: TaskStatus | undefined) => {
    const nextStatus: TaskStatus =
      currentStatus === 'completed'
        ? 'pending'
        : currentStatus === 'in-progress'
        ? 'completed'
        : 'in-progress';
    await updateNode(taskId, { taskStatus: nextStatus });
  };

  const handleCreateTask = async () => {
    const node = await createNode({ type: 'task', title: 'New task', taskStatus: 'pending' });
    navigate(`/page/${node.id}`);
  };

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Tasks"
        subtitle={`${counts.all} total`}
        actions={
          <button
            onClick={handleCreateTask}
            className="inline-flex h-7 items-center gap-1 rounded-md bg-foreground px-2.5 text-[12.5px] font-medium text-background transition-opacity hover:opacity-90"
          >
            <Plus className="h-3 w-3" />
            New task
          </button>
        }
      />

      <PageContainer>
        <div className="mb-4 flex items-center gap-0.5 border-b border-border/40">
          {FILTERS.map((f) => (
            <button
              key={f.value}
              onClick={() => setFilter(f.value)}
              className={cn(
                'flex h-7 items-center gap-1.5 border-b-2 px-2.5 text-[12.5px] transition-colors',
                filter === f.value
                  ? 'border-foreground/80 text-foreground'
                  : 'border-transparent text-muted-foreground/70 hover:text-foreground'
              )}
            >
              {f.label}
              <span className="rounded bg-accent/60 px-1 py-px text-[10.5px] tabular-nums text-muted-foreground/70">
                {counts[f.value]}
              </span>
            </button>
          ))}
        </div>

        <ListSection title="Tasks" count={tasks.length} total={counts.all}>
          {tasks.length === 0 ? (
            <div className="rounded-lg border border-dashed border-border/60 px-6 py-12 text-center">
              <p className="text-[13px] text-muted-foreground/70">
                {filter === 'all'
                  ? 'No tasks yet. Create your first task to get started.'
                  : `No ${filter.replace('-', ' ')} tasks.`}
              </p>
            </div>
          ) : (
            <ul className="divide-y divide-border/40">
              {tasks.map((task) => (
                <li key={task.id}>
                  <div className="group flex items-center gap-3 px-1 py-2 transition-colors hover:bg-accent/30">
                    <button
                      onClick={() => handleStatusToggle(task.id, task.taskStatus)}
                      aria-label={
                        task.taskStatus === 'completed' ? 'Mark as pending' : 'Mark as completed'
                      }
                      className={cn(
                        'flex h-4 w-4 flex-shrink-0 items-center justify-center rounded border transition-colors',
                        task.taskStatus === 'completed'
                          ? 'border-foreground/40 bg-foreground/90 text-background'
                          : task.taskStatus === 'in-progress'
                          ? 'border-foreground/50 bg-foreground/15'
                          : 'border-muted-foreground/40 hover:border-foreground/60'
                      )}
                    >
                      {task.taskStatus === 'completed' && <Check className="h-3 w-3" strokeWidth={3} />}
                      {task.taskStatus === 'in-progress' && (
                        <span className="block h-1.5 w-1.5 rounded-full bg-foreground/70" />
                      )}
                    </button>
                    <button
                      onClick={() => navigate(`/page/${task.id}`)}
                      className="min-w-0 flex-1 text-left"
                    >
                      <p
                        className={cn(
                          'truncate text-[13.5px] text-foreground/90',
                          task.taskStatus === 'completed' && 'text-muted-foreground/50 line-through'
                        )}
                      >
                        {task.title || 'Untitled task'}
                      </p>
                    </button>
                    <span className="hidden w-20 text-right text-[11.5px] tabular-nums text-muted-foreground/50 sm:block">
                      {formatRelative(task.updatedAt)}
                    </span>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </ListSection>
      </PageContainer>
    </div>
  );
}
