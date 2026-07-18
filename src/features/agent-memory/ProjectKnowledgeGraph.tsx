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
  Check,
  ChevronDown,
  Clock3,
  FileCode2,
  Filter,
  GitBranch,
  Network,
  Search,
  ShieldCheck,
  X,
} from "lucide-react";
import {
  readAgentCitedEvidence,
  readAgentProjectGraphEvidence,
  readAgentProjectGraphHistory,
  readAgentProjectGraphView,
} from "./api";
import { cn } from "@/shared/lib/classnames";
import type {
  ArtifactEvidenceReference,
  GraphCitation,
  ProjectGraphEdgeKind,
  ProjectGraphEvidenceExcerpt,
  ProjectGraphFilters,
  ProjectGraphHistory,
  ProjectGraphNodeKind,
  ProjectGraphProvenance,
  ProjectGraphView,
  ProjectGraphViewEdge,
  ProjectGraphViewNode,
} from "./types";

type FlowNode = Node<
  { label: string; source: ProjectGraphViewNode },
  "default"
>;
type FlowEdge = Edge<{ source: ProjectGraphViewEdge }>;
type InspectionTarget =
  | { kind: "node"; value: ProjectGraphViewNode }
  | { kind: "edge"; value: ProjectGraphViewEdge };

const NODE_KINDS: ProjectGraphNodeKind[] = [
  "project",
  "file",
  "dependency",
  "symbol",
  "external-module",
  "external-symbol",
];
const EDGE_KINDS: ProjectGraphEdgeKind[] = [
  "contains",
  "defines",
  "imports",
  "calls",
  "inherits",
  "implements",
  "references",
  "depends-on",
];
const PROVENANCES: ProjectGraphProvenance[] = [
  "deterministic",
  "user-authored",
  "agent-authored",
  "inferred",
];

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

const CAPTURE_DATE = new Intl.DateTimeFormat(undefined, {
  dateStyle: "medium",
  timeStyle: "short",
});
const NUMBER = new Intl.NumberFormat();

