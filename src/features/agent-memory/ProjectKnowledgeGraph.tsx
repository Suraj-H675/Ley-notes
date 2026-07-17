import "@xyflow/react/dist/style.css";
import { useEffect, useMemo, useState } from "react";
import {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  ReactFlow,
  type Edge,
  type Node,
} from "@xyflow/react";
import {
  AlertTriangle,
  GitBranch,
  Network,
  Search,
  ShieldCheck,
  X,
} from "lucide-react";
import { readAgentProjectGraphView } from "./api";
import type {
  ProjectGraphNodeKind,
  ProjectGraphView,
  ProjectGraphViewNode,
} from "./types";

type FlowNode = Node<
  { label: string; source: ProjectGraphViewNode },
  "default"
>;

const KIND_LANES: Record<ProjectGraphNodeKind, number> = {
  project: 0,
  file: 300,
  dependency: 300,
  symbol: 620,
  "external-module": 940,
  "external-symbol": 940,
};

const KIND_COLORS: Record<ProjectGraphNodeKind, string> = {
  project: "#8b5cf6",
  file: "#3b82f6",
  dependency: "#f59e0b",
  symbol: "#10b981",
  "external-module": "#64748b",
  "external-symbol": "#94a3b8",
};

export function ProjectKnowledgeGraph({
  projectPath,
}: {
  projectPath: string;
}) {
  const [query, setQuery] = useState("");
  const [deferredQuery, setDeferredQuery] = useState("");
  const [view, setView] = useState<ProjectGraphView | null>(null);
  const [selected, setSelected] = useState<ProjectGraphViewNode | null>(null);
  const [completedRequest, setCompletedRequest] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const requestKey = `${projectPath}\u0000${deferredQuery}`;
  const loading = completedRequest !== requestKey;

  useEffect(() => {
    const timer = window.setTimeout(() => setDeferredQuery(query), 240);
    return () => window.clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    let current = true;
    void readAgentProjectGraphView(projectPath, deferredQuery)
      .then((next) => {
        if (current) {
          setView(next);
          setSelected(null);
          setError(null);
        }
      })
      .catch((cause) => {
        if (current)
          setError(cause instanceof Error ? cause.message : String(cause));
      })
      .finally(() => {
        if (current) setCompletedRequest(requestKey);
      });
    return () => {
      current = false;
    };
  }, [deferredQuery, projectPath, requestKey]);

  const flow = useMemo(() => toFlow(view), [view]);

  return (
    <section className="space-y-5" aria-labelledby="project-graph-title">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <p className="mb-1 text-micro font-semibold uppercase tracking-[0.14em] text-primary">
            Deterministic relationships
          </p>
          <h2
            id="project-graph-title"
            className="text-xl font-semibold tracking-tight"
          >
            Project graph
          </h2>
          <p className="mt-1 max-w-2xl text-meta leading-relaxed text-muted-foreground">
            Explore files, symbols, imports, calls, and dependencies from the
            captured snapshot. Select a node to inspect its evidence.
          </p>
        </div>
        <label className="relative block w-full lg:w-80">
          <span className="sr-only">Search project graph</span>
          <Search
            aria-hidden="true"
            className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
            size={15}
          />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="File, symbol, package, language"
            className="h-10 w-full rounded-lg border border-border bg-surface-1 pl-9 pr-3 text-meta outline-none transition focus:border-primary/60 focus:ring-2 focus:ring-primary/20"
          />
        </label>
      </div>

      {error && !loading ? (
        <div
          role="alert"
          className="flex items-start gap-3 rounded-xl border border-destructive/25 bg-destructive/8 p-4 text-meta"
        >
          <AlertTriangle
            size={17}
            className="mt-0.5 shrink-0 text-destructive"
            aria-hidden="true"
          />
          <div>
            <p className="font-semibold">Could not build project graph view</p>
            <p className="mt-0.5 text-muted-foreground">{error}</p>
          </div>
        </div>
      ) : (
        <div className="overflow-hidden rounded-xl border border-border bg-surface-1">
          <div className="flex min-h-12 flex-wrap items-center justify-between gap-2 border-b border-border px-3 py-2">
            <div className="flex flex-wrap gap-1.5">
              {Object.entries(KIND_COLORS).map(([kind, color]) => (
                <span
                  key={kind}
                  className="inline-flex items-center gap-1.5 rounded-full bg-surface-2 px-2 py-1 text-[10px] font-medium text-muted-foreground"
                >
                  <span
                    className="size-1.5 rounded-full"
                    style={{ backgroundColor: color }}
                  />
                  {humanize(kind)}
                </span>
              ))}
            </div>
            <p className="text-micro text-muted-foreground" aria-live="polite">
              {loading && !view
                ? "Building bounded view…"
                : view
                  ? `${view.nodes.length} of ${view.totalNodes} nodes · ${view.edges.length} of ${view.totalEdges} edges`
                  : ""}
            </p>
          </div>

          <div className="relative h-[34rem] min-h-[24rem] bg-background/60">
            {view && view.nodes.length > 0 ? (
              <ReactFlow<FlowNode, Edge>
                nodes={flow.nodes}
                edges={flow.edges}
                nodesDraggable={false}
                nodesConnectable={false}
                edgesReconnectable={false}
                fitView
                fitViewOptions={{ padding: 0.22, maxZoom: 1 }}
                minZoom={0.12}
                maxZoom={1.75}
                onNodeClick={(_, node) => setSelected(node.data.source)}
                aria-label="Interactive deterministic project graph"
                proOptions={{ hideAttribution: true }}
              >
                <Background
                  variant={BackgroundVariant.Dots}
                  gap={22}
                  size={1}
                  color="var(--border)"
                />
                <Controls showInteractive={false} />
                <MiniMap
                  pannable
                  zoomable
                  nodeColor={(node) =>
                    KIND_COLORS[
                      (node.data?.source as ProjectGraphViewNode).kind
                    ]
                  }
                  maskColor="color-mix(in srgb, var(--background) 72%, transparent)"
                />
              </ReactFlow>
            ) : loading ? (
              <div className="flex h-full items-center justify-center text-meta text-muted-foreground">
                Building bounded view…
              </div>
            ) : (
              <div className="flex h-full flex-col items-center justify-center px-5 text-center">
                <Network
                  size={26}
                  className="mb-3 text-muted-foreground"
                  aria-hidden="true"
                />
                <p className="font-semibold">No graph nodes match</p>
                <p className="mt-1 text-meta text-muted-foreground">
                  Try a file path, symbol, language, or dependency name.
                </p>
              </div>
            )}

            {loading && view && (
              <div className="absolute left-3 top-3 rounded-full border border-border bg-background/90 px-2.5 py-1 text-micro text-muted-foreground shadow-sm backdrop-blur">
                Updating view…
              </div>
            )}

            {selected && (
              <NodeInspector
                node={selected}
                onClose={() => setSelected(null)}
              />
            )}
          </div>

          {view && (
            <div className="grid gap-3 border-t border-border px-4 py-3 text-micro text-muted-foreground sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center">
              <span className="inline-flex min-w-0 items-center gap-2">
                <ShieldCheck
                  size={14}
                  className="shrink-0 text-primary"
                  aria-hidden="true"
                />
                <span className="truncate">
                  {view.selection}. Snapshot {shortId(view.graphSnapshotId)}.
                </span>
              </span>
              <span>
                {view.omittedNodes.toLocaleString()} nodes and{" "}
                {view.omittedEdges.toLocaleString()} edges outside this bounded
                view
              </span>
            </div>
          )}
        </div>
      )}

      {view?.git && (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-2 rounded-xl border border-border bg-surface-1 px-4 py-3 text-micro text-muted-foreground">
          <span className="inline-flex items-center gap-2 font-medium text-foreground">
            <GitBranch size={14} className="text-primary" aria-hidden="true" />
            {view.git.branch ?? "Detached HEAD"}
          </span>
          <span>{view.git.changes.length} local changes at capture</span>
          {(view.git.ahead > 0 || view.git.behind > 0) && (
            <span>
              {view.git.ahead} ahead · {view.git.behind} behind
            </span>
          )}
          <span>Live Git state not checked in this view</span>
        </div>
      )}

      {view && view.diagnostics.length > 0 && (
        <details className="group overflow-hidden rounded-xl border border-warning/25 bg-warning/5">
          <summary className="flex cursor-pointer list-none items-center gap-2 px-4 py-3 text-meta font-medium outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary">
            <AlertTriangle
              size={15}
              className="shrink-0 text-warning"
              aria-hidden="true"
            />
            Capture diagnostics
            <span className="rounded-full bg-warning/10 px-1.5 text-micro text-warning">
              {view.diagnostics.length + view.omittedDiagnostics}
            </span>
          </summary>
          <div className="divide-y divide-warning/15 border-t border-warning/15">
            {view.diagnostics.map((diagnostic, index) => (
              <div
                key={`${diagnostic.artifactPath}:${diagnostic.kind}:${index}`}
                className="grid gap-1 px-4 py-3 text-micro sm:grid-cols-[minmax(0,15rem)_minmax(0,1fr)] sm:gap-4"
              >
                <span className="break-all font-mono text-foreground">
                  {diagnostic.artifactPath}
                </span>
                <span className="text-muted-foreground">
                  {humanize(diagnostic.kind)} · {diagnostic.message}
                </span>
              </div>
            ))}
            {view.omittedDiagnostics > 0 && (
              <p className="px-4 py-3 text-micro text-muted-foreground">
                {view.omittedDiagnostics.toLocaleString()} more diagnostics
                omitted by the response bound.
              </p>
            )}
          </div>
        </details>
      )}
    </section>
  );
}

