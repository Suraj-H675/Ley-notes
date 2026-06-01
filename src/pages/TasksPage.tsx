import { useState, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useNodes } from '@/hooks';
import { Button } from '@/components/ui';
import { ArrowLeft, CheckSquare, Filter } from 'lucide-react';
import { formatRelative } from '@/lib/utils';
import type { TaskStatus } from '@/types';
import { cn } from '@/lib/utils';

export function TasksPage() {
  const navigate = useNavigate();
  const { nodes, updateNode } = useNodes();
  const [filter, setFilter] = useState<TaskStatus | 'all'>('all');

  const tasks = useMemo(() => {
    return nodes
      .filter((n) => n.type === 'task')
      .filter((n) => filter === 'all' || n.taskStatus === filter)
      .sort((a, b) => b.updatedAt - a.updatedAt);
  }, [nodes, filter]);

  const handleStatusToggle = async (taskId: string, currentStatus: TaskStatus | undefined) => {
    const nextStatus: TaskStatus =
      currentStatus === 'completed'
        ? 'pending'
        : currentStatus === 'in-progress'
        ? 'completed'
        : 'in-progress';
    await updateNode(taskId, { taskStatus: nextStatus });
  };

  const counts = useMemo(() => {
    const taskNodes = nodes.filter((n) => n.type === 'task');
    return {
      all: taskNodes.length,
      pending: taskNodes.filter((n) => n.taskStatus === 'pending').length,
      'in-progress': taskNodes.filter((n) => n.taskStatus === 'in-progress').length,
      completed: taskNodes.filter((n) => n.taskStatus === 'completed').length,
    };
  }, [nodes]);

  return (
    <div className="h-full flex flex-col">
      <header className="flex items-center gap-4 border-b p-4">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => navigate(-1)}
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <h1 className="text-xl font-semibold">Tasks</h1>
      </header>

      <div className="border-b px-8 py-3 flex items-center gap-2">
        <Filter className="h-4 w-4 text-muted-foreground" />
        <div className="flex gap-1">
          {(['all', 'pending', 'in-progress', 'completed'] as const).map((f) => (
            <Button
              key={f}
              variant={filter === f ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setFilter(f)}
              className="capitalize"
            >
              {f === 'all' ? 'All' : f.replace('-', ' ')}
              <span className="ml-1 text-xs opacity-60">
                ({counts[f]})
              </span>
            </Button>
          ))}
        </div>
      </div>

      <main className="flex-1 overflow-auto p-8">
        <div className="max-w-3xl mx-auto space-y-4">
          {tasks.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <CheckSquare className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>
                {filter === 'all'
                  ? 'No tasks yet. Create your first task to get started.'
                  : `No ${filter.replace('-', ' ')} tasks.`}
              </p>
            </div>
          ) : (
            tasks.map((task) => (
              <div
                key={task.id}
                className="w-full flex items-center gap-3 p-4 rounded-lg border bg-card hover:bg-accent transition-colors"
              >
                <button
                  onClick={() => handleStatusToggle(task.id, task.taskStatus)}
                  className={cn(
                    'w-5 h-5 rounded border flex items-center justify-center flex-shrink-0 transition-colors',
                    task.taskStatus === 'completed'
                      ? 'bg-green-500 border-green-500 text-white'
                      : 'border-muted-foreground hover:border-green-500'
                  )}
                >
                  {task.taskStatus === 'completed' && (
                    <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                    </svg>
                  )}
                </button>
                <button
                  onClick={() => navigate(`/page/${task.id}`)}
                  className="flex-1 min-w-0 text-left"
                >
                  <p
                    className={cn(
                      'font-medium truncate',
                      task.taskStatus === 'completed' && 'line-through opacity-60'
                    )}
                  >
                    {task.title || 'Untitled Task'}
                  </p>
                  <div className="flex items-center gap-2 text-xs text-muted-foreground">
                    <span
                      className={`px-2 py-0.5 rounded ${
                        task.taskStatus === 'completed'
                          ? 'bg-green-500/20 text-green-500'
                          : task.taskStatus === 'in-progress'
                          ? 'bg-yellow-500/20 text-yellow-500'
                          : 'bg-gray-500/20 text-gray-500'
                      }`}
                    >
                      {task.taskStatus || 'pending'}
                    </span>
                    <span>{formatRelative(task.updatedAt)}</span>
                  </div>
                </button>
              </div>
            ))
          )}
        </div>
      </main>
    </div>
  );
}
