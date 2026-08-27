import "@xyflow/react/dist/style.css";

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Background,
  ConnectionMode,
  Controls,
  Position,
  ReactFlow,
  type Connection,
  type FitViewOptions,
  type NodeChange,
} from "@xyflow/react";
import {
  Boxes,
  ExternalLink,
  LayoutDashboard,
  Link2,
  Plus,
  Save,
  StickyNote,
  Trash2,
  X,
  Image as ImageIcon,
} from "lucide-react";
import {
  canvasContentNodeCount,
  canvasColorValue,
  createCanvas,
  deleteCanvas,
  listCanvases,
  newFileCanvasNode,
  newLinkCanvasNode,
  newTextCanvasNode,
  nextCanvasCardPosition,
  saveCanvas,
  type CanvasDocument,
  type CanvasSide,
  type CanvasSummary,
} from "@/core/vault/canvas";
import {
  groupAroundContent,
  resizeCanvasNode,
} from "@/core/vault/canvas-geometry";
import { CanvasNodeCard, type CanvasFlowNode } from "./CanvasNodeCard";
import { CanvasEdgeLayer } from "./CanvasEdgeLayer";
import { usePages } from "@/features/notes/usePages";
import type { Page } from "@/infrastructure/database/schema";
import { useNavStore } from "@/shared/state/nav";
import { nanoid } from "@/shared/lib/nanoid";
import { saveAttachment } from "@/core/vault/attachments";
import { useUIStore, type Theme } from "@/shared/state/ui";
import * as Dialog from "@radix-ui/react-dialog";

const canvasNodeTypes = { canvasCard: CanvasNodeCard } as const;
const canvasPalette = [undefined, "1", "2", "3", "4", "5", "6"] as const;