export function ProjectKnowledgeGraph({
  projectPath,
  focus = null,
  onOpenArtifact = () => undefined,
}: {
  projectPath: string;
  focus?: {
    evidence: ArtifactEvidenceReference;
    requestId: number;
  } | null;
  onOpenArtifact?: (path: string) => void;
}) {
  const [query, setQuery] = useState(focus?.evidence.artifactPath ?? "");
  const [deferredQuery, setDeferredQuery] = useState(
    focus?.evidence.artifactPath ?? "",
  );
  const [historyResult, setHistoryResult] = useState<{
    projectPath: string;
    data?: ProjectGraphHistory;
    error?: string;
  } | null>(null);
  const [snapshotSelection, setSnapshotSelection] = useState<{
    projectPath: string;
    snapshotId: string;
  } | null>(null);
  const [filters, setFilters] = useState<ProjectGraphFilters>({
    nodeKinds: NODE_KINDS,
    edgeKinds: EDGE_KINDS,
    provenances: PROVENANCES,
  });
  const [view, setView] = useState<ProjectGraphView | null>(null);
  const [inspection, setInspection] = useState<InspectionTarget | null>(null);
  const [dismissedFocusId, setDismissedFocusId] = useState<number | null>(null);
  const [completedRequest, setCompletedRequest] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const history =
    historyResult?.projectPath === projectPath
      ? (historyResult.data ?? null)
      : null;
  const historyError =
    historyResult?.projectPath === projectPath
      ? (historyResult.error ?? null)
      : null;
  const focusedCapture = focus
    ? history?.entries.find(
        (entry) =>
          entry.artifactSnapshotId === focus.evidence.artifactSnapshotId,
      )
    : undefined;
  const selectedSnapshotId =
    snapshotSelection?.projectPath === projectPath
      ? snapshotSelection.snapshotId
      : focusedCapture && !focusedCapture.current
        ? focusedCapture.graphSnapshotId
        : null;
  const filterKey = `${filters.nodeKinds.join(",")}|${filters.edgeKinds.join(",")}|${filters.provenances.join(",")}`;
  const requestKey = `${projectPath}\u0000${selectedSnapshotId ?? "current"}\u0000${deferredQuery}\u0000${filterKey}`;
  const loading = completedRequest !== requestKey;
  const activeFilterGroups =
    Number(filters.nodeKinds.length !== NODE_KINDS.length) +
    Number(filters.edgeKinds.length !== EDGE_KINDS.length) +
    Number(filters.provenances.length !== PROVENANCES.length);

  useEffect(() => {
    const timer = window.setTimeout(() => setDeferredQuery(query), 240);
    return () => window.clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    let current = true;
    void readAgentProjectGraphHistory(projectPath)
      .then((next) => {
        if (current) setHistoryResult({ projectPath, data: next });
      })
      .catch((cause) => {
        if (current)
          setHistoryResult({
            projectPath,
            error: cause instanceof Error ? cause.message : String(cause),
          });
      });
    return () => {
      current = false;
    };
  }, [projectPath]);

  useEffect(() => {
    let current = true;
    void readAgentProjectGraphView(
      projectPath,
      deferredQuery,
      selectedSnapshotId ?? undefined,
      filters,
    )
      .then((next) => {
        if (current) {
          setView(next);
          setInspection(null);
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
  }, [
    deferredQuery,
    filterKey,
    filters,
    projectPath,
    requestKey,
    selectedSnapshotId,
  ]);

  const flow = useMemo(() => toFlow(view), [view]);
  const nodesById = useMemo(
    () => new Map(view?.nodes.map((node) => [node.id, node]) ?? []),
    [view],
  );
  const historical =
    history !== null &&
    view !== null &&
    view.graphSnapshotId !== history.currentGraphSnapshotId;

  function toggleNodeKind(kind: ProjectGraphNodeKind) {
    setFilters((current) => ({
      ...current,
      nodeKinds: toggleRequiredFilter(current.nodeKinds, kind),
    }));
  }

  function toggleEdgeKind(kind: ProjectGraphEdgeKind) {
    setFilters((current) => ({
      ...current,
      edgeKinds: toggleRequiredFilter(current.edgeKinds, kind),
    }));
  }

  function toggleProvenance(provenance: ProjectGraphProvenance) {
    setFilters((current) => ({
      ...current,
      provenances: toggleRequiredFilter(current.provenances, provenance),
    }));
  }

  function resetFilters() {
    setFilters({
      nodeKinds: NODE_KINDS,
      edgeKinds: EDGE_KINDS,
      provenances: PROVENANCES,
    });
  }

  return (
    <section className="space-y-5" aria-labelledby="project-graph-title">
      <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
        <div>
          <p className="mb-1 text-micro font-semibold uppercase tracking-[0.14em] text-primary">
            Captured relationships
          </p>
          <h2
            id="project-graph-title"
            className="text-xl font-semibold tracking-tight text-balance"
          >
            Project Graph
          </h2>
          <p className="mt-1 max-w-2xl text-meta leading-relaxed text-muted-foreground">
            Travel through immutable captures, isolate relationship types, and
            inspect the exact redacted source behind each fact.
          </p>
        </div>
        <div className="grid w-full gap-2 sm:grid-cols-2 xl:w-auto xl:grid-cols-[15rem_20rem]">
          <label className="grid gap-1 text-micro font-medium text-muted-foreground">
            Captured view
            <span className="relative">
              <Clock3
                aria-hidden="true"
                className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2"
                size={15}
              />
              <select
                name="graph-capture"
                autoComplete="off"
                value={
                  selectedSnapshotId ??
                  history?.currentGraphSnapshotId ??
                  view?.graphSnapshotId ??
                  ""
                }
                onChange={(event) => {
                  const snapshotId = event.target.value;
                  setSnapshotSelection(
                    snapshotId === history?.currentGraphSnapshotId
                      ? null
                      : { projectPath, snapshotId },
                  );
                }}
                disabled={!history || history.entries.length === 0}
                className="h-10 w-full appearance-none rounded-lg border border-border bg-surface-1 pl-9 pr-9 text-meta text-foreground outline-none transition-[border-color,box-shadow] focus-visible:border-primary/60 focus-visible:ring-2 focus-visible:ring-primary/20 disabled:cursor-not-allowed disabled:opacity-60"
              >
                {!history && (
                  <option value={view?.graphSnapshotId ?? ""}>
                    Loading captures…
                  </option>
                )}
                {history?.entries.map((entry) => (
                  <option
                    key={entry.graphSnapshotId}
                    value={entry.graphSnapshotId}
                  >
                    {entry.current ? "Current · " : ""}
                    {formatCaptureDate(entry.generatedAtUnixMs)}
                  </option>
                ))}
              </select>
              <ChevronDown
                aria-hidden="true"
                className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground"
                size={15}
              />
            </span>
          </label>
          <label className="grid gap-1 text-micro font-medium text-muted-foreground">
            Search graph
            <span className="relative">
              <Search
                aria-hidden="true"
                className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2"
                size={15}
              />
              <input
                type="search"
                name="graph-search"
                autoComplete="off"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="File, symbol, package…"
                className="h-10 w-full rounded-lg border border-border bg-surface-1 pl-9 pr-3 text-meta text-foreground outline-none transition-[border-color,box-shadow] placeholder:text-muted-foreground/70 focus-visible:border-primary/60 focus-visible:ring-2 focus-visible:ring-primary/20"
              />
            </span>
          </label>
        </div>
      </div>

      {historyError && (
        <p role="status" className="text-micro text-warning">
          Capture history is unavailable; the current graph remains usable.{" "}
          {historyError}
        </p>
      )}

      {historical && view && (
        <div className="flex items-start gap-3 rounded-xl border border-primary/20 bg-primary/6 px-4 py-3 text-meta">
          <Clock3
            size={17}
            className="mt-0.5 shrink-0 text-primary"
            aria-hidden="true"
          />
          <div className="min-w-0">
            <p className="font-semibold">Viewing an immutable capture</p>
            <p className="mt-0.5 text-muted-foreground">
              Captured {formatCaptureDate(view.generatedAtUnixMs)}. Search,
              relationships, Git state, and source evidence all stay pinned to
              this point in time; live files are not checked.
            </p>
          </div>
        </div>
      )}

      <GraphFilters
        filters={filters}
        activeGroups={activeFilterGroups}
        onToggleNode={toggleNodeKind}
        onToggleEdge={toggleEdgeKind}
        onToggleProvenance={toggleProvenance}
        onReset={resetFilters}
      />

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
            <p className="font-semibold">Could not build this graph view</p>
            <p className="mt-0.5 text-muted-foreground">{error}</p>
          </div>
        </div>
      ) : (
        <div className="overflow-hidden rounded-xl border border-border bg-surface-1">
          <div className="flex min-h-12 flex-wrap items-center justify-between gap-2 border-b border-border px-3 py-2">
            <div className="flex flex-wrap gap-1.5" aria-label="Node legend">
              {NODE_KINDS.filter((kind) =>
                filters.nodeKinds.includes(kind),
              ).map((kind) => (
                <span
                  key={kind}
                  className="inline-flex items-center gap-1.5 rounded-full bg-surface-2 px-2 py-1 text-[10px] font-medium text-muted-foreground"
                >
                  <span
                    className="size-1.5 rounded-full"
                    style={{ backgroundColor: KIND_COLORS[kind] }}
                  />
                  {humanize(kind)}
                </span>
              ))}
            </div>
            <p
              className="text-micro tabular-nums text-muted-foreground"
              aria-live="polite"
            >
              {loading && !view
                ? "Building bounded view…"
                : view
                  ? `${NUMBER.format(view.nodes.length)} of ${NUMBER.format(view.filteredNodes)} filtered nodes · ${NUMBER.format(view.edges.length)} of ${NUMBER.format(view.filteredEdges)} filtered links`
                  : ""}
            </p>
          </div>

          <div className="project-graph-canvas relative h-[36rem] min-h-[24rem]">
            {view && view.nodes.length > 0 ? (
              <ReactFlow<FlowNode, FlowEdge>
                nodes={flow.nodes}
                edges={flow.edges}
                nodesDraggable={false}
                nodesConnectable={false}
                edgesReconnectable={false}
                fitView
                fitViewOptions={{ padding: 0.22, maxZoom: 1 }}
                minZoom={0.12}
                maxZoom={1.75}
                onNodeClick={(_, node) =>
                  setInspection({ kind: "node", value: node.data.source })
                }
                onEdgeClick={(_, edge) =>
                  setInspection({ kind: "edge", value: edge.data!.source })
                }
                onPaneClick={() => setInspection(null)}
                aria-label="Interactive captured project graph"
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
                  nodeColor={(node) => {
                    const source = node.data?.source as
                      ProjectGraphViewNode | undefined;
                    return source ? KIND_COLORS[source.kind] : "#64748b";
                  }}
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
                <p className="font-semibold">No graph facts match</p>
                <p className="mt-1 max-w-sm text-meta text-muted-foreground">
                  Change the search or include more node, relationship, or
                  provenance filters.
                </p>
              </div>
            )}

            {loading && view && (
              <div
                role="status"
                className="absolute left-3 top-3 rounded-full border border-border bg-background/90 px-2.5 py-1 text-micro text-muted-foreground shadow-sm backdrop-blur"
              >
                Updating view…
              </div>
            )}

            {inspection && view && (
              <GraphInspector
                projectPath={projectPath}
                graphSnapshotId={view.graphSnapshotId}
                target={inspection}
                edges={view.edges}
                nodesById={nodesById}
                onInspect={setInspection}
                onClose={() => setInspection(null)}
              />
            )}
            {view &&
              focus &&
              dismissedFocusId !== focus.requestId &&
              !inspection && (
                <FocusedEvidenceInspector
                  projectPath={projectPath}
                  evidence={focus.evidence}
                  onOpenArtifact={onOpenArtifact}
                  onClose={() => setDismissedFocusId(focus.requestId)}
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
                  {view.selection}. Snapshot{" "}
                  <span translate="no">{shortId(view.graphSnapshotId)}</span>.
                </span>
              </span>
              <span className="tabular-nums">
                {NUMBER.format(view.omittedNodes)} nodes and{" "}
                {NUMBER.format(view.omittedEdges)} links outside this bounded
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
            <span translate="no">{view.git.branch ?? "Detached HEAD"}</span>
          </span>
          <span className="tabular-nums">
            {NUMBER.format(view.git.changes.length)} local changes at capture
          </span>
          {(view.git.ahead > 0 || view.git.behind > 0) && (
            <span className="tabular-nums">
              {NUMBER.format(view.git.ahead)} ahead ·{" "}
              {NUMBER.format(view.git.behind)} behind
            </span>
          )}
          <span>Live Git state not checked</span>
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
            <span className="rounded-full bg-warning/10 px-1.5 text-micro tabular-nums text-warning">
              {NUMBER.format(view.diagnostics.length + view.omittedDiagnostics)}
            </span>
          </summary>
          <div className="divide-y divide-warning/15 border-t border-warning/15">
            {view.diagnostics.map((diagnostic, index) => (
              <div
                key={`${diagnostic.artifactPath}:${diagnostic.kind}:${index}`}
                className="grid gap-1 px-4 py-3 text-micro sm:grid-cols-[minmax(0,15rem)_minmax(0,1fr)] sm:gap-4"
              >
                <span
                  translate="no"
                  className="break-all font-mono text-foreground"
                >
                  {diagnostic.artifactPath}
                </span>
                <span className="text-muted-foreground">
                  {humanize(diagnostic.kind)} · {diagnostic.message}
                </span>
              </div>
            ))}
            {view.omittedDiagnostics > 0 && (
              <p className="px-4 py-3 text-micro tabular-nums text-muted-foreground">
                {NUMBER.format(view.omittedDiagnostics)} more diagnostics
                omitted by the response bound.
              </p>
            )}
          </div>
        </details>
      )}

      {history && history.omittedEntries > 0 && (
        <p className="text-micro text-muted-foreground">
          Showing the newest {NUMBER.format(history.entries.length)} of{" "}
          {NUMBER.format(history.totalEntries)} indexed captures.
        </p>
      )}
    </section>
  );
}

function GraphFilters({
  filters,
  activeGroups,
  onToggleNode,
  onToggleEdge,
  onToggleProvenance,
  onReset,
}: {
  filters: ProjectGraphFilters;
  activeGroups: number;
  onToggleNode: (kind: ProjectGraphNodeKind) => void;
  onToggleEdge: (kind: ProjectGraphEdgeKind) => void;
  onToggleProvenance: (provenance: ProjectGraphProvenance) => void;
  onReset: () => void;
}) {
  return (
    <details className="group overflow-hidden rounded-xl border border-border bg-surface-1">
      <summary className="flex min-h-11 cursor-pointer list-none items-center justify-between gap-3 px-4 py-2 text-meta font-medium outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary">
        <span className="inline-flex items-center gap-2">
          <Filter size={15} className="text-primary" aria-hidden="true" />
          Graph filters
          {activeGroups > 0 && (
            <span className="rounded-full bg-primary/10 px-1.5 text-micro tabular-nums text-primary">
              {activeGroups}
            </span>
          )}
        </span>
        <ChevronDown
          size={15}
          className="text-muted-foreground transition-transform duration-200 motion-reduce:transition-none group-open:rotate-180"
          aria-hidden="true"
        />
      </summary>
      <div className="grid gap-5 border-t border-border p-4 lg:grid-cols-3">
        <FilterGroup
          legend="Node types"
          description="Choose which facts can become nodes."
          values={NODE_KINDS}
          selected={filters.nodeKinds}
          onToggle={onToggleNode}
        />
        <FilterGroup
          legend="Relationships"
          description="Limit the links used for layout and degree."
          values={EDGE_KINDS}
          selected={filters.edgeKinds}
          onToggle={onToggleEdge}
        />
        <FilterGroup
          legend="Provenance"
          description="Trace who or what established each fact."
          values={PROVENANCES}
          selected={filters.provenances}
          onToggle={onToggleProvenance}
        />
      </div>
      {activeGroups > 0 && (
        <div className="flex justify-end border-t border-border px-4 py-2">
          <button
            type="button"
            onClick={onReset}
            className="rounded-md px-2.5 py-1.5 text-micro font-medium text-primary outline-none transition-transform hover:bg-primary/8 active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
          >
            Reset filters
          </button>
        </div>
      )}
    </details>
  );
}

function FilterGroup<T extends string>({
  legend,
  description,
  values,
  selected,
  onToggle,
}: {
  legend: string;
  description: string;
  values: T[];
  selected: T[];
  onToggle: (value: T) => void;
}) {
  return (
    <fieldset className="min-w-0">
      <legend className="text-meta font-semibold">{legend}</legend>
      <p className="mb-2 mt-0.5 text-micro leading-relaxed text-muted-foreground">
        {description}
      </p>
      <div className="flex flex-wrap gap-1.5">
        {values.map((value) => {
          const checked = selected.includes(value);
          const onlySelection = checked && selected.length === 1;
          return (
            <label
              key={value}
              title={
                onlySelection
                  ? `Keep at least one ${legend.toLowerCase()} filter selected`
                  : undefined
              }
              className="inline-flex min-h-8 cursor-pointer touch-manipulation items-center gap-1.5 rounded-lg border border-border bg-background px-2.5 py-1.5 text-micro text-muted-foreground transition-[transform,border-color,background-color,color] hover:border-primary/30 hover:text-foreground active:scale-[0.97] motion-reduce:transform-none has-[:disabled]:cursor-not-allowed has-[:disabled]:opacity-60 has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-primary has-[:checked]:border-primary/30 has-[:checked]:bg-primary/8 has-[:checked]:text-foreground"
            >
              <input
                type="checkbox"
                checked={checked}
                disabled={onlySelection}
                onChange={() => onToggle(value)}
                className="sr-only"
              />
              <span
                className="flex size-3.5 items-center justify-center rounded border border-current"
                aria-hidden="true"
              >
                {checked && <Check size={11} strokeWidth={3} />}
              </span>
              {humanize(value)}
            </label>
          );
        })}
      </div>
    </fieldset>
  );
}

function FocusedEvidenceInspector({
  projectPath,
  evidence,
  onOpenArtifact,
  onClose,
}: {
  projectPath: string;
  evidence: ArtifactEvidenceReference;
  onOpenArtifact: (path: string) => void;
  onClose: () => void;
}) {
  const citation: GraphCitation = {
    ...evidence,
    startColumn: 1,
    endColumn: 1,
  };
  return (
    <aside
      aria-label="Memory evidence inspector"
      className="graph-source-inspector absolute inset-x-3 bottom-3 z-10 max-h-[82%] overflow-y-auto overscroll-contain rounded-xl border p-4 sm:inset-x-auto sm:right-3 sm:w-[27rem]"
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <span className="text-micro font-semibold uppercase tracking-wider text-primary">
            Memory evidence
          </span>
          <h3 className="mt-0.5 break-words text-body font-semibold text-pretty">
            Captured source at the time
          </h3>
        </div>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close memory evidence"
          className="flex size-8 shrink-0 touch-manipulation items-center justify-center rounded-md text-muted-foreground outline-none transition-transform hover:bg-surface-2 hover:text-foreground active:scale-90 motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
        >
          <X size={15} aria-hidden="true" />
        </button>
      </div>
      <p className="mt-3 text-micro leading-5 text-muted-foreground">
        This excerpt is read from the immutable local snapshot cited by the
        session—not from today&apos;s working tree.
      </p>
      <CapturedSource
        projectPath={projectPath}
        graphSnapshotId=""
        citation={citation}
        direct
      />
      <button
        type="button"
        onClick={() => onOpenArtifact(evidence.artifactPath)}
        className="mt-3 inline-flex h-9 touch-manipulation items-center gap-2 rounded-lg border border-border bg-surface-1 px-3 text-micro font-semibold text-foreground outline-none transition-[transform,border-color,background-color] hover:border-primary/35 hover:bg-surface-2 active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
      >
        <FileCode2 size={14} aria-hidden="true" />
        Open artifact record
      </button>
    </aside>
  );
}

function GraphInspector({
  projectPath,
  graphSnapshotId,
  target,
  edges,
  nodesById,
  onInspect,
  onClose,
}: {
  projectPath: string;
  graphSnapshotId: string;
  target: InspectionTarget;
  edges: ProjectGraphViewEdge[];
  nodesById: Map<string, ProjectGraphViewNode>;
  onInspect: (target: InspectionTarget) => void;
  onClose: () => void;
}) {
  useEffect(() => {
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose]);

  const node = target.kind === "node" ? target.value : null;
  const edge = target.kind === "edge" ? target.value : null;
  const citation = node?.citation ?? edge?.citation;
  const connections = node
    ? edges.filter(
        (candidate) =>
          candidate.source === node.id || candidate.target === node.id,
      )
    : [];
  const sourceNode = edge ? nodesById.get(edge.source) : undefined;
  const targetNode = edge ? nodesById.get(edge.target) : undefined;

  return (
    <aside
      aria-label="Graph source inspector"
      className="graph-source-inspector absolute inset-x-3 bottom-3 z-10 max-h-[82%] overscroll-contain overflow-y-auto rounded-xl border p-4 sm:inset-x-auto sm:right-3 sm:w-[27rem]"
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <span className="text-micro font-semibold uppercase tracking-wider text-primary">
            {node ? humanize(node.kind) : humanize(edge!.kind)}
          </span>
          <h3 className="mt-0.5 break-words text-body font-semibold text-pretty">
            {node
              ? node.name
              : `${sourceNode?.name ?? "Visible node"} → ${targetNode?.name ?? "Visible node"}`}
          </h3>
        </div>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close graph inspector"
          className="flex size-8 shrink-0 touch-manipulation items-center justify-center rounded-md text-muted-foreground outline-none transition-transform hover:bg-surface-2 hover:text-foreground active:scale-90 motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
        >
          <X size={15} aria-hidden="true" />
        </button>
      </div>

      <dl className="mt-4 grid grid-cols-[6.5rem_minmax(0,1fr)] gap-x-3 gap-y-2 text-micro">
        {node?.path && (
          <>
            <dt className="text-muted-foreground">Path</dt>
            <dd translate="no" className="break-all font-mono text-foreground">
              {node.path}
            </dd>
          </>
        )}
        {node?.language && (
          <>
            <dt className="text-muted-foreground">Language</dt>
            <dd translate="no">{node.language}</dd>
          </>
        )}
        {node?.symbolKind && (
          <>
            <dt className="text-muted-foreground">Symbol</dt>
            <dd>{node.symbolKind}</dd>
          </>
        )}
        {node?.packageManager && (
          <>
            <dt className="text-muted-foreground">Package manager</dt>
            <dd translate="no">{node.packageManager}</dd>
          </>
        )}
        {edge?.label && (
          <>
            <dt className="text-muted-foreground">Label</dt>
            <dd translate="no" className="break-words">
              {edge.label}
            </dd>
          </>
        )}
        <dt className="text-muted-foreground">
          {node ? "Connections" : "Relationship"}
        </dt>
        <dd className="tabular-nums">
          {node ? NUMBER.format(node.degree) : humanize(edge!.kind)}
        </dd>
        <dt className="text-muted-foreground">Provenance</dt>
        <dd>{humanize(node?.provenance ?? edge!.provenance)}</dd>
        <dt className="text-muted-foreground">Confidence</dt>
        <dd className="tabular-nums">
          {Math.round((node?.confidence ?? edge!.confidence) * 100)}%
        </dd>
      </dl>

      {edge && sourceNode && targetNode && (
        <div className="mt-4">
          <p className="text-micro font-semibold">Endpoints</p>
          <div className="mt-2 grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-2">
            <button
              type="button"
              onClick={() => onInspect({ kind: "node", value: sourceNode })}
              className="min-w-0 touch-manipulation truncate rounded-lg border border-border bg-surface-1 px-2.5 py-2 text-left text-micro font-medium outline-none transition-[transform,border-color,background-color] hover:border-primary/30 hover:bg-surface-2 active:scale-[0.98] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
              translate="no"
            >
              {sourceNode.name}
            </button>
            <span className="text-muted-foreground" aria-hidden="true">
              →
            </span>
            <button
              type="button"
              onClick={() => onInspect({ kind: "node", value: targetNode })}
              className="min-w-0 touch-manipulation truncate rounded-lg border border-border bg-surface-1 px-2.5 py-2 text-left text-micro font-medium outline-none transition-[transform,border-color,background-color] hover:border-primary/30 hover:bg-surface-2 active:scale-[0.98] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
              translate="no"
            >
              {targetNode.name}
            </button>
          </div>
        </div>
      )}

      {connections.length > 0 && (
        <div className="mt-4">
          <p className="text-micro font-semibold">Visible connections</p>
          <div className="mt-2 grid max-h-32 gap-1 overflow-y-auto overscroll-contain pr-1">
            {connections.map((connection) => {
              const neighborId =
                connection.source === node!.id
                  ? connection.target
                  : connection.source;
              const neighbor = nodesById.get(neighborId);
              return (
                <button
                  key={connection.id}
                  type="button"
                  onClick={() => onInspect({ kind: "edge", value: connection })}
                  className="flex min-w-0 touch-manipulation items-center justify-between gap-3 rounded-lg px-2.5 py-2 text-left text-micro outline-none transition-transform hover:bg-surface-2 active:scale-[0.985] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
                >
                  <span className="min-w-0 truncate">
                    {humanize(connection.kind)} ·{" "}
                    <span translate="no">{neighbor?.name ?? neighborId}</span>
                  </span>
                  <span className="shrink-0 text-muted-foreground">
                    Inspect
                  </span>
                </button>
              );
            })}
          </div>
        </div>
      )}

      {citation ? (
        <CapturedSource
          projectPath={projectPath}
          graphSnapshotId={graphSnapshotId}
          citation={citation}
        />
      ) : (
        <p className="mt-4 rounded-lg bg-surface-1 p-3 text-micro text-muted-foreground">
          This aggregate fact has no single captured source range.
        </p>
      )}
    </aside>
  );
}

function CapturedSource({
  projectPath,
  graphSnapshotId,
  citation,
  direct = false,
}: {
  projectPath: string;
  graphSnapshotId: string;
  citation: GraphCitation;
  direct?: boolean;
}) {
  const evidenceKey = `${projectPath}\u0000${direct ? "direct" : graphSnapshotId}\u0000${citation.artifactSnapshotId}\u0000${citation.artifactPath}\u0000${citation.startLine}\u0000${citation.endLine}\u0000${citation.contentHash}`;
  const [result, setResult] = useState<{
    key: string;
    evidence?: ProjectGraphEvidenceExcerpt;
    error?: string;
  } | null>(null);
  const currentResult = result?.key === evidenceKey ? result : null;
  const evidence = currentResult?.evidence ?? null;
  const error = currentResult?.error ?? null;
  const loading = currentResult === null;

  useEffect(() => {
    let current = true;
    const request = direct
      ? readAgentCitedEvidence(projectPath, citation)
      : readAgentProjectGraphEvidence(projectPath, graphSnapshotId, citation);
    void request
      .then((next) => {
        if (current) setResult({ key: evidenceKey, evidence: next });
      })
      .catch((cause) => {
        if (current)
          setResult({
            key: evidenceKey,
            error: cause instanceof Error ? cause.message : String(cause),
          });
      });
    return () => {
      current = false;
    };
  }, [citation, direct, evidenceKey, graphSnapshotId, projectPath]);

  return (
    <div className="mt-4 overflow-hidden rounded-lg border border-border bg-surface-1">
      <div className="border-b border-border px-3 py-2">
        <p className="text-micro font-semibold">Captured source</p>
        <p
          translate="no"
          className="mt-0.5 break-all font-mono text-[10px] text-muted-foreground"
        >
          {citation.artifactPath}:{citation.startLine}
          {citation.endLine !== citation.startLine
            ? `–${citation.endLine}`
            : ""}
        </p>
      </div>
      <div>
        {loading ? (
          <p role="status" className="p-3 text-micro text-muted-foreground">
            Verifying captured source…
          </p>
        ) : error ? (
          <div role="alert" className="flex items-start gap-2 p-3 text-micro">
            <AlertTriangle
              size={14}
              className="mt-0.5 shrink-0 text-warning"
              aria-hidden="true"
            />
            <p className="text-muted-foreground">
              {error} Refresh the project with Structured capture to retain
              inspectable redacted text.
            </p>
          </div>
        ) : evidence ? (
          <>
            <div
              translate="no"
              className="max-h-64 overflow-auto overscroll-contain bg-background/70 py-2 font-mono text-[11px] leading-5"
            >
              {sourceLines(evidence).map((line) => (
                <div
                  key={`${line.number}:${line.text}`}
                  className={cn(
                    "grid min-w-max grid-cols-[3.25rem_minmax(0,1fr)] border-l-2 px-2",
                    line.number >= citation.startLine &&
                      line.number <= citation.endLine
                      ? "border-primary bg-primary/8"
                      : "border-transparent",
                  )}
                >
                  <span
                    aria-hidden="true"
                    className="select-none pr-3 text-right tabular-nums text-muted-foreground/60"
                  >
                    {line.number}
                  </span>
                  <code className="whitespace-pre pr-3 text-foreground">
                    {line.text || " "}
                  </code>
                </div>
              ))}
            </div>
            <div className="border-t border-border px-3 py-2 text-[10px] leading-relaxed text-muted-foreground">
              Redacted snapshot evidence · live source not checked
              {evidence.truncated ? " · excerpt bounded" : ""}
            </div>
          </>
        ) : null}
      </div>
    </div>
  );
}

function toFlow(view: ProjectGraphView | null): {
  nodes: FlowNode[];
  edges: FlowEdge[];
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

  const edges: FlowEdge[] = view.edges.map((source) => ({
    id: source.id,
    source: source.source,
    target: source.target,
    label: source.label,
    type: "smoothstep",
    animated: false,
    data: { source },
    style: {
      stroke: "var(--muted-foreground)",
      strokeOpacity: 0.32,
      strokeWidth: source.kind === "calls" ? 1.7 : 1,
      cursor: "pointer",
    },
    labelStyle: {
      fill: "var(--muted-foreground)",
      fontSize: 9,
    },
  }));
  return { nodes, edges };
}

function toggleRequiredFilter<T>(values: T[], value: T): T[] {
  if (!values.includes(value)) return [...values, value];
  if (values.length === 1) return values;
  return values.filter((candidate) => candidate !== value);
}

function sourceLines(evidence: ProjectGraphEvidenceExcerpt): Array<{
  number: number;
  text: string;
}> {
  const lines = evidence.text.split("\n");
  return lines.map((text, index) => ({
    number: evidence.citation.startLine + index,
    text,
  }));
}

function formatCaptureDate(unixMs: number): string {
  return CAPTURE_DATE.format(new Date(unixMs));
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
