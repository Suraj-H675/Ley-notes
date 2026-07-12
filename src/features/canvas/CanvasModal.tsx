import '@xyflow/react/dist/style.css';

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Background, Controls, ReactFlow, type Connection, type EdgeChange, type NodeChange } from '@xyflow/react';
import { FilePlus2, LayoutDashboard, Link2, Plus, Save, StickyNote, Trash2, X } from 'lucide-react';
import {
  createCanvas,
  deleteCanvas,
  listCanvases,
  newFileCanvasNode,
  newTextCanvasNode,
  saveCanvas,
  type CanvasDocument,
  type CanvasSummary,
} from '@/core/vault/canvas';
import { usePages } from '@/features/notes/usePages';
import { useNavStore } from '@/shared/state/nav';
import { nanoid } from '@/shared/lib/nanoid';
import { useUIStore } from '@/shared/state/ui';

export function CanvasModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const queriedPages = usePages();
  const pages = useMemo(() => queriedPages ?? [], [queriedPages]);
  const [canvases, setCanvases] = useState<CanvasSummary[]>([]);
  const [activePath, setActivePath] = useState<string | null>(null);
  const [document, setDocument] = useState<CanvasDocument>({ nodes: [], edges: [] });
  const [newName, setNewName] = useState('');
  const [selectedPage, setSelectedPage] = useState('');
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
      const first = items[0];
      setActivePath(first?.path ?? null);
      setDocument(first?.document ?? { nodes: [], edges: [] });
      setDirty(false);
    });
    return () => { active = false; };
  }, [open]);

  const activeCanvas = canvases.find((canvas) => canvas.path === activePath) ?? null;
  const pageByPath = useMemo(() => new Map(pages.map((page) => [page.path, page])), [pages]);
  const openFile = useCallback(async (path: string) => {
    const page = pageByPath.get(path);
    if (!page) return;
    if (dirty && activePath) await saveCanvas(activePath, document);
    const nav = useNavStore.getState();
    nav.openPage(page.id);
    nav.pushRecent(page.id);
    onClose();
  }, [activePath, dirty, document, onClose, pageByPath]);
  const flowNodes = useMemo(() => document.nodes.map((node) => ({
    id: node.id,
    position: { x: node.x, y: node.y },
    style: {
      width: node.width,
      height: node.height,
      borderRadius: 12,
      border: '1px solid hsl(var(--border))',
      background: 'hsl(var(--surface-1))',
      color: 'hsl(var(--foreground))',
      padding: 0,
      overflow: 'hidden',
    },
    data: {
      label: node.type === 'text'
        ? <textarea className="nodrag nowheel h-full w-full resize-none bg-transparent p-3 text-meta leading-relaxed outline-none" value={node.text} aria-label="Canvas text card" onChange={(event) => updateText(node.id, event.target.value)} />
        : <button type="button" className="nodrag flex h-full w-full flex-col items-start justify-center gap-1 p-3 text-left hover:bg-surface-2" onClick={() => void openFile(node.file)}><span className="flex items-center gap-1.5 text-meta font-medium"><FilePlus2 size={14} className="text-secondary" />{pageByPath.get(node.file)?.title ?? node.file}</span><span className="font-mono text-micro text-muted-foreground">{node.file}</span></button>,
    },
  })), [document.nodes, openFile, pageByPath]);
  const flowEdges = useMemo(() => document.edges.map((edge) => ({ id: edge.id, source: edge.fromNode, target: edge.toNode, label: edge.label, animated: false })), [document.edges]);

  if (!open) return null;

  async function chooseCanvas(canvas: CanvasSummary) {
    if (dirty && activePath) {
      await saveCanvas(activePath, document);
      setCanvases((current) => current.map((item) => item.path === activePath ? { ...item, document, updatedAt: Date.now() } : item));
    }
    setActivePath(canvas.path);
    setDocument(canvas.document);
    setDirty(false);
    setStatus(null);
    setDeleteArmed(false);
  }

  async function addCanvas() {
    if (!newName.trim()) return;
    const created = await createCanvas(newName);
    const next = [...canvases.filter((canvas) => canvas.path !== created.path), created].sort((left, right) => left.name.localeCompare(right.name));
    setCanvases(next);
    await chooseCanvas(created);
    setNewName('');
  }

  function updateText(id: string, text: string) {
    setDocument((current) => ({ ...current, nodes: current.nodes.map((node) => node.id === id && node.type === 'text' ? { ...node, text } : node) }));
    setDirty(true);
  }

  function onNodesChange(changes: NodeChange[]) {
    setDocument((current) => {
      let nodes = current.nodes;
      let edges = current.edges;
      for (const change of changes) {
        if (change.type === 'position' && change.position) nodes = nodes.map((node) => node.id === change.id ? { ...node, x: change.position!.x, y: change.position!.y } : node);
        if (change.type === 'remove') {
          nodes = nodes.filter((node) => node.id !== change.id);
          edges = edges.filter((edge) => edge.fromNode !== change.id && edge.toNode !== change.id);
        }
      }
      return { nodes, edges };
    });
    if (changes.some((change) => change.type === 'position' || change.type === 'remove')) setDirty(true);
  }

  function connect(connection: Connection) {
    if (!connection.source || !connection.target || connection.source === connection.target) return;
    setDocument((current) => ({ ...current, edges: [...current.edges, { id: nanoid(), fromNode: connection.source!, toNode: connection.target! }] }));
    setDirty(true);
  }

  function onEdgesChange(changes: EdgeChange[]) {
    const removed = new Set(changes.filter((change) => change.type === 'remove').map((change) => change.id));
    if (removed.size === 0) return;
    setDocument((current) => ({ ...current, edges: current.edges.filter((edge) => !removed.has(edge.id)) }));
    setDirty(true);
  }

  function addText() {
    setDocument((current) => ({ ...current, nodes: [...current.nodes, newTextCanvasNode(nextCardPosition(current.nodes.length))] }));
    setDirty(true);
  }

  function addFile() {
    const page = pages.find((candidate) => candidate.id === selectedPage);
    if (!page) return;
    setDocument((current) => ({ ...current, nodes: [...current.nodes, newFileCanvasNode(page.path, nextCardPosition(current.nodes.length))] }));
    setSelectedPage('');
    setDirty(true);
  }

  async function persist(): Promise<boolean> {
    if (!activePath) return true;
    setStatus('Saving…');
    try {
      await saveCanvas(activePath, document);
      setCanvases((current) => current.map((canvas) => canvas.path === activePath ? { ...canvas, document, updatedAt: Date.now() } : canvas));
      setDirty(false);
      setStatus('Saved');
      window.setTimeout(() => setStatus(null), 1500);
      return true;
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
      return false;
    }
  }

  async function closeCanvas() {
    if (!dirty || await persist()) onClose();
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
    setDirty(false);
    setDeleteArmed(false);
  }

  return (
    <div role="dialog" aria-modal="true" aria-label="Canvas" className="fixed inset-0 z-[85] flex flex-col bg-background text-foreground">
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border bg-surface-1 px-2 sm:px-3">
        <LayoutDashboard size={16} className="text-primary" />
        <span className="font-semibold">Canvas</span>
        <span className="hidden truncate text-meta text-muted-foreground sm:inline">{activeCanvas?.name ?? 'Choose or create a canvas'}</span>
        <div className="ml-auto flex items-center gap-2">
          {status && <span className="text-micro text-muted-foreground" role="status">{status}</span>}
          {activePath && <button type="button" onClick={() => void removeActiveCanvas()} className={`flex items-center gap-1 rounded-md px-2 py-1.5 text-meta ${deleteArmed ? 'bg-destructive text-white' : 'text-muted-foreground hover:bg-surface-3 hover:text-destructive'}`}><Trash2 size={13} />{deleteArmed ? 'Move to trash?' : <span className="hidden sm:inline">Delete</span>}</button>}
          <button type="button" disabled={!dirty || !activePath} onClick={() => void persist()} className="flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-meta font-medium text-primary-foreground disabled:opacity-40"><Save size={13} />Save</button>
          <button type="button" onClick={() => void closeCanvas()} className="rounded-md p-1.5 text-muted-foreground hover:bg-surface-3 hover:text-foreground" aria-label="Close canvas"><X size={16} /></button>
        </div>
      </header>
      <div className="flex min-h-0 flex-1 flex-col md:flex-row">
        <aside className="flex max-h-64 w-full shrink-0 flex-col overflow-auto border-b border-border bg-surface-1 p-3 md:max-h-none md:w-64 md:border-b-0 md:border-r">
          <div className="mb-2 text-micro font-medium uppercase tracking-[0.12em] text-muted-foreground">Canvases</div>
          <div className="flex gap-1 overflow-x-auto md:block md:space-y-1">
            {canvases.map((canvas) => <button key={canvas.path} type="button" onClick={() => void chooseCanvas(canvas)} className={`w-full shrink-0 rounded-md px-2 py-1.5 text-left text-meta ${canvas.path === activePath ? 'bg-surface-3 text-foreground' : 'text-muted-foreground-strong hover:bg-surface-2'}`}>{canvas.name}</button>)}
          </div>
          <div className="mt-3 flex gap-1">
            <input value={newName} onChange={(event) => setNewName(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') void addCanvas(); }} placeholder="New canvas" className="min-w-0 flex-1 rounded-md border border-border bg-background px-2 text-meta outline-none focus:border-primary" />
            <button type="button" onClick={() => void addCanvas()} className="rounded-md border border-border p-2 hover:bg-surface-2" aria-label="Create canvas"><Plus size={13} /></button>
          </div>
          {activePath && <div className="mt-3 space-y-2 border-t border-border pt-3 md:mt-auto">
            <button type="button" onClick={addText} className="flex w-full items-center gap-2 rounded-md border border-border px-2 py-1.5 text-meta hover:bg-surface-2"><StickyNote size={13} />Add text card</button>
            <div className="flex gap-1">
              <select value={selectedPage} onChange={(event) => setSelectedPage(event.target.value)} className="min-w-0 flex-1 rounded-md border border-border bg-background px-2 text-micro outline-none"><option value="">Choose note…</option>{pages.map((page) => <option key={page.id} value={page.id}>{page.title}</option>)}</select>
              <button type="button" onClick={addFile} disabled={!selectedPage} className="rounded-md border border-border p-2 disabled:opacity-40" aria-label="Add note card"><Link2 size={13} /></button>
            </div>
            <p className="text-micro leading-relaxed text-muted-foreground">Drag cards, connect their handles, and press Delete to remove a selection.</p>
          </div>}
        </aside>
        <main className="min-w-0 flex-1">
          {activePath ? <ReactFlow className="bg-background" nodes={flowNodes} edges={flowEdges} onNodesChange={onNodesChange} onEdgesChange={onEdgesChange} onConnect={connect} fitView colorMode={theme} deleteKeyCode={['Backspace', 'Delete']} proOptions={{ hideAttribution: true }}><Background gap={22} size={1} color="hsl(var(--border))" /><Controls /></ReactFlow> : <div className="flex h-full items-center justify-center p-6"><div className="max-w-sm rounded-2xl border border-border bg-surface-1 p-7 text-center shadow-sm"><LayoutDashboard size={28} className="mx-auto mb-3 text-primary" /><h2 className="text-body font-semibold text-foreground">Make space for an idea</h2><p className="mt-1 text-meta leading-relaxed text-muted-foreground">Create a canvas, then arrange notes and freeform thoughts without changing their place in your vault.</p></div></div>}
        </main>
      </div>
    </div>
  );
}

function nextCardPosition(index: number): { x: number; y: number } {
  return { x: 80 + (index % 3) * 340, y: 80 + Math.floor(index / 3) * 220 };
}
