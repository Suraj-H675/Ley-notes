import { useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useNodes } from '@/hooks';
import { Button } from '@/components/ui';
import { ArrowLeft, Folder, Plus } from 'lucide-react';
import { formatRelative } from '@/lib/utils';

export function ProjectsPage() {
  const navigate = useNavigate();
  const { nodes } = useNodes();

  const projects = useMemo(() => {
    return nodes
      .filter((n) => n.type === 'project')
      .sort((a, b) => b.updatedAt - a.updatedAt);
  }, [nodes]);

  const { createNode } = useNodes();

  const handleCreateProject = async () => {
    const node = await createNode({
      type: 'project',
      title: 'Untitled Project',
    });
    navigate(`/page/${node.id}`);
  };

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
        <h1 className="text-xl font-semibold">Projects</h1>
        <div className="flex-1" />
        <Button size="sm" onClick={handleCreateProject}>
          <Plus className="h-4 w-4 mr-2" />
          New Project
        </Button>
      </header>

      <main className="flex-1 overflow-auto p-8">
        <div className="max-w-4xl mx-auto">
          {projects.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <Folder className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>No projects yet. Create your first project to get started.</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {projects.map((project) => (
                <button
                  key={project.id}
                  onClick={() => navigate(`/page/${project.id}`)}
                  className="flex flex-col p-4 rounded-lg border bg-card hover:bg-accent transition-colors text-left"
                >
                  <div className="flex items-start gap-3">
                    <span className="text-2xl">{project.emoji || '📁'}</span>
                    <div className="flex-1 min-w-0">
                      <p className="font-medium truncate">
                        {project.title || 'Untitled Project'}
                      </p>
                      <p className="text-xs text-muted-foreground mt-1">
                        Updated {formatRelative(project.updatedAt)}
                      </p>
                    </div>
                  </div>
                  {project.properties && Object.keys(project.properties).length > 0 && (
                    <div className="mt-3 pt-3 border-t text-xs text-muted-foreground">
                      {Object.entries(project.properties).slice(0, 3).map(([key, value]) => (
                        <div key={key} className="flex justify-between">
                          <span>{key}:</span>
                          <span className="truncate ml-2">{value}</span>
                        </div>
                      ))}
                    </div>
                  )}
                </button>
              ))}
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
