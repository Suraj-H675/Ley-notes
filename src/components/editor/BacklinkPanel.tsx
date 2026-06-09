import { useNavigate } from 'react-router-dom';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/lib/db';
import { cn } from '@/lib/utils';
import { ArrowUpRight, Link2, Users, ChevronRight } from 'lucide-react';

interface BacklinkNode {
  id: string;
  title: string;
  type: string;
  emoji?: string;
}

interface BacklinkPanelProps {
  nodeId: string;
  className?: string;
}

export function BacklinkPanel({ nodeId, className }: BacklinkPanelProps) {
  const navigate = useNavigate();

  const data = useLiveQuery(async () => {
    const allEdges = await db.edges.toArray();
    const allNodes = await db.nodes.toArray();
    const nodeMap = new Map(allNodes.map((n) => [n.id, n]));

    const outgoingEdges = allEdges.filter((e) => e.source === nodeId);
    const incomingEdges = allEdges.filter((e) => e.target === nodeId);

    const mapToNode = (id: string): BacklinkNode | null => {
      const n = nodeMap.get(id);
      if (!n) return null;
      return { id: n.id, title: n.title, type: n.type, emoji: n.emoji };
    };

    const outgoing = outgoingEdges.map((e) => mapToNode(e.target)).filter((n): n is BacklinkNode => n !== null);
    const incoming = incomingEdges.map((e) => mapToNode(e.source)).filter((n): n is BacklinkNode => n !== null);

    const directIds = new Set([
      ...outgoingEdges.map((e) => e.target),
      ...incomingEdges.map((e) => e.source),
    ]);

    const secondDegreeIds = new Set<string>();
    for (const edge of allEdges) {
      if (
        directIds.has(edge.source) &&
        !directIds.has(edge.target) &&
        edge.target !== nodeId
      ) {
        secondDegreeIds.add(edge.target);
      }
      if (
        directIds.has(edge.target) &&
        !directIds.has(edge.source) &&
        edge.source !== nodeId
      ) {
        secondDegreeIds.add(edge.source);
      }
    }
    const related = Array.from(secondDegreeIds)
      .map(mapToNode)
      .filter((n): n is BacklinkNode => n !== null);

    return { outgoing, incoming, related };
  }, [nodeId]);

  if (!data) return null;
  const { outgoing, incoming, related } = data;
  const hasAny = outgoing.length > 0 || incoming.length > 0 || related.length > 0;

  if (!hasAny) {
    return (
      <div className={cn('px-1 py-4', className)}>
        <p className="text-[12px] text-muted-foreground/60">No pages link here yet.</p>
        <p className="mt-1 text-[11px] text-muted-foreground/40">Link to this page using [[Page Title]]</p>
      </div>
    );
  }

  return (
    <div className={cn('space-y-6', className)}>
      {outgoing.length > 0 && (
        <BacklinkSection
          icon={<ArrowUpRight className="h-3.5 w-3.5" />}
          title="Links to"
          nodes={outgoing}
          onOpen={(id) => navigate(`/page/${id}`)}
        />
      )}
      {incoming.length > 0 && (
        <BacklinkSection
          icon={<Link2 className="h-3.5 w-3.5" />}
          title="Referenced by"
          nodes={incoming}
          onOpen={(id) => navigate(`/page/${id}`)}
        />
      )}
      {related.length > 0 && (
        <BacklinkSection
          icon={<Users className="h-3.5 w-3.5" />}
          title="Related"
          nodes={related}
          onOpen={(id) => navigate(`/page/${id}`)}
        />
      )}
    </div>
  );
}

function BacklinkSection({
  icon,
  title,
  nodes,
  onOpen,
}: {
  icon: React.ReactNode;
  title: string;
  nodes: BacklinkNode[];
  onOpen: (id: string) => void;
}) {
  return (
    <section>
      <div className="mb-1.5 flex items-center gap-1.5 px-1 text-[11px] font-medium uppercase tracking-wider text-muted-foreground/65">
        {icon}
        <span>{title}</span>
        <span className="ml-1 tabular-nums text-muted-foreground/45">{nodes.length}</span>
      </div>
      <ul className="divide-y divide-border/40 rounded-md border border-border/40">
        {nodes.map((node) => (
          <li key={node.id}>
            <button
              onClick={() => onOpen(node.id)}
              className="group flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-[13px] transition-colors hover:bg-accent/30"
            >
              <span className="flex h-4 w-4 flex-shrink-0 items-center justify-center text-[12px] leading-none">
                {node.emoji || (
                  <span className="block h-1 w-1 rounded-full bg-muted-foreground/40" />
                )}
              </span>
              <span className="flex-1 truncate text-foreground/90">{node.title || 'Untitled'}</span>
              <span className="text-[11px] capitalize text-muted-foreground/55">{node.type}</span>
              <ChevronRight className="h-3 w-3 text-muted-foreground/30 transition-transform group-hover:translate-x-0.5 group-hover:text-foreground/60" />
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