export function CanvasModal({
  open,
  initialPath,
  onClose,
}: {
  open: boolean;
  initialPath?: string | null;
  onClose: () => void;
}) {
  const queriedPages = usePages();
  const pages = useMemo(() => queriedPages ?? [], [queriedPages]);
  const [canvases, setCanvases] = useState<CanvasSummary[]>([]);
  const [activePath, setActivePath] = useState<string | null>(null);
  const [document, setDocument] = useState<CanvasDocument>({
    nodes: [],
    edges: [],
  });
  const [newName, setNewName] = useState("");
  const [selectedPage, setSelectedPage] = useState("");
  const [linkUrl, setLinkUrl] = useState("");
  const [selectedNodeIds, setSelectedNodeIds] = useState<string[]>([]);
  const [selectedEdgeId, setSelectedEdgeId] = useState<string | null>(null);
  const [pendingConnection, setPendingConnection] = useState<{
    nodeId: string;
    side: CanvasSide;
  } | null>(null);
  const [dirty, setDirty] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [deleteArmed, setDeleteArmed] = useState(false);
  const theme = useUIStore((state) => state.theme);

  useEffect(() => {
    if (!open) return;
    let active = true;
    void listCanvases().then((items) => {
      if (!active) return;
      setCanvases(items);
      const selected =
        items.find((canvas) => canvas.path === initialPath) ?? items[0];
      setActivePath(selected?.path ?? null);
      setDocument(selected?.document ?? { nodes: [], edges: [] });
      setSelectedNodeIds([]);
      setSelectedEdgeId(null);
      setPendingConnection(null);
      setDirty(false);
    });
    return () => {
      active = false;
    };
  }, [initialPath, open]);

  useEffect(() => {
    if (!open) return;
    const refreshExternalCanvas = (event: Event) => {
      const paths =
        (event as CustomEvent<{ paths?: string[] }>).detail?.paths ?? [];
      if (!paths.some((path) => path.toLowerCase().endsWith(".canvas"))) return;
      void listCanvases().then((items) => {
        setCanvases(items);
        if (dirty) return;
        const current =
          items.find((canvas) => canvas.path === activePath) ??
          items[0] ??
          null;
        setActivePath(current?.path ?? null);
        setDocument(current?.document ?? { nodes: [], edges: [] });
      });
    };
    window.addEventListener("ley:vault-files-changed", refreshExternalCanvas);
    return () =>
      window.removeEventListener(
        "ley:vault-files-changed",
        refreshExternalCanvas,
      );
  }, [activePath, dirty, open]);

  const activeCanvas =
    canvases.find((canvas) => canvas.path === activePath) ?? null;
  const pageByPath = useMemo(
    () => new Map(pages.map((page) => [page.path, page])),
    [pages],
  );
  const openFile = useCallback(
    async (path: string) => {
      const page = pageByPath.get(path);
      if (!page) return;
      if (dirty && activePath) await saveCanvas(activePath, document);
      const nav = useNavStore.getState();
      nav.openPage(page.id);
      nav.pushRecent(page.id);
      onClose();
    },
    [activePath, dirty, document, onClose, pageByPath],
  );
  const updateText = useCallback((id: string, text: string) => {
    setDocument((current) => ({
      ...current,
      nodes: current.nodes.map((node) =>
        node.id === id && node.type === "text" ? { ...node, text } : node,
      ),
    }));
    setDirty(true);
  }, []);
  const updateGroupLabel = useCallback((id: string, label: string) => {
    setDocument((current) => ({
      ...current,
      nodes: current.nodes.map((node) =>
        node.id === id && node.type === "group" ? { ...node, label } : node,
      ),
    }));
    setDirty(true);
  }, []);
  const connectByClick = useCallback(
    (nodeId: string, side: CanvasSide) => {
      if (!pendingConnection) {
        setPendingConnection({ nodeId, side });
        setStatus("Choose a handle on another card");
        return;
      }
      if (pendingConnection.nodeId === nodeId) {
        setPendingConnection(null);
        setStatus(null);
        return;
      }
      setDocument((current) => ({
        ...current,
        edges: [
          ...current.edges,
          {
            id: nanoid(),
            fromNode: pendingConnection.nodeId,
            fromSide: pendingConnection.side,
            toNode: nodeId,
            toSide: side,
            toEnd: "arrow",
          },
        ],
      }));
      setPendingConnection(null);
      setDirty(true);
      setStatus(null);
    },
    [pendingConnection],
  );
  const flowNodes = useMemo<CanvasFlowNode[]>(
    () =>
      document.nodes.map((node) => ({
        id: node.id,
        type: "canvasCard",
        position: { x: node.x, y: node.y },
        width: node.width,
        height: node.height,
        handles: canvasFlowHandles(node.width, node.height),
        style: { width: node.width, height: node.height },
        zIndex: node.type === "group" ? 0 : 1,
        selected: selectedNodeIds.includes(node.id),
        ariaLabel:
          node.type === "group"
            ? `Canvas group: ${node.label ?? "Untitled"}`
            : `Canvas ${node.type} card`,
        data: {
          node,
          fileTitle:
            node.type === "file" ? pageByPath.get(node.file)?.title : undefined,
          onOpenFile: (path) => {
            void openFile(path);
          },
          onUpdateText: updateText,
          onUpdateGroupLabel: updateGroupLabel,
          onConnectClick: connectByClick,
          pendingConnection: pendingConnection ?? undefined,
        },
      })),
    [
      connectByClick,
      document.nodes,
      openFile,
      pageByPath,
      pendingConnection,
      selectedNodeIds,
      updateGroupLabel,
      updateText,
    ],
  );
  const selectedNode =
    selectedNodeIds.length === 1
      ? (document.nodes.find((node) => node.id === selectedNodeIds[0]) ?? null)
      : null;
  const selectedEdge = selectedEdgeId
    ? (document.edges.find((edge) => edge.id === selectedEdgeId) ?? null)
    : null;
  const fitViewOptions = useMemo<FitViewOptions<CanvasFlowNode>>(
    () => ({ padding: 0.16, minZoom: 0.15, maxZoom: 2.5, nodes: flowNodes }),
    [flowNodes],
  );

  if (!open) return null;

  async function chooseCanvas(canvas: CanvasSummary) {
    if (dirty && activePath) {
      await saveCanvas(activePath, document);
      setCanvases((current) =>
        current.map((item) =>
          item.path === activePath
            ? { ...item, document, updatedAt: Date.now() }
            : item,
        ),
      );
    }
    setActivePath(canvas.path);
    setDocument(canvas.document);
    setSelectedNodeIds([]);
    setSelectedEdgeId(null);
    setPendingConnection(null);
    setDirty(false);
    setStatus(null);
    setDeleteArmed(false);
  }

  async function addCanvas() {
    if (!newName.trim()) return;
    const created = await createCanvas(newName);
    const next = [
      ...canvases.filter((canvas) => canvas.path !== created.path),
      created,
    ].sort((left, right) => left.name.localeCompare(right.name));
    setCanvases(next);
    await chooseCanvas(created);
    setNewName("");
  }

  function onNodesChange(changes: NodeChange[]) {
    setDocument((current) => {
      let nodes = current.nodes;
      let edges = current.edges;
      for (const change of changes) {
        if (change.type === "position" && change.position)
          nodes = nodes.map((node) =>
            node.id === change.id
              ? { ...node, x: change.position!.x, y: change.position!.y }
              : node,
          );
        if (change.type === "dimensions" && change.dimensions)
          nodes = nodes.map((node) =>
            node.id === change.id
              ? resizeCanvasNode(
                  node,
                  change.dimensions!.width,
                  change.dimensions!.height,
                )
              : node,
          );
        if (change.type === "remove") {
          nodes = nodes.filter((node) => node.id !== change.id);
          edges = edges.filter(
            (edge) => edge.fromNode !== change.id && edge.toNode !== change.id,
          );
        }
      }
      return { nodes, edges };
    });
    for (const change of changes) {
      if (change.type === "select") {
        setSelectedNodeIds((current) =>
          change.selected
            ? [...new Set([...current, change.id])]
            : current.filter((id) => id !== change.id),
        );
        if (change.selected) setSelectedEdgeId(null);
      }
      if (change.type === "remove")
        setSelectedNodeIds((current) =>
          current.filter((id) => id !== change.id),
        );
    }
    if (
      changes.some(
        (change) =>
          change.type === "position" ||
          change.type === "dimensions" ||
          change.type === "remove",
      )
    )
      setDirty(true);
  }

  function connect(connection: Connection) {
    if (
      !connection.source ||
      !connection.target ||
      connection.source === connection.target
    )
      return;
    setDocument((current) => ({
      ...current,
      edges: [
        ...current.edges,
        {
          id: nanoid(),
          fromNode: connection.source!,
          toNode: connection.target!,
          fromSide: canvasSide(connection.sourceHandle),
          toSide: canvasSide(connection.targetHandle),
          toEnd: "arrow",
        },
      ],
    }));
    setPendingConnection(null);
    setStatus(null);
    setDirty(true);
  }

  function addText() {
    setDocument((current) => ({
      ...current,
      nodes: [
        ...current.nodes,
        newTextCanvasNode(
          nextCanvasCardPosition(canvasContentNodeCount(current.nodes)),
        ),
      ],
    }));
    setDirty(true);
  }

  function addGroup() {
    setDocument((current) => ({
      ...current,
      nodes: [groupAroundContent(current.nodes), ...current.nodes],
    }));
    setDirty(true);
  }

  function addLink() {
    const value = linkUrl.trim();
    if (!isHttpUrl(value)) {
      setStatus("Use a complete http:// or https:// URL");
      return;
    }
    setDocument((current) => ({
      ...current,
      nodes: [
        ...current.nodes,
        newLinkCanvasNode(
          value,
          nextCanvasCardPosition(canvasContentNodeCount(current.nodes)),
        ),
      ],
    }));
    setLinkUrl("");
    setStatus(null);
    setDirty(true);
  }

  function setSelectionColor(color: string | undefined) {
    const nodeIds = new Set(selectedNodeIds);
    setDocument((current) => ({
      nodes: current.nodes.map((node) =>
        nodeIds.has(node.id) ? { ...node, color } : node,
      ),
      edges: current.edges.map((edge) =>
        edge.id === selectedEdgeId ? { ...edge, color } : edge,
      ),
    }));
    setDirty(true);
  }

  async function setSelectionBackground(file: File | undefined) {
    if (!selectedNode || selectedNode.type !== "group" || !file) return;
    try {
      setStatus("Adding background…");
      const attachment = await saveAttachment(selectedNode.id, file);
      setDocument((current) => ({
        ...current,
        nodes: current.nodes.map((node) =>
          node.id === selectedNode.id && node.type === "group"
            ? { ...node, background: attachment.path, backgroundStyle: "cover" }
            : node,
        ),
      }));
      setDirty(true);
      setStatus("Background added");
      window.setTimeout(() => setStatus(null), 1500);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    }
  }

  function removeSelectionBackground() {
    if (!selectedNode || selectedNode.type !== "group") return;
    setDocument((current) => ({
      ...current,
      nodes: current.nodes.map((node) =>
        node.id === selectedNode.id && node.type === "group"
          ? { ...node, background: undefined, backgroundStyle: undefined }
          : node,
      ),
    }));
    setDirty(true);
  }

  function updateEdgeLabel(label: string) {
    if (!selectedEdgeId) return;
    setDocument((current) => ({
      ...current,
      edges: current.edges.map((edge) =>
        edge.id === selectedEdgeId ? { ...edge, label } : edge,
      ),
    }));
    setDirty(true);
  }

  function removeSelectedEdge() {
    if (!selectedEdgeId) return;
    setDocument((current) => ({
      ...current,
      edges: current.edges.filter((edge) => edge.id !== selectedEdgeId),
    }));
    setSelectedEdgeId(null);
    setDirty(true);
  }

  function addFile() {
    const page = pages.find((candidate) => candidate.id === selectedPage);
    if (!page) return;
    setDocument((current) => ({
      ...current,
      nodes: [
        ...current.nodes,
        newFileCanvasNode(
          page.path,
          nextCanvasCardPosition(canvasContentNodeCount(current.nodes)),
        ),
      ],
    }));
    setSelectedPage("");
    setDirty(true);
  }

  async function persist(): Promise<boolean> {
    if (!activePath) return true;
    setStatus("Saving…");
    try {
      await saveCanvas(activePath, document);
      setCanvases((current) =>
        current.map((canvas) =>
          canvas.path === activePath
            ? { ...canvas, document, updatedAt: Date.now() }
            : canvas,
        ),
      );
      setDirty(false);
      setStatus("Saved");
      window.setTimeout(() => setStatus(null), 1500);
      return true;
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
      return false;
    }
  }

  async function closeCanvas() {
    if (!dirty || (await persist())) onClose();
  }

  async function removeActiveCanvas() {
    if (!activePath) return;
    if (!deleteArmed) {
      setDeleteArmed(true);
      window.setTimeout(() => setDeleteArmed(false), 3500);
      return;
    }
    await deleteCanvas(activePath);
    const remaining = canvases.filter((canvas) => canvas.path !== activePath);
    const next = remaining[0] ?? null;
    setCanvases(remaining);
    setActivePath(next?.path ?? null);
    setDocument(next?.document ?? { nodes: [], edges: [] });
    setSelectedNodeIds([]);
    setSelectedEdgeId(null);
    setPendingConnection(null);
    setDirty(false);
    setDeleteArmed(false);
  }

  return (
    <CanvasModalView
      open={open}
      activeCanvas={activeCanvas}
      canvases={canvases}
      activePath={activePath}
      newName={newName}
      setNewName={setNewName}
      chooseCanvas={chooseCanvas}
      addCanvas={addCanvas}
      status={status}
      deleteArmed={deleteArmed}
      removeActiveCanvas={removeActiveCanvas}
      dirty={dirty}
      persist={persist}
      closeCanvas={closeCanvas}
      selectedPage={selectedPage}
      setSelectedPage={setSelectedPage}
      pages={pages}
      addFile={addFile}
      addText={addText}
      addGroup={addGroup}
      linkUrl={linkUrl}
      setLinkUrl={setLinkUrl}
      addLink={addLink}
      selectedNodeIds={selectedNodeIds}
      selectedNode={selectedNode}
      selectedEdge={selectedEdge}
      updateEdgeLabel={updateEdgeLabel}
      removeSelectedEdge={removeSelectedEdge}
      setSelectionColor={setSelectionColor}
      setSelectionBackground={setSelectionBackground}
      removeSelectionBackground={removeSelectionBackground}
      flowNodes={flowNodes}
      onNodesChange={onNodesChange}
      connect={connect}
      setStatus={setStatus}
      theme={theme}
      fitViewOptions={fitViewOptions}
      document={document}
      selectedEdgeId={selectedEdgeId}
      setSelectedNodeIds={setSelectedNodeIds}
      setSelectedEdgeId={setSelectedEdgeId}
    />
  );
}

