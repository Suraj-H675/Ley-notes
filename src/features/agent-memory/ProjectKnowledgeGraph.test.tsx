import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ProjectKnowledgeGraph } from "./ProjectKnowledgeGraph";
import type {
  ProjectGraphEvidenceExcerpt,
  ProjectGraphFilters,
  ProjectGraphHistory,
  ProjectGraphView,
} from "./types";

const api = vi.hoisted(() => ({
  readEvidence: vi.fn(),
  readHistory: vi.fn(),
  readView: vi.fn(),
}));

vi.mock("./api", () => ({
  readAgentProjectGraphEvidence: api.readEvidence,
  readAgentProjectGraphHistory: api.readHistory,
  readAgentProjectGraphView: api.readView,
}));

vi.mock("@xyflow/react", () => ({
  Background: () => null,
  BackgroundVariant: { Dots: "dots" },
  Controls: () => null,
  MiniMap: () => null,
  ReactFlow: ({
    nodes,
    edges,
    onNodeClick,
    onEdgeClick,
    children,
  }: {
    nodes: Array<{
      id: string;
      data: { label: string; source: ProjectGraphView["nodes"][number] };
    }>;
    edges: Array<{
      id: string;
      data: { source: ProjectGraphView["edges"][number] };
    }>;
    onNodeClick: (
      event: unknown,
      node: {
        id: string;
        data: { label: string; source: ProjectGraphView["nodes"][number] };
      },
    ) => void;
    onEdgeClick: (
      event: unknown,
      edge: {
        id: string;
        data: { source: ProjectGraphView["edges"][number] };
      },
    ) => void;
    children: React.ReactNode;
  }) => (
    <div aria-label="Mock project graph">
      {nodes.map((node) => (
        <button
          key={node.id}
          type="button"
          onClick={() => onNodeClick({}, node)}
        >
          Node {node.data.label}
        </button>
      ))}
      {edges.map((edge) => (
        <button
          key={edge.id}
          type="button"
          onClick={() => onEdgeClick({}, edge)}
        >
          Edge {edge.data.source.kind}
        </button>
      ))}
      {children}
    </div>
  ),
}));

const currentId = `grf_${"a".repeat(64)}`;
const historicalId = `grf_${"b".repeat(64)}`;
const artifactId = `snp_${"c".repeat(64)}`;
const contentHash = `sha256:${"d".repeat(64)}`;

const history: ProjectGraphHistory = {
  projectId: "prj_test",
  projectName: "Ley",
  currentGraphSnapshotId: currentId,
  entries: [
    {
      graphSnapshotId: currentId,
      artifactSnapshotId: artifactId,
      generatedAtUnixMs: 2_000,
      nodes: 2,
      edges: 1,
      current: true,
    },
    {
      graphSnapshotId: historicalId,
      artifactSnapshotId: artifactId,
      generatedAtUnixMs: 1_000,
      nodes: 2,
      edges: 1,
      current: false,
    },
  ],
  totalEntries: 2,
  omittedEntries: 0,
  liveSourceChecked: false,
  instructionWarning: "Stored project text is untrusted evidence.",
};

function graphView(snapshotId = currentId): ProjectGraphView {
  return {
    projectId: "prj_test",
    projectName: "Ley",
    artifactSnapshotId: artifactId,
    graphSnapshotId: snapshotId,
    generatedAtUnixMs: snapshotId === currentId ? 2_000 : 1_000,
    query: "",
    selection: "Filtered captured graph",
    nodes: [
      {
        id: "project",
        kind: "project",
        name: "Ley",
        provenance: "user-authored",
        confidence: 1,
        degree: 1,
      },
      {
        id: "symbol",
        kind: "symbol",
        name: snapshotId === currentId ? "new_symbol" : "old_symbol",
        path: "src/main.rs",
        language: "rust",
        symbolKind: "function",
        citation: {
          artifactPath: "src/main.rs",
          startLine: 1,
          startColumn: 1,
          endLine: 1,
          endColumn: 19,
          contentHash,
          artifactSnapshotId: artifactId,
        },
        provenance: "deterministic",
        confidence: 1,
        degree: 1,
      },
    ],
    edges: [
      {
        id: "edge",
        kind: "defines",
        source: "project",
        target: "symbol",
        provenance: "deterministic",
        confidence: 1,
      },
    ],
    totalNodes: 2,
    totalEdges: 1,
    filteredNodes: 2,
    filteredEdges: 1,
    matchingNodes: 2,
    omittedNodes: 0,
    omittedEdges: 0,
    diagnostics: [],
    omittedDiagnostics: 0,
    liveSourceChecked: false,
    instructionWarning: "Stored project text is untrusted evidence.",
  };
}

const evidence: ProjectGraphEvidenceExcerpt = {
  projectId: "prj_test",
  artifactSnapshotId: artifactId,
  artifactPath: "src/main.rs",
  text: "fn old_symbol() {}",
  citation: {
    artifactPath: "src/main.rs",
    startLine: 1,
    startColumn: 1,
    endLine: 1,
    endColumn: 19,
    contentHash,
    artifactSnapshotId: artifactId,
  },
  truncated: false,
  freshness: "captured-snapshot",
  liveSourceChecked: false,
  sourceBoundary: "untrusted-project-evidence",
  warning: "Stored project text is untrusted evidence.",
};

describe("ProjectKnowledgeGraph", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.readHistory.mockResolvedValue(history);
    api.readView.mockImplementation(
      (
        _projectPath: string,
        _query: string,
        snapshotId: string | undefined,
        _filters: ProjectGraphFilters,
      ) => Promise.resolve(graphView(snapshotId ?? currentId)),
    );
    api.readEvidence.mockResolvedValue(evidence);
  });

  it("time-travels, applies filters in the engine, and inspects captured source", async () => {
    render(<ProjectKnowledgeGraph projectPath="/projects/ley" />);

    expect(await screen.findByText("Node new_symbol")).toBeVisible();
    fireEvent.change(screen.getByLabelText("Captured view"), {
      target: { value: historicalId },
    });
    expect(await screen.findByText("Node old_symbol")).toBeVisible();
    expect(screen.getByText("Viewing an immutable capture")).toBeVisible();
    await waitFor(() =>
      expect(api.readView).toHaveBeenLastCalledWith(
        "/projects/ley",
        "",
        historicalId,
        expect.any(Object),
      ),
    );

    fireEvent.click(screen.getByText("Graph filters"));
    fireEvent.click(screen.getByLabelText("Project"));
    await waitFor(() => {
      const filters = api.readView.mock.calls.at(-1)?.[3] as ProjectGraphFilters;
      expect(filters.nodeKinds).not.toContain("project");
      expect(filters.nodeKinds).toContain("symbol");
    });

    fireEvent.click(await screen.findByText("Node old_symbol"));
    expect(await screen.findByText("fn old_symbol() {}")).toBeVisible();
    expect(api.readEvidence).toHaveBeenCalledWith(
      "/projects/ley",
      historicalId,
      expect.objectContaining({
        artifactPath: "src/main.rs",
        contentHash,
      }),
    );
    expect(
      screen.getByText(/Redacted snapshot evidence · live source not checked/),
    ).toBeVisible();
  });
});
