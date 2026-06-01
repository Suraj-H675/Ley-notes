import { useNavigate } from 'react-router-dom';
import { useNodes } from '@/hooks';
import { formatRelative } from '@/lib/utils';
import { FilePlus, Sparkles, ArrowUpRight } from 'lucide-react';
import { KnowledgeHealthCard } from '@/components/home/KnowledgeHealthCard';
import { seedDemoData } from '@/scripts/seedDemoData';

export function HomePage() {
  const navigate = useNavigate();
  const { nodes, createNode } = useNodes();

  const recentNodes = [...nodes]
    .sort((a, b) => b.updatedAt - a.updatedAt)
    .slice(0, 8);

  const documentCount = nodes.filter((n) => n.type === 'document').length;
  const taskCount = nodes.filter((n) => n.type === 'task').length;
  const projectCount = nodes.filter((n) => n.type === 'project').length;
  const conceptCount = nodes.filter((n) => n.type === 'concept').length;

  const handleLoadDemo = async () => {
    if (nodes.length > 0) {
      const ok = window.confirm('Replace current data with demo content?');
      if (!ok) return;
    }
    await seedDemoData();
  };

  const handleNewPage = async () => {
    const node = await createNode({ type: 'document', title: '' });
    navigate(`/page/${node.id}`);
  };

  return (
    <div className="h-full overflow-auto">
      <div className="mx-auto max-w-3xl space-y-10 px-8 py-12">
        <header className="space-y-1.5">
          <h1 className="text-[28px] font-semibold tracking-[-0.015em] text-foreground">
            {greeting()}, {firstName()}.
          </h1>
          <p className="text-[14px] text-muted-foreground/80">
            {nodes.length === 0
              ? 'A blank workspace. Start a page, or load the demo to see how things connect.'
              : `${nodes.length} page${nodes.length === 1 ? '' : 's'} in your knowledge graph.`}
          </p>
        </header>

        <div className="flex items-center gap-2">
          <button
            onClick={handleNewPage}
            className="inline-flex h-8 items-center gap-1.5 rounded-md bg-foreground px-3 text-[13px] font-medium text-background transition-opacity hover:opacity-90"
          >
            <FilePlus className="h-3.5 w-3.5" />
            New page
          </button>
          <button
            onClick={() => navigate('/universe')}
            className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border/80 bg-background/40 px-3 text-[13px] font-medium text-foreground/80 transition-colors hover:bg-accent"
          >
            Open universe
            <ArrowUpRight className="h-3 w-3 text-muted-foreground/60" />
          </button>
          {nodes.length === 0 && (
            <button
              onClick={handleLoadDemo}
              className="inline-flex h-8 items-center gap-1.5 rounded-md px-2.5 text-[13px] text-muted-foreground/80 transition-colors hover:text-foreground"
            >
              <Sparkles className="h-3.5 w-3.5" />
              Load demo
            </button>
          )}
        </div>

        <section className="grid grid-cols-4 divide-x divide-border/60 rounded-lg border border-border/60">
          <StatCell label="Documents" count={documentCount} onClick={() => {}} />
          <StatCell label="Tasks" count={taskCount} onClick={() => navigate('/tasks')} />
          <StatCell label="Projects" count={projectCount} onClick={() => navigate('/projects')} />
          <StatCell label="Concepts" count={conceptCount} onClick={() => {}} />
        </section>

        <section>
          <KnowledgeHealthCard />
        </section>

        <section className="space-y-2">
          <div className="flex items-center justify-between px-1">
            <h2 className="text-[13px] font-medium text-muted-foreground/80">
              Recent
            </h2>
            <span className="text-[11px] text-muted-foreground/50">
              {recentNodes.length} of {nodes.length}
            </span>
          </div>

          {recentNodes.length === 0 ? (
            <EmptyState onCreate={handleNewPage} onLoadDemo={handleLoadDemo} />
          ) : (
            <ul className="divide-y divide-border/40">
              {recentNodes.map((node) => (
                <li key={node.id}>
                  <button
                    onClick={() => navigate(`/page/${node.id}`)}
                    className="group flex w-full items-center gap-3 px-2 py-2.5 text-left transition-colors hover:bg-accent/40"
                  >
                    <span className="flex h-5 w-5 flex-shrink-0 items-center justify-center text-[14px] leading-none">
                      {node.emoji || (
                        <span className="block h-1.5 w-1.5 rounded-full bg-muted-foreground/40" />
                      )}
                    </span>
                    <span className="flex-1 truncate text-[13.5px] text-foreground/90">
                      {node.title || 'Untitled'}
                    </span>
                    <span className="text-[11px] capitalize text-muted-foreground/60">
                      {node.type}
                    </span>
                    <span className="hidden w-20 text-right text-[11px] text-muted-foreground/50 sm:block">
                      {formatRelative(node.updatedAt)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </div>
  );
}

function StatCell({ label, count, onClick }: { label: string; count: number; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="group flex flex-col items-start gap-0.5 px-4 py-3 text-left transition-colors first:rounded-l-lg last:rounded-r-lg hover:bg-accent/40"
    >
      <span className="text-[22px] font-medium tabular-nums tracking-tight text-foreground/90">
        {count}
      </span>
      <span className="text-[11px] text-muted-foreground/70">{label}</span>
    </button>
  );
}

function EmptyState({ onCreate, onLoadDemo }: { onCreate: () => void; onLoadDemo: () => void }) {
  return (
    <div className="rounded-lg border border-dashed border-border/60 px-6 py-10 text-center">
      <h3 className="text-[14px] font-medium text-foreground/90">A blank workspace</h3>
      <p className="mx-auto mt-1.5 max-w-sm text-[12.5px] text-muted-foreground/70">
        Create your first page to start building your knowledge graph, or load a curated set of demo pages to explore.
      </p>
      <div className="mt-5 flex items-center justify-center gap-2">
        <button
          onClick={onCreate}
          className="inline-flex h-8 items-center gap-1.5 rounded-md bg-foreground px-3 text-[13px] font-medium text-background hover:opacity-90"
        >
          <FilePlus className="h-3.5 w-3.5" />
          New page
        </button>
        <button
          onClick={onLoadDemo}
          className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border/80 px-3 text-[13px] text-foreground/80 hover:bg-accent/50"
        >
          <Sparkles className="h-3.5 w-3.5" />
          Load demo
        </button>
      </div>
    </div>
  );
}

function greeting(): string {
  const h = new Date().getHours();
  if (h < 5) return 'Working late';
  if (h < 12) return 'Good morning';
  if (h < 18) return 'Good afternoon';
  return 'Good evening';
}

function firstName(): string {
  // Light personalization without assuming; falls back to neutral
  return 'there';
}
