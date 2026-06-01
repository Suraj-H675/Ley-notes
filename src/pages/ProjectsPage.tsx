import { useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useNodes } from '@/hooks';
import { PageHeader, PageContainer, ListSection } from '@/components/layout';
import { Plus, ArrowUpRight } from 'lucide-react';
import { formatRelative } from '@/lib/utils';

export function ProjectsPage() {
  const navigate = useNavigate();
  const { nodes, createNode } = useNodes();

  const projects = useMemo(() => {
    return nodes
      .filter((n) => n.type === 'project')
      .sort((a, b) => b.updatedAt - a.updatedAt);
  }, [nodes]);

  const handleCreateProject = async () => {
    const node = await createNode({ type: 'project', title: 'Untitled project' });
    navigate(`/page/${node.id}`);
  };

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Projects"
        subtitle={`${projects.length} total`}
        actions={
          <button
            onClick={handleCreateProject}
            className="inline-flex h-7 items-center gap-1 rounded-md bg-foreground px-2.5 text-[12.5px] font-medium text-background transition-opacity hover:opacity-90"
          >
            <Plus className="h-3 w-3" />
            New project
          </button>
        }
      />

      <PageContainer>
        <ListSection title="Projects" count={projects.length} total={projects.length}>
          {projects.length === 0 ? (
            <div className="rounded-lg border border-dashed border-border/60 px-6 py-12 text-center">
              <p className="text-[13px] text-muted-foreground/70">
                No projects yet. Group related tasks and pages into a project to track them together.
              </p>
            </div>
          ) : (
            <ul className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              {projects.map((project) => {
                const props = Object.entries(project.properties || {}).slice(0, 2);
                return (
                  <li key={project.id}>
                    <button
                      onClick={() => navigate(`/page/${project.id}`)}
                      className="group block w-full rounded-lg border border-border/60 bg-card/40 p-4 text-left transition-colors hover:border-border hover:bg-accent/30"
                    >
                      <div className="flex items-start gap-2.5">
                        <span className="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-md bg-accent/50 text-[15px] leading-none">
                          {project.emoji || (
                            <span className="block h-1.5 w-1.5 rounded-full bg-muted-foreground/40" />
                          )}
                        </span>
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-1">
                            <p className="flex-1 truncate text-[14px] font-medium text-foreground/90">
                              {project.title || 'Untitled project'}
                            </p>
                            <ArrowUpRight className="h-3.5 w-3.5 flex-shrink-0 text-muted-foreground/30 transition-colors group-hover:text-foreground/70" />
                          </div>
                          <p className="mt-0.5 text-[11.5px] tabular-nums text-muted-foreground/60">
                            Updated {formatRelative(project.updatedAt)}
                          </p>
                        </div>
                      </div>

                      {props.length > 0 && (
                        <div className="mt-3 space-y-1 border-t border-border/40 pt-2.5">
                          {props.map(([key, value]) => (
                            <div
                              key={key}
                              className="flex items-center justify-between gap-2 text-[12px]"
                            >
                              <span className="text-muted-foreground/70">{key}</span>
                              <span className="truncate text-foreground/80">{value || 'Empty'}</span>
                            </div>
                          ))}
                        </div>
                      )}
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </ListSection>
      </PageContainer>
    </div>
  );
}
