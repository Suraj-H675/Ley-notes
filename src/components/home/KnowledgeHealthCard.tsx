import { useMemo } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/lib/db';
import { cn } from '@/lib/utils';
import { Link2, AlertCircle, CheckCircle2, TrendingUp } from 'lucide-react';

interface KnowledgeHealthCardProps {
  className?: string;
}

export function KnowledgeHealthCard({ className }: KnowledgeHealthCardProps) {
  const health = useLiveQuery(async () => {
    const [nodes, edges] = await Promise.all([
      db.nodes.where('isArchived').equals(0).toArray(),
      db.edges.toArray(),
    ]);

    if (nodes.length === 0) {
      return {
        totalNodes: 0,
        totalEdges: 0,
        orphanNodes: 0,
        wikiLinks: 0,
        healthScore: 0,
      };
    }

    const wikiLinks = edges.filter((e) => e.type === 'wiki-link').length;

    const connectedNodeIds = new Set<string>();
    edges.forEach((e) => {
      connectedNodeIds.add(e.source);
      connectedNodeIds.add(e.target);
    });

    const orphanNodes = nodes.filter((n) => !connectedNodeIds.has(n.id)).length;
    const orphanPercentage = nodes.length > 0 ? (orphanNodes / nodes.length) * 100 : 0;

    // Health score: 100 if no orphans, decreases by 10% per orphan node
    const healthScore = Math.max(0, Math.round(100 - orphanPercentage * 10));

    return {
      totalNodes: nodes.length,
      totalEdges: edges.length,
      orphanNodes,
      wikiLinks,
      healthScore,
    };
  }, []);

  const healthLevel = useMemo(() => {
    if (!health) return 'unknown';
    if (health.healthScore >= 80) return 'excellent';
    if (health.healthScore >= 60) return 'good';
    if (health.healthScore >= 40) return 'fair';
    return 'needs-attention';
  }, [health?.healthScore]);

  const HealthIcon = useMemo(() => {
    switch (healthLevel) {
      case 'excellent':
      case 'good':
        return CheckCircle2;
      case 'fair':
        return TrendingUp;
      default:
        return AlertCircle;
    }
  }, [healthLevel]);

  const healthColor = useMemo(() => {
    switch (healthLevel) {
      case 'excellent':
        return 'text-green-500';
      case 'good':
        return 'text-blue-500';
      case 'fair':
        return 'text-yellow-500';
      default:
        return 'text-red-500';
    }
  }, [healthLevel]);

  if (!health) {
    return null;
  }

  return (
    <div className={cn('p-4 rounded-lg border bg-card', className)}>
      <div className="flex items-start justify-between">
        <div>
          <h3 className="font-semibold flex items-center gap-2">
            <TrendingUp className="h-4 w-4" />
            Knowledge Health
          </h3>
          <p className="text-xs text-muted-foreground mt-1">
            Your knowledge base wellness
          </p>
        </div>
        <div className={cn('flex items-center gap-1', healthColor)}>
          <HealthIcon className="h-5 w-5" />
          <span className="text-lg font-bold">{health.healthScore}%</span>
        </div>
      </div>

      <div className="mt-4 grid grid-cols-3 gap-2 text-center">
        <div className="p-2 rounded bg-accent/50">
          <div className="text-lg font-bold">{health.totalNodes}</div>
          <div className="text-xs text-muted-foreground">Nodes</div>
        </div>
        <div className="p-2 rounded bg-accent/50">
          <div className="text-lg font-bold">{health.wikiLinks}</div>
          <div className="text-xs text-muted-foreground flex items-center justify-center gap-1">
            <Link2 className="h-3 w-3" />
            Links
          </div>
        </div>
        <div className="p-2 rounded bg-accent/50">
          <div className="text-lg font-bold">{health.orphanNodes}</div>
          <div className="text-xs text-muted-foreground">Orphans</div>
        </div>
      </div>

      {health.orphanNodes > 0 && (
        <p className="mt-3 text-xs text-muted-foreground">
          {health.orphanNodes} node{health.orphanNodes !== 1 ? 's' : ''} not connected to any other node.
          Link them to improve your knowledge graph.
        </p>
      )}
    </div>
  );
}