function CanvasModalView({
  open,
  activeCanvas,
  canvases,
  activePath,
  newName,
  setNewName,
  chooseCanvas,
  addCanvas,
  status,
  deleteArmed,
  removeActiveCanvas,
  dirty,
  persist,
  closeCanvas,
  selectedPage,
  setSelectedPage,
  pages,
  addFile,
  addText,
  addGroup,
  linkUrl,
  setLinkUrl,
  addLink,
  selectedNodeIds,
  selectedNode,
  selectedEdge,
  updateEdgeLabel,
  removeSelectedEdge,
  setSelectionColor,
  setSelectionBackground,
  removeSelectionBackground,
  flowNodes,
  onNodesChange,
  connect,
  setStatus,
  theme,
  fitViewOptions,
  document,
  selectedEdgeId,
  setSelectedNodeIds,
  setSelectedEdgeId,
}: {
  open: boolean;
  activeCanvas: CanvasSummary | null;
  canvases: CanvasSummary[];
  activePath: string | null;
  newName: string;
  setNewName: (value: string) => void;
  chooseCanvas: (canvas: CanvasSummary) => Promise<void>;
  addCanvas: () => Promise<void>;
  status: string | null;
  deleteArmed: boolean;
  removeActiveCanvas: () => Promise<void>;
  dirty: boolean;
  persist: () => Promise<boolean>;
  closeCanvas: () => Promise<void>;
  selectedPage: string;
  setSelectedPage: (value: string) => void;
  pages: Page[];
  addFile: () => void;
  addText: () => void;
  addGroup: () => void;
  linkUrl: string;
  setLinkUrl: (value: string) => void;
  addLink: () => void;
  selectedNodeIds: string[];
  selectedNode: CanvasDocument["nodes"][number] | null;
  selectedEdge: CanvasDocument["edges"][number] | null;
  updateEdgeLabel: (label: string) => void;
  removeSelectedEdge: () => void;
  setSelectionColor: (color: string | undefined) => void;
  setSelectionBackground: (file: File | undefined) => Promise<void>;
  removeSelectionBackground: () => void;
  flowNodes: CanvasFlowNode[];
  onNodesChange: (changes: NodeChange[]) => void;
  connect: (connection: Connection) => void;
  setStatus: (value: string | null) => void;
  theme: Theme;
  fitViewOptions: FitViewOptions<CanvasFlowNode>;
  document: CanvasDocument;
  selectedEdgeId: string | null;
  setSelectedNodeIds: (value: string[]) => void;
  setSelectedEdgeId: (value: string | null) => void;
}) {
  return (
    <Dialog.Root
      open={open}
      onOpenChange={(next) => {
        if (!next) void closeCanvas();
      }}
    >
      <Dialog.Portal>
        <Dialog.Content
          aria-describedby={undefined}
          onPointerDownOutside={(event) => event.preventDefault()}
          onInteractOutside={(event) => event.preventDefault()}
          className="fixed inset-0 z-[85] flex flex-col bg-background text-foreground outline-none"
        >
          <Dialog.Title className="sr-only">Canvas</Dialog.Title>
          <CanvasModalHeader
            activeCanvas={activeCanvas}
            activePath={activePath}
            status={status}
            deleteArmed={deleteArmed}
            dirty={dirty}
            removeActiveCanvas={removeActiveCanvas}
            persist={persist}
            closeCanvas={closeCanvas}
          />
          <div className="flex min-h-0 flex-1 flex-col md:flex-row">
            <aside className="app-sidebar flex max-h-[46vh] w-full shrink-0 flex-col overflow-y-auto border-b border-border p-3 md:max-h-none md:w-72 md:border-b-0 md:border-r">
              <div className="mb-2 text-micro font-medium uppercase tracking-[0.12em] text-muted-foreground">
                Canvases
              </div>
              <div className="flex gap-1 overflow-x-auto md:block md:space-y-1">
                {canvases.map((canvas) => (
                  <button
                    key={canvas.path}
                    type="button"
                    onClick={() => void chooseCanvas(canvas)}
                    className={`min-w-32 shrink-0 rounded-sm px-2 py-1.5 text-left text-meta outline-none transition-colors focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary ${canvas.path === activePath ? "bg-surface-3 text-foreground" : "text-muted-foreground-strong hover:bg-surface-2"}`}
                  >
                    {canvas.name}
                  </button>
                ))}
              </div>
              <div className="mt-3 flex gap-1">
                <input
                  value={newName}
                  onChange={(event) => setNewName(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") void addCanvas();
                  }}
                  placeholder="New canvas"
                  className="min-w-0 flex-1 rounded-md border border-border bg-background px-2 text-meta outline-none transition-[border-color,box-shadow] focus:border-primary focus:ring-2 focus:ring-primary/35"
                />
                <button
                  type="button"
                  onClick={() => void addCanvas()}
                  className="rounded-md border border-border p-2 outline-none transition-colors hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-primary"
                  aria-label="Create canvas"
                >
                  <Plus size={13} />
                </button>
              </div>
              {activePath && (
                <div className="mt-3 space-y-2 border-t border-border pt-3 md:mt-auto">
                  <div className="grid grid-cols-2 gap-1.5">
                    <button
                      type="button"
                      onClick={addText}
                      className="flex items-center gap-2 rounded-md border border-border px-2 py-1.5 text-meta outline-none transition-colors hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-primary"
                    >
                      <StickyNote size={13} />
                      Text
                    </button>
                    <button
                      type="button"
                      onClick={addGroup}
                      className="flex items-center gap-2 rounded-md border border-border px-2 py-1.5 text-meta outline-none transition-colors hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-primary"
                    >
                      <Boxes size={13} />
                      Group
                    </button>
                  </div>
                  <div className="flex gap-1">
                    <select
                      value={selectedPage}
                      onChange={(event) => setSelectedPage(event.target.value)}
                      className="min-w-0 flex-1 rounded-md border border-border bg-background px-2 text-micro outline-none transition-[border-color,box-shadow] focus:border-primary focus:ring-2 focus:ring-primary/35"
                    >
                      <option value="">Choose note…</option>
                      {pages.map((page) => (
                        <option key={page.id} value={page.id}>
                          {page.title}
                        </option>
                      ))}
                    </select>
                    <button
                      type="button"
                      onClick={addFile}
                      disabled={!selectedPage}
                      className="rounded-md border border-border p-2 outline-none transition-colors hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-primary disabled:opacity-40"
                      aria-label="Add note card"
                    >
                      <Link2 size={13} />
                    </button>
                  </div>
                  <div className="flex gap-1">
                    <input
                      value={linkUrl}
                      onChange={(event) => setLinkUrl(event.target.value)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter") addLink();
                      }}
                      placeholder="https://example.com"
                      aria-label="Link card URL"
                      className="min-w-0 flex-1 rounded-md border border-border bg-background px-2 text-micro outline-none transition-[border-color,box-shadow] focus:border-primary focus:ring-2 focus:ring-primary/35"
                    />
                    <button
                      type="button"
                      onClick={addLink}
                      disabled={!linkUrl.trim()}
                      className="rounded-md border border-border p-2 outline-none transition-colors hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-primary disabled:opacity-40"
                      aria-label="Add link card"
                    >
                      <ExternalLink size={13} />
                    </button>
                  </div>
                  {(selectedNodeIds.length > 0 || selectedEdge) && (
                    <section
                      className="space-y-2 rounded-md border border-border bg-background/70 p-2.5"
                      aria-label="Canvas selection"
                    >
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-micro font-medium uppercase tracking-[0.1em] text-muted-foreground">
                          Selection
                        </span>
                        <span className="text-micro text-muted-foreground">
                          {selectedEdge
                            ? "Connection"
                            : selectedNodeIds.length === 1
                              ? selectedNode?.type
                              : `${selectedNodeIds.length} cards`}
                        </span>
                      </div>
                      {selectedEdge && (
                        <div className="flex gap-1">
                          <input
                            value={selectedEdge.label ?? ""}
                            onChange={(event) =>
                              updateEdgeLabel(event.target.value)
                            }
                            placeholder="Connection label"
                            aria-label="Connection label"
                            className="min-w-0 flex-1 rounded-md border border-border bg-surface-1 px-2 py-1.5 text-meta outline-none transition-[border-color,box-shadow] focus:border-primary focus:ring-2 focus:ring-primary/35"
                          />
                          <button
                            type="button"
                            onClick={removeSelectedEdge}
                            className="rounded-md border border-border p-2 text-muted-foreground outline-none transition-colors hover:bg-surface-2 hover:text-destructive focus-visible:ring-2 focus-visible:ring-destructive"
                            aria-label="Delete connection"
                          >
                            <Trash2 size={13} />
                          </button>
                        </div>
                      )}
                      <div
                        className="flex items-center gap-1"
                        aria-label="Selection color"
                      >
                        {canvasPalette.map((color) => (
                          <button
                            key={color ?? "none"}
                            type="button"
                            onClick={() => setSelectionColor(color)}
                            className="size-6 rounded-full border border-border shadow-sm transition-transform hover:scale-110 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
                            style={{
                              background:
                                canvasColorValue(color) ??
                                "hsl(var(--surface-2))",
                            }}
                            aria-label={
                              color ? `Set color ${color}` : "Remove color"
                            }
                          />
                        ))}
                      </div>
                      {selectedNode?.type === "group" && (
                        <div className="flex gap-1">
                          <label className="flex min-w-0 flex-1 cursor-pointer items-center justify-center gap-1 rounded-md border border-border px-2 py-1.5 text-meta outline-none transition-colors hover:bg-surface-2 has-focus-visible:ring-2 has-focus-visible:ring-primary">
                            <ImageIcon size={13} />
                            {selectedNode.background
                              ? "Replace image"
                              : "Set image"}
                            <input
                              type="file"
                              accept="image/png,image/jpeg,image/gif,image/webp"
                              className="sr-only"
                              onChange={(event) => {
                                void setSelectionBackground(
                                  event.target.files?.[0],
                                );
                                event.target.value = "";
                              }}
                            />
                          </label>
                          {selectedNode.background && (
                            <button
                              type="button"
                              onClick={removeSelectionBackground}
                              className="rounded-md border border-border p-2 text-muted-foreground outline-none transition-colors hover:bg-surface-2 hover:text-destructive focus-visible:ring-2 focus-visible:ring-destructive"
                              aria-label="Remove background"
                            >
                              <Trash2 size={13} />
                            </button>
                          )}
                        </div>
                      )}
                    </section>
                  )}
                  <p className="text-micro leading-relaxed text-muted-foreground">
                    Drag to arrange. Pull between handles—or click two
                    handles—to connect. Resize from a selected card’s edges.
                  </p>
                </div>
              )}
            </aside>
            <main className="min-h-[360px] min-w-0 flex-1">
              {activePath ? (
                <ReactFlow
                  className="bg-background"
                  nodes={flowNodes}
                  edges={[]}
                  nodeTypes={canvasNodeTypes}
                  onNodesChange={onNodesChange}
                  onConnect={connect}
                  onError={(_code, message) => setStatus(message)}
                  connectOnClick={false}
                  connectionMode={ConnectionMode.Loose}
                  fitView
                  fitViewOptions={fitViewOptions}
                  minZoom={0.15}
                  maxZoom={2.5}
                  colorMode={theme}
                  deleteKeyCode={["Backspace", "Delete"]}
                  proOptions={{ hideAttribution: true }}
                >
                  <CanvasEdgeLayer
                    document={document}
                    selectedEdgeId={selectedEdgeId}
                    onSelect={(id) => {
                      setSelectedNodeIds([]);
                      setSelectedEdgeId(id);
                    }}
                  />
                  <Background gap={22} size={1} color="hsl(var(--border))" />
                  <Controls fitViewOptions={fitViewOptions} />
                </ReactFlow>
              ) : (
                <div className="flex h-full items-center justify-center p-6">
                  <div className="max-w-sm rounded-sm border border-border bg-surface-1 p-7 text-center shadow-sm">
                    <LayoutDashboard
                      size={28}
                      className="mx-auto mb-3 text-primary"
                    />
                    <h2 className="text-body font-semibold text-foreground">
                      Make space for an idea
                    </h2>
                    <p className="mt-1 text-meta leading-relaxed text-muted-foreground">
                      Create a canvas, then arrange notes and freeform thoughts
                      without changing their place in your vault.
                    </p>
                  </div>
                </div>
              )}
            </main>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function CanvasModalHeader({
  activeCanvas,
  activePath,
  status,
  deleteArmed,
  dirty,
  removeActiveCanvas,
  persist,
  closeCanvas,
}: {
  activeCanvas: CanvasSummary | null;
  activePath: string | null;
  status: string | null;
  deleteArmed: boolean;
  dirty: boolean;
  removeActiveCanvas: () => Promise<void>;
  persist: () => Promise<boolean>;
  closeCanvas: () => Promise<void>;
}) {
  return (
    <header className="app-chrome flex h-12 shrink-0 items-center gap-2 px-2 sm:px-3">
      <LayoutDashboard size={16} className="text-primary" />
      <span className="font-semibold">Canvas</span>
      <span className="hidden truncate text-meta text-muted-foreground sm:inline">
        {activeCanvas?.name ?? "Choose or create a canvas"}
      </span>
      <div className="ml-auto flex items-center gap-2">
        {status && (
          <span className="text-micro text-muted-foreground" role="status">
            {status}
          </span>
        )}
        {activePath && (
          <button
            type="button"
            onClick={() => void removeActiveCanvas()}
            aria-label={deleteArmed ? "Confirm delete canvas" : "Delete canvas"}
            title={deleteArmed ? "Confirm delete canvas" : "Delete canvas"}
            className={`flex items-center gap-1 rounded-md px-2 py-1.5 text-meta outline-none transition-colors focus-visible:ring-2 focus-visible:ring-destructive ${deleteArmed ? "bg-destructive text-destructive-foreground" : "text-muted-foreground hover:bg-surface-3 hover:text-destructive"}`}
          >
            <Trash2 size={13} />
            {deleteArmed ? (
              "Move to trash?"
            ) : (
              <span className="hidden sm:inline">Delete</span>
            )}
          </button>
        )}
        <button
          type="button"
          disabled={!dirty || !activePath}
          onClick={() => void persist()}
          className="flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-meta font-medium text-primary-foreground outline-none transition-transform hover:opacity-90 active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-background disabled:opacity-40"
        >
          <Save size={13} />
          Save
        </button>
        <button
          type="button"
          onClick={() => void closeCanvas()}
          className="rounded-md p-1.5 text-muted-foreground outline-none transition-colors hover:bg-surface-3 hover:text-foreground focus-visible:ring-2 focus-visible:ring-primary"
          aria-label="Close canvas"
        >
          <X size={16} />
        </button>
      </div>
    </header>
  );
}

function canvasSide(value: string | null | undefined): CanvasSide | undefined {
  return value === "top" ||
    value === "right" ||
    value === "bottom" ||
    value === "left"
    ? value
    : undefined;
}

function isHttpUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

function canvasFlowHandles(width: number, height: number) {
  const size = 10;
  return [
    {
      id: "top",
      type: "source" as const,
      position: Position.Top,
      x: width / 2 - size / 2,
      y: -size / 2,
      width: size,
      height: size,
    },
    {
      id: "right",
      type: "source" as const,
      position: Position.Right,
      x: width - size / 2,
      y: height / 2 - size / 2,
      width: size,
      height: size,
    },
    {
      id: "bottom",
      type: "source" as const,
      position: Position.Bottom,
      x: width / 2 - size / 2,
      y: height - size / 2,
      width: size,
      height: size,
    },
    {
      id: "left",
      type: "source" as const,
      position: Position.Left,
      x: -size / 2,
      y: height / 2 - size / 2,
      width: size,
      height: size,
    },
  ];
}
