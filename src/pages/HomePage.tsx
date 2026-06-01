import { useNavigate } from 'react-router-dom';
import { useNodes } from '@/hooks';
import { Button } from '@/components/ui';
import { formatRelative } from '@/lib/utils';
import { FilePlus, Lightbulb, CheckSquare, Folder } from 'lucide-react';
import { KnowledgeHealthCard } from '@/components/home/KnowledgeHealthCard';

export function HomePage() {
  const navigate = useNavigate();
  const { nodes } = useNodes();

  const recentNodes = nodes
    .sort((a, b) => b.updatedAt - a.updatedAt)
    .slice(0, 10);

  const taskNodes = nodes.filter((n) => n.type === 'task');
  const projectNodes = nodes.filter((n) => n.type === 'project');
  const conceptNodes = nodes.filter((n) => n.type === 'concept');
  const documentNodes = nodes.filter((n) => n.type === 'document');

  return (
    <div className="h-full overflow-auto p-8">
      <div className="max-w-4xl mx-auto space-y-8">
        <header className="space-y-4">
          <h1 className="text-3xl font-bold">Welcome to Knowledge Universe</h1>
          <p className="text-muted-foreground">
            Your local-first workspace for documents, tasks, and ideas.
          </p>
        </header>

        <section className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <StatCard
            icon={<FilePlus className="h-5 w-5" />}
            label="Documents"
            count={documentNodes.length}
            onClick={() => {}}
          />
          <StatCard
            icon={<CheckSquare className="h-5 w-5" />}
            label="Tasks"
            count={taskNodes.length}
            onClick={() => navigate('/tasks')}
          />
          <StatCard
            icon={<Folder className="h-5 w-5" />}
            label="Projects"
            count={projectNodes.length}
            onClick={() => {}}
          />
          <StatCard
            icon={<Lightbulb className="h-5 w-5" />}
            label="Concepts"
            count={conceptNodes.length}
            onClick={() => {}}
          />
        </section>

        <section>
          <KnowledgeHealthCard />
        </section>

        <section className="space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-xl font-semibold">Recent</h2>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                navigate('/page/new');
              }}
            >
              <FilePlus className="h-4 w-4 mr-2" />
              New Page
            </Button>
          </div>

          {recentNodes.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <p>No pages yet. Create your first page to get started.</p>
            </div>
          ) : (
            <div className="space-y-2">
              {recentNodes.map((node) => (
                <button
                  key={node.id}
                  onClick={() => navigate(`/page/${node.id}`)}
                  className="w-full flex items-center gap-3 p-3 rounded-lg border hover:bg-accent hover:text-accent-foreground transition-colors text-left"
                >
                  <span className="text-xl">{node.emoji || '📄'}</span>
                  <div className="flex-1 min-w-0">
                    <p className="font-medium truncate">
                      {node.title || 'Untitled'}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      {node.type} · {formatRelative(node.updatedAt)}
                    </p>
                  </div>
                </button>
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}

interface StatCardProps {
  icon: React.ReactNode;
  label: string;
  count: number;
  onClick: () => void;
}

function StatCard({ icon, label, count, onClick }: StatCardProps) {
  return (
    <button
      onClick={onClick}
      className="flex flex-col items-center gap-2 p-4 rounded-lg border bg-card hover:bg-accent transition-colors"
    >
      <div className="text-muted-foreground">{icon}</div>
      <div className="text-2xl font-bold">{count}</div>
      <div className="text-xs text-muted-foreground">{label}</div>
    </button>
  );
}