function NodeInspector({
  node,
  onClose,
}: {
  node: ProjectGraphViewNode;
  onClose: () => void;
}) {
  return (
    <aside className="absolute inset-x-3 bottom-3 z-10 max-h-[70%] overflow-y-auto rounded-xl border border-border bg-background/95 p-4 shadow-xl backdrop-blur sm:inset-x-auto sm:right-3 sm:w-80">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <span className="text-micro font-semibold uppercase tracking-wider text-primary">
            {humanize(node.kind)}
          </span>
          <h3 className="mt-0.5 break-words text-body font-semibold">
            {node.name}
          </h3>
        </div>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close node details"
          className="flex size-8 shrink-0 items-center justify-center rounded-md text-muted-foreground outline-none hover:bg-surface-2 hover:text-foreground focus-visible:ring-2 focus-visible:ring-primary"
        >
          <X size={15} aria-hidden="true" />
        </button>
      </div>
      <dl className="mt-4 grid grid-cols-[6rem_minmax(0,1fr)] gap-x-3 gap-y-2 text-micro">
        {node.path && (
          <>
            <dt className="text-muted-foreground">Path</dt>
            <dd className="break-all font-mono text-foreground">{node.path}</dd>
          </>
        )}
        {node.language && (
          <>
            <dt className="text-muted-foreground">Language</dt>
            <dd>{node.language}</dd>
          </>
        )}
        {node.symbolKind && (
          <>
            <dt className="text-muted-foreground">Symbol</dt>
            <dd>{node.symbolKind}</dd>
          </>
        )}
        {node.packageManager && (
          <>
            <dt className="text-muted-foreground">Package manager</dt>
            <dd>{node.packageManager}</dd>
          </>
        )}
        <dt className="text-muted-foreground">Connections</dt>
        <dd>{node.degree.toLocaleString()}</dd>
        <dt className="text-muted-foreground">Provenance</dt>
        <dd>{humanize(node.provenance)}</dd>
        <dt className="text-muted-foreground">Confidence</dt>
        <dd>{Math.round(node.confidence * 100)}%</dd>
      </dl>
      {node.citation ? (
        <div className="mt-4 rounded-lg border border-border bg-surface-1 p-3 text-micro">
          <p className="font-semibold">Source evidence</p>
          <p className="mt-1 break-all font-mono text-muted-foreground">
            {node.citation.artifactPath}:{node.citation.startLine}
            {node.citation.endLine !== node.citation.startLine
              ? `–${node.citation.endLine}`
              : ""}
          </p>
          <p className="mt-1 text-muted-foreground">
            Snapshot {shortId(node.citation.artifactSnapshotId)} · content{" "}
            {shortId(node.citation.contentHash)}
          </p>
        </div>
      ) : (
        <p className="mt-4 rounded-lg bg-surface-1 p-3 text-micro text-muted-foreground">
          This aggregate node has no single source range.
        </p>
      )}
    </aside>
  );
}

