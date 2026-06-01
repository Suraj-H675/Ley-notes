import { useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/lib/db';
import { cn } from '@/lib/utils';
import { Badge } from '@/components/ui';
import { ArrowRight, Link2, ArrowUpRight, Users } from 'lucide-react';

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

    // Outgoing: edges where this node is the source
    const outgoingEdges = allEdges.filter((e) => e.source === nodeId);
    const outgoingNodes: BacklinkNode[] = outgoingEdges
      .map((e) => nodeMap.get(e.target))
      .filter(Boolean)
      .map((n) => ({
        id: n!.id,
        title: n!.title,
        type: n!.type,
        emoji: n!.emoji,
      }));

    // Incoming: edges where this node is the target
    const incomingEdges = allEdges.filter((e) => e.target === nodeId);
    const incomingNodes: BacklinkNode[] = incomingEdges
      .map((e) => nodeMap.get(e.source))
      .filter(Boolean)
      .map((n) => ({
        id: n!.id,
        title: n!.title,
        type: n!.type,
        emoji: n!.emoji,
      }));

    // Related: 2nd-degree connections (neighbors of neighbors, minus direct connections)
    const directNeighborIds = new Set([
      ...outgoingEdges.map((e) => e.target),
      ...incomingEdges.map((e) => e.source),
    ]);

    const secondDegreeIds = new Set<string>();
    for (const edge of allEdges) {
      if (directNeighborIds.has(edge.source) && !directNeighborIds.has(edge.target) && edge.target !== nodeId) {
        secondDegreeIds.add(edge.target);
      }
      if (directNeighborIds.has(edge.target) && !directNeighborIds.has(edge.source) && edge.source !== nodeId) {
        secondDegreeIds.add(edge.source);
      }
    }

    const relatedNodes: BacklinkNode[] = Array.from(secondDegreeIds)
      .map((id) => nodeMap.get(id))
      .filter(Boolean)
      .map((n) => ({
        id: n!.id,
        title: n!.title,
        type: n!.type,
        emoji: n!.emoji,
      }));

    return { outgoing: outgoingNodes, incoming: incomingNodes, related: relatedNodes };
  }, [nodeId]);

  const outgoing = data?.outgoing || [];
  const incoming = data?.incoming || [];
  const related = data?.related || [];

  const sortedOutgoing = useMemo(
    () => [...outgoing].sort((a, b) => (a.title || 'Untitled').localeCompare(b.title || 'Untitled')),
    [outgoing]
  );

  const sortedIncoming = useMemo(
    () => [...incoming].sort((a, b) => (a.title || 'Untitled').localeCompare(b.title || 'Untitled')),
    [incoming]
  );

  const sortedRelated = useMemo(
    () => [...related].sort((a, b) => (a.title || 'Untitled').localeCompare(b.title || 'Untitled')),
    [related]
  );

  const hasAnyLinks = sortedOutgoing.length > 0 || sortedIncoming.length > 0 || sortedRelated.length > 0;

  if (!hasAnyLinks) {
    return null;
  }

  return (
    <div className={cn('space-y-4', className)}>
      {/* Links To - Outgoing */}
      {sortedOutgoing.length > 0 && (
        <section>
          <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground mb-2">
            <ArrowUpRight className="h-4 w-4" />
            <span>Links To ({sortedOutgoing.length})</span>
          </div>
          <div className="space-y-1">
            {sortedOutgoing.map((node) => (
              <button
                key={node.id}
                onClick={() => navigate(`/page/${node.id}`)}
                className="w-full flex items-center gap-2 px-3 py-2 rounded-md text-sm text-left hover:bg-accent transition-colors"
              >
                <span className="text-lg">{node.emoji || '📄'}</span>
                <div className="flex-1 min-w-0">
                  <p className="truncate font-medium">{node.title || 'Untitled'}</p>
                  <Badge variant="outline" className="text-xs px-1 py-0">{node.type}</Badge>
                </div>
                <ArrowRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
              </button>
            ))}
          </div>
        </section>
      )}

      {/* Referenced By - Incoming */}
      {sortedIncoming.length > 0 && (
        <section>
          <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground mb-2">
            <Link2 className="h-4 w-4" />
            <span>Referenced By ({sortedIncoming.length})</span>
          </div>
          <div className="space-y-1">
            {sortedIncoming.map((node) => (
              <button
                key={node.id}
                onClick={() => navigate(`/page/${node.id}`)}
                className="w-full flex items-center gap-2 px-3 py-2 rounded-md text-sm text-left hover:bg-accent transition-colors"
              >
                <span className="text-lg">{node.emoji || '📄'}</span>
                <div className="flex-1 min-w-0">
                  <p className="truncate font-medium">{node.title || 'Untitled'}</p>
                  <Badge variant="outline" className="text-xs px-1 py-0">{node.type}</Badge>
                </div>
                <ArrowRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
              </button>
            ))}
          </div>
        </section>
      )}

      {/* Related - 2nd degree */}
      {sortedRelated.length > 0 && (
        <section>
          <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground mb-2">
            <Users className="h-4 w-4" />
            <span>Related ({sortedRelated.length})</span>
          </div>
          <div className="space-y-1">
            {sortedRelated.map((node) => (
              <button
                key={node.id}
                onClick={() => navigate(`/page/${node.id}`)}
                className="w-full flex items-center gap-2 px-3 py-2 rounded-md text-sm text-left hover:bg-accent transition-colors"
              >
                <span className="text-lg">{node.emoji || '📄'}</span>
                <div className="flex-1 min-w-0">
                  <p className="truncate font-medium">{node.title || 'Untitled'}</p>
                  <Badge variant="outline" className="text-xs px-1 py-0">{node.type}</Badge>
                </div>
                <ArrowRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
              </button>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
