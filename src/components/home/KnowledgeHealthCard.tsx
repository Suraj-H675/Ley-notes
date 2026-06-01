import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/lib/db';
import { cn } from '@/lib/utils';

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
      return { totalNodes: 0, wikiLinks: 0, orphanNodes: 0, healthScore: 0 };
    }

    const wikiLinks = edges.filter((e) => e.type === 'wiki-link').length;
    const connected = new Set<string>();
    edges.forEach((e) => {
      connected.add(e.source);
      connected.add(e.target);
    });
    const orphanNodes = nodes.filter((n) => !connected.has(n.id)).length;
    const orphanRatio = nodes.length > 0 ? orphanNodes / nodes.length : 0;
    const healthScore = Math.max(0, Math.round(100 - orphanRatio * 100));

    return { totalNodes: nodes.length, wikiLinks, orphanNodes, healthScore };
  }, []);

  if (!health) return null;

  return (
    <div className={cn('rounded-lg border border-border/60 bg-card/40 p-4', className)}>
      <div className="mb-3 flex items-baseline justify-between">
        <h3 className="text-[13px] font-medium text-foreground/90">Knowledge health</h3>
        <span className="text-[11px] tabular-nums text-muted-foreground/60">
          {health.healthScore}<span className="opacity-50">/100</span>
        </span>
      </div>

      <div className="mb-3 h-1 overflow-hidden rounded-full bg-accent/50">
        <div
          className="h-full rounded-full bg-primary transition-all duration-500"
          style={{ width: `${health.healthScore}%` }}
        />
      </div>

      <div className="grid grid-cols-3 divide-x divide-border/40">
        <Metric label="Pages" value={health.totalNodes} />
        <Metric label="Links" value={health.wikiLinks} />
        <Metric label="Orphans" value={health.orphanNodes} muted={health.orphanNodes > 0} />
      </div>

      {health.orphanNodes > 0 && (
        <p className="mt-3 text-[12px] leading-relaxed text-muted-foreground/70">
          {health.orphanNodes} page{health.orphanNodes !== 1 ? 's are' : ' is'} not connected to anything yet. Linking them improves the graph.
        </p>
      )}
    </div>
  );
}

function Metric({ label, value, muted }: { label: string; value: number; muted?: boolean }) {
  return (
    <div className="px-3 first:pl-0 last:pr-0">
      <div className={cn('text-[18px] font-medium tabular-nums tracking-tight', muted ? 'text-muted-foreground/70' : 'text-foreground/90')}>
        {value}
      </div>
      <div className="text-[11px] text-muted-foreground/60">{label}</div>
    </div>
  );
}