function toFlow(view: ProjectGraphView | null): {
  nodes: FlowNode[];
  edges: Edge[];
} {
  if (!view) return { nodes: [], edges: [] };
  const byKind = new Map<ProjectGraphNodeKind, ProjectGraphViewNode[]>();
  for (const node of view.nodes) {
    const group = byKind.get(node.kind) ?? [];
    group.push(node);
    byKind.set(node.kind, group);
  }
  for (const group of byKind.values()) {
    group.sort(
      (left, right) =>
        right.degree - left.degree ||
        (left.path ?? left.name).localeCompare(right.path ?? right.name),
    );
  }

  const nodes: FlowNode[] = [];
  for (const [kind, group] of byKind) {
    group.forEach((source, index) => {
      const column = Math.floor(index / 24);
      const row = index % 24;
      const x =
        KIND_LANES[kind] + column * 230 + (kind === "dependency" ? 90 : 0);
      const y =
        row * 78 +
        (kind === "dependency" ? 38 : 0) +
        (kind === "project" ? 80 : 0);
      nodes.push({
        id: source.id,
        type: "default",
        position: { x, y },
        data: {
          label: truncateLabel(source.name),
          source,
        },
        style: {
          width: kind === "project" ? 180 : 205,
          minHeight: 42,
          borderRadius: 10,
          border: `1px solid ${KIND_COLORS[kind]}80`,
          borderLeft: `4px solid ${KIND_COLORS[kind]}`,
          background: "var(--surface-1)",
          color: "var(--foreground)",
          fontSize: 12,
          fontWeight: 600,
          padding: "10px 12px",
          boxShadow: "0 4px 16px rgb(0 0 0 / 0.08)",
        },
        selectable: true,
        draggable: false,
      });
    });
  }

  const edges: Edge[] = view.edges.map((source) => ({
    id: source.id,
    source: source.source,
    target: source.target,
    label: source.label,
    type: "smoothstep",
    animated: false,
    style: {
      stroke: "var(--muted-foreground)",
      strokeOpacity: 0.3,
      strokeWidth: source.kind === "calls" ? 1.6 : 1,
    },
    labelStyle: {
      fill: "var(--muted-foreground)",
      fontSize: 9,
    },
  }));
  return { nodes, edges };
}

function truncateLabel(value: string): string {
  return value.length > 28 ? `${value.slice(0, 27)}…` : value;
}

function humanize(value: string): string {
  return value
    .replaceAll("-", " ")
    .replace(/\b\w/g, (character) => character.toUpperCase());
}

function shortId(value: string): string {
  return value.length > 12 ? `${value.slice(0, 12)}…` : value;
}
