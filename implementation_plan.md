# Knowledge Universe — Final Implementation Plan v2.0

## Synthesized from Claude's Spec + GPT's Critiques + Independent Research

---

# PART A: DECISIONS LOG

Every point of disagreement between Claude and GPT, plus bugs found during research. This section explains **what we chose and why**.

---

## Bugs Fixed from Claude's Plan

| # | Bug | Fix |
|---|---|---|
| 1 | `Set<string>` in zustand persist → serializes to `{}` | Use `string[]` instead. `JSON.stringify` can't handle `Set`. |
| 2 | Missing `dexie-react-hooks` package | Added. `useLiveQuery` is in a separate package, not built into Dexie v4. |
| 3 | Missing `@tiptap/suggestion` package | Added. It's NOT part of `@tiptap/starter-kit`. |
| 4 | Missing positioning library for suggestion popups | Added `@floating-ui/dom`. Required to position WikiLink + SlashCommand dropdowns. |
| 5 | `isArchived` uses boolean but Dexie can't index booleans | Store as `0 \| 1` number. IndexedDB doesn't support boolean keys. |
| 6 | WikiLink extension missing `render` function in suggestion config | Added complete `ReactRenderer` + Floating UI render pattern. |
| 7 | SlashCommand extension file listed in tree but never implemented | Added full `Extension.create()` implementation. |
| 8 | `useNodes` hook uses manual `useState` + reload instead of reactive queries | Use Dexie's `useLiveQuery` for automatic re-rendering when DB changes. |
| 9 | `editor.store.ts` referenced in file tree but never defined | Added implementation. |
| 10 | React Flow v12 uses named imports (`{ ReactFlow }`) not default | Fixed all imports. `nodeTypes`/`edgeTypes` must be defined OUTSIDE components. |
| 11 | FlexSearch results are field-grouped, need deduplication | Added `flattenResults` helper in search module. |

---

## GPT's Critiques — Verdict

| # | GPT's Suggestion | Verdict | Reasoning |
|---|---|---|---|
| 1 | **Typed relationships** (`depends-on`, `part-of`, etc.) | ✅ **Phase 1** | Low effort — extend `EdgeType` enum + add a dropdown in Universe edge creation. Huge value for graph queries. |
| 2 | **Multiple graph viz modes** (network, dependency, timeline) | ⚠️ **Design for, build force-only** | Architecture supports pluggable layouts. Ship force-directed only in Phase 1. |
| 3 | **Node templates** (Book Note, Research Paper, Person) | ✅ **Phase 1 (simplified)** | Add `properties: Record<string, string>` to `KnowledgeNode` + a template picker in "New Page". |
| 4 | **Knowledge health metrics** (orphans, hubs, dead knowledge) | ✅ **Phase 1** | Already have graph metrics. Add a "Knowledge Health" card on the Home page. |
| 5 | **Enhanced bidirectional linking** (Links To / Referenced By) | ✅ **Phase 1** | `BacklinkPanel` exists. Enhance it to show outgoing + incoming + related. |
| 6 | **Graph query language** | ❌ **Phase 2** | Search operators already cover 80%. Full query language is scope creep. |
| 7 | **Spatial regions** (Frontend Zone, Research Zone) | ❌ **Phase 2** | Cool but significant UI/UX work. Defer. |
| 8 | **Knowledge evolution timeline** (time-travel replay) | ❌ **Phase 2** | Requires snapshot infrastructure. Defer. |
| 9 | **Embedded subgraphs** in documents | ❌ **Phase 2** | Complex TipTap integration. Defer. |
| 10 | **Universal command palette** (actions, not just search) | ✅ **Phase 1** | Extend Cmd+K with "Create Page", "Create Task", "Open Universe", etc. Low effort, high UX value. |
| 11 | **Node references** (synced blocks) | ❌ **Phase 2** | Notion-level complexity. Defer. |
| 12 | **Plugin system** | ❌ **Design folder structure only** | No runtime plugin system yet. |
| 13 | **Graph snapshots** | ❌ **Phase 2** | |
| 14 | **Multi-universe / workspaces** | ❌ **Phase 2** | |
| 15 | **Semantic layer** (typed node properties) | ✅ **Phase 1 (simplified)** | Merged with #3 — `properties` field on nodes. |

---

## My Own Additions

| Addition | Why |
|---|---|
| **Use `useLiveQuery`** from `dexie-react-hooks` | Automatic reactivity — components re-render when IndexedDB changes. No manual state sync. |
| **Use `@floating-ui/dom`** instead of tippy.js | Future-proof (TipTap v3 drops tippy.js), smaller bundle, tree-shakable. |
| **Use `string[]`** instead of `Set<string>` in stores | Zustand persist can't serialize `Set`. Avoid adding `superjson` dependency for one field. |
| **React Flow: `nodeTypes` outside component** | Defining inside causes infinite re-renders (new object every render). Critical for perf. |
| **React Flow: `onNodesChange` for position persistence** | `onNodeDrag` is for side effects only in v12. Using it for state causes positions to revert. |
| **Debounced position persistence** | Save to IndexedDB on drag-end only, not every tick. |

---

# PART B: FINAL TECH STACK

> [!IMPORTANT]
> Additions/changes from Claude's original stack are marked with 🆕.

## Core Framework
```
vite@5.x
react@18.x
react-dom@18.x
typescript@5.x
```

## Styling
```
tailwindcss@3.4.x
postcss@8.x
autoprefixer@10.x
```

## Routing
```
react-router-dom@6.x
```

## State Management
```
zustand@4.x
immer@10.x
```

## Database (Local)
```
dexie@4.x
dexie-react-hooks@1.x              🆕 — useLiveQuery for reactive DB queries
```

## Editor
```
@tiptap/react@2.x
@tiptap/starter-kit@2.x
@tiptap/suggestion@2.x             🆕 — REQUIRED, not in starter-kit
@tiptap/extension-placeholder@2.x
@tiptap/extension-character-count@2.x
@tiptap/extension-code-block-lowlight@2.x
@tiptap/extension-task-list@2.x
@tiptap/extension-task-item@2.x
@tiptap/extension-table@2.x
@tiptap/extension-table-row@2.x
@tiptap/extension-table-header@2.x
@tiptap/extension-table-cell@2.x
@tiptap/extension-image@2.x
@tiptap/extension-link@2.x
lowlight@3.x
highlight.js@11.x
@floating-ui/dom@1.x               🆕 — replaces tippy.js for suggestion popup positioning
```

## Graph / Universe
```
@xyflow/react@12.x                 — React Flow v12 (named import: { ReactFlow })
d3-force@3.x
graphology@0.25.x
graphology-communities-louvain@2.x
graphology-layout-force@0.7.x
```

## Search
```
flexsearch@0.7.x
```

## UI / Animation / Icons
```
framer-motion@11.x
lucide-react@0.400.x
@radix-ui/react-dialog@1.x
@radix-ui/react-dropdown-menu@2.x
@radix-ui/react-tooltip@1.x
@radix-ui/react-popover@1.x
@radix-ui/react-context-menu@2.x
@radix-ui/react-scroll-area@1.x
@radix-ui/react-separator@1.x
@radix-ui/react-slot@1.x
clsx@2.x
tailwind-merge@2.x
```

## Utilities
```
uuid@9.x
date-fns@3.x
nanoid@5.x
```

## Dev Dependencies
```
@types/react@18.x
@types/react-dom@18.x
@types/uuid@9.x
@vitejs/plugin-react@4.x
eslint@8.x
prettier@3.x
```

### Removed from Claude's Stack
- ❌ `tippy.js` — replaced by `@floating-ui/dom`

---

# PART C: ENHANCED TYPE SYSTEM

Changes from Claude's original, incorporating GPT's feedback and bug fixes.

## node.types.ts — Changes

```diff
+ // Node templates for GPT's suggestion #3
+ export type NodeTemplate = 'blank' | 'book-note' | 'research-paper' | 'meeting-note' | 'person' | 'concept';

  export interface KnowledgeNode {
    id: string;
    type: NodeType;
    title: string;
    emoji?: string;
    content: TiptapContent | null;
    plainText: string;
    collections: string[];
    tags: string[];
+   properties: Record<string, string>;    // 🆕 GPT #15: typed key-value properties
+   template?: NodeTemplate;               // 🆕 GPT #3: which template created this
    taskStatus?: TaskStatus;
    taskDueDate?: number;
-   isArchived: boolean;
+   isArchived: number;                    // 🆕 FIX: 0 | 1 — IndexedDB can't index booleans
    createdAt: number;
    updatedAt: number;
    parentId?: string;
  }
```

## edge.types.ts — Changes

```diff
  export type EdgeType =
    | 'wiki-link'
    | 'explicit'
    | 'task-dependency'
-   | 'project-member';
+   | 'project-member'
+   | 'depends-on'         // 🆕 GPT #1: typed relationships
+   | 'part-of'
+   | 'related-to'
+   | 'contradicts'
+   | 'extends'
+   | 'uses'
+   | 'created-by';

  export interface KnowledgeEdge {
    id: string;
    source: string;
    target: string;
    type: EdgeType;
    label?: string;
+   strength?: number;      // 🆕 optional 0-1 weight for layout influence
    createdAt: number;
  }
```

## search.types.ts — Changes (Command Palette)

```diff
+ // 🆕 GPT #10: Universal command palette actions
+ export interface CommandAction {
+   id: string;
+   label: string;
+   icon?: string;
+   category: 'navigation' | 'create' | 'action';
+   keywords: string[];          // extra search terms
+   execute: () => void;
+ }
```

All other types remain exactly as Claude specified.

---

# PART D: CORRECTED IMPLEMENTATION PATTERNS

These replace the buggy patterns from Claude's spec.

## D.1 — Database: Boolean Indexing Fix

```typescript
// When creating a node, use 0 instead of false:
const node: KnowledgeNode = {
  // ...
  isArchived: 0,  // NOT false
};

// When querying:
await db.nodes.where('isArchived').equals(0).toArray();  // ✅ works
// NOT: .equals(false)  ❌ broken
```

## D.2 — Reactive Hooks with useLiveQuery

**Replace Claude's manual `useState` + `loadNodes()` pattern:**

```typescript
// src/hooks/useNodes.ts — CORRECTED
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '../lib/db';

export function useNodes() {
  // Auto-updates when DB changes — no manual reload needed
  const nodes = useLiveQuery(
    () => db.nodes.where('isArchived').equals(0).toArray(),
    [],    // no deps
    []     // default value (avoids undefined)
  );

  // ... createNode, updateNode, deleteNode remain as CRUD functions
  // that write to DB — useLiveQuery handles the re-render automatically
}
```

## D.3 — Zustand Store: No Set, Use Array

```typescript
// workspace.store.ts — CORRECTED
interface WorkspaceState {
  expandedCollections: string[];  // NOT Set<string>
  // ...
  toggleCollection: (id: string) => void;
}

// In the store:
toggleCollection: (id) => set(state => {
  if (state.expandedCollections.includes(id)) {
    state.expandedCollections = state.expandedCollections.filter(c => c !== id);
  } else {
    state.expandedCollections = [...state.expandedCollections, id];
  }
}),
```

## D.4 — TipTap Suggestion Render Function (Floating UI)

**This is the critical missing piece from Claude's plan:**

```typescript
// src/components/editor/suggestion-renderer.ts
import { ReactRenderer } from '@tiptap/react';
import { computePosition, flip, shift, offset } from '@floating-ui/dom';

export function createSuggestionRenderer(SuggestionComponent: React.ComponentType) {
  return () => {
    let component: ReactRenderer | null = null;
    let floatingEl: HTMLDivElement | null = null;

    const updatePosition = (clientRect: () => DOMRect) => {
      if (!floatingEl || !clientRect) return;
      computePosition(
        { getBoundingClientRect: clientRect } as Element,
        floatingEl,
        { placement: 'bottom-start', middleware: [offset(8), flip(), shift({ padding: 8 })] }
      ).then(({ x, y }) => {
        if (floatingEl) {
          Object.assign(floatingEl.style, { left: `${x}px`, top: `${y}px` });
        }
      });
    };

    return {
      onStart: (props: any) => {
        component = new ReactRenderer(SuggestionComponent, { props, editor: props.editor });
        floatingEl = document.createElement('div');
        floatingEl.style.position = 'fixed';
        floatingEl.style.zIndex = '9999';
        floatingEl.appendChild(component.element);
        document.body.appendChild(floatingEl);
        if (props.clientRect) updatePosition(props.clientRect);
      },
      onUpdate: (props: any) => {
        component?.updateProps(props);
        if (props.clientRect) updatePosition(props.clientRect);
      },
      onKeyDown: (props: any) => {
        if (props.event.key === 'Escape') { floatingEl?.remove(); return true; }
        return component?.ref?.onKeyDown(props) ?? false;
      },
      onExit: () => { floatingEl?.remove(); component?.destroy(); },
    };
  };
}
```

**Wire into extensions:**
```typescript
// WikiLink extension suggestion config:
suggestion: {
  char: '[[',
  items: async ({ query }) => { /* search nodes by title */ },
  render: createSuggestionRenderer(WikiLinkSuggestionList),
  command: ({ editor, range, props }) => {
    editor.chain().focus().insertContentAt(range, [
      { type: 'wikiLink', attrs: { id: props.id, label: props.label } },
      { type: 'text', text: ' ' },
    ]).run();
  },
}
```

## D.5 — SlashCommand Extension (Missing from Claude's plan)

```typescript
// src/components/editor/extensions/SlashCommand.extension.ts
import { Extension } from '@tiptap/core';
import { PluginKey } from '@tiptap/pm/state';
import Suggestion from '@tiptap/suggestion';

export const SlashCommandExtension = Extension.create({
  name: 'slashCommand',

  addOptions() {
    return {
      suggestion: {
        char: '/',
        pluginKey: new PluginKey('slashCommand'),
        startOfLine: false,
        items: ({ query }: { query: string }) => {
          return SLASH_COMMANDS.filter(cmd =>
            cmd.title.toLowerCase().includes(query.toLowerCase())
          );
        },
        command: ({ editor, range, props }: any) => {
          editor.chain().focus().deleteRange(range).run();
          props.command({ editor, range });
        },
      },
    };
  },

  addProseMirrorPlugins() {
    return [
      Suggestion({ editor: this.editor, ...this.options.suggestion }),
    ];
  },
});
```

## D.6 — React Flow v12 Integration Pattern

```typescript
// OUTSIDE component — prevents infinite re-renders
const nodeTypes = { universe: UniverseNodeComponent };
const edgeTypes = { universe: UniverseEdgeComponent };

function UniverseView() {
  // Use React Flow's built-in state management
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  // Persist positions on drag-end only
  const handleNodesChange = useCallback((changes: NodeChange[]) => {
    onNodesChange(changes);
    const dragEndChanges = changes.filter(
      c => c.type === 'position' && c.dragging === false
    );
    if (dragEndChanges.length > 0) {
      // debounced save to IndexedDB
    }
  }, [onNodesChange]);

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      onNodesChange={handleNodesChange}
      onEdgesChange={onEdgesChange}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      colorMode="dark"
      fitView
    >
      <MiniMap pannable zoomable nodeColor={getNodeColor} />
      <Controls />
      <Background variant="dots" gap={16} size={1} />
    </ReactFlow>
  );
}
```

## D.7 — FlexSearch Result Deduplication

```typescript
// src/lib/search/index.ts — helper
function flattenSearchResults(
  rawResults: Array<{ field: string; result: Array<{ id: string; doc: Record<string, unknown> }> }>
): Array<{ id: string; doc: Record<string, unknown> }> {
  const seen = new Set<string>();
  const docs: Array<{ id: string; doc: Record<string, unknown> }> = [];
  for (const group of rawResults) {
    for (const item of group.result) {
      if (!seen.has(item.id)) {
        seen.add(item.id);
        docs.push(item);
      }
    }
  }
  return docs;
}
```

---

# PART E: ENHANCED FILE STRUCTURE

Changes from Claude's original (additions marked 🆕):

```
knowledge-universe/
├── src/
│   ├── types/
│   │   ├── ... (same as Claude)
│   │   └── command.types.ts          🆕 CommandAction type for palette
│   │
│   ├── lib/
│   │   ├── db/
│   │   │   ├── ... (same as Claude)
│   │   │   └── templates.ts          🆕 node template definitions
│   │   ├── editor/
│   │   │   ├── extractText.ts
│   │   │   └── parseWikiLinks.ts
│   │   └── ... (graph, search, utils same as Claude)
│   │
│   ├── store/
│   │   ├── workspace.store.ts        (FIXED: string[] not Set)
│   │   ├── editor.store.ts           🆕 (was missing implementation)
│   │   ├── universe.store.ts
│   │   ├── search.store.ts
│   │   └── index.ts
│   │
│   ├── hooks/
│   │   ├── useNodes.ts               (FIXED: useLiveQuery)
│   │   ├── useEdges.ts               (FIXED: useLiveQuery)
│   │   ├── useCollections.ts         (FIXED: useLiveQuery)
│   │   ├── ... (rest same as Claude)
│   │   └── useCommands.ts            🆕 command palette actions
│   │
│   ├── components/
│   │   ├── editor/
│   │   │   ├── BlockEditor.tsx        (FIXED: proper suggestion setup)
│   │   │   ├── EditorToolbar.tsx
│   │   │   ├── SlashCommandMenu.tsx
│   │   │   ├── WikiLinkSuggestion.tsx
│   │   │   ├── BacklinkPanel.tsx      (ENHANCED: outgoing + incoming + related)
│   │   │   ├── suggestion-renderer.ts 🆕 Floating UI render function
│   │   │   └── extensions/
│   │   │       ├── WikiLink.extension.ts   (FIXED: proper suggestion config)
│   │   │       └── SlashCommand.extension.ts 🆕 (was missing)
│   │   │
│   │   ├── universe/
│   │   │   ├── ... (same as Claude)
│   │   │   └── EdgeTypeSelector.tsx   🆕 dropdown for relationship type
│   │   │
│   │   ├── search/
│   │   │   ├── CommandPalette.tsx     🆕 (replaces SearchModal — search + actions)
│   │   │   ├── SearchInput.tsx
│   │   │   ├── SearchResult.tsx
│   │   │   ├── SearchResultGroup.tsx
│   │   │   ├── CommandGroup.tsx       🆕 action commands section
│   │   │   └── RecentSearches.tsx
│   │   │
│   │   ├── home/
│   │   │   └── KnowledgeHealth.tsx    🆕 GPT #4: health metrics card
│   │   │
│   │   ├── document/
│   │   │   ├── ... (same as Claude)
│   │   │   └── TemplatePicker.tsx     🆕 GPT #3: template selection
│   │   │
│   │   └── ... (rest same as Claude)
│   │
│   └── pages/
│       └── ... (same as Claude)
```

---

# PART F: GPT FEATURES — IMPLEMENTATION DETAILS

## F.1 — Typed Relationships (GPT #1)

**How it works:** When a user creates an edge in Universe View (by dragging between nodes), show an `EdgeTypeSelector` dropdown. Default to `'related-to'`. The edge type renders as a colored label on the edge.

**Search integration:** The existing `related:React` operator becomes filterable:
```
depends:React          → edges where type === 'depends-on' AND target title matches
uses:JavaScript        → edges where type === 'uses' AND target title matches
```

Add these to the search operator parser.

## F.2 — Node Templates (GPT #3)

```typescript
// src/lib/db/templates.ts
export const NODE_TEMPLATES: Record<NodeTemplate, { properties: Record<string, string>; emoji: string }> = {
  'blank':          { properties: {}, emoji: '📄' },
  'book-note':      { properties: { Author: '', Pages: '', Rating: '', Summary: '' }, emoji: '📚' },
  'research-paper': { properties: { Authors: '', DOI: '', Year: '', 'Key Findings': '' }, emoji: '🔬' },
  'meeting-note':   { properties: { Date: '', Attendees: '', 'Action Items': '' }, emoji: '📋' },
  'person':         { properties: { Role: '', Organization: '', Email: '' }, emoji: '👤' },
  'concept':        { properties: { Domain: '', Definition: '', 'Related Concepts': '' }, emoji: '💡' },
};
```

Properties render as a key-value table below the document title, above the editor. Editable inline.

## F.3 — Knowledge Health Metrics (GPT #4)

Renders on the **Home Page** as a card. Uses existing graph metrics:

```typescript
// Computed from useGraph hook:
const healthMetrics = {
  totalNodes: nodes.length,
  totalEdges: edges.length,
  orphanNodes: nodes.filter(n => degrees.get(n.id) === 0),
  hubNodes: nodes.filter(n => (degrees.get(n.id) ?? 0) > 5).sort(byDegree),
  deadKnowledge: nodes.filter(n => n.updatedAt < thirtyDaysAgo),
  fastestGrowing: /* cluster with most new nodes in last 7 days */,
  avgConnections: totalEdges / totalNodes,
};
```

## F.4 — Enhanced Bidirectional Linking (GPT #5)

The `BacklinkPanel` shows three sections:
1. **Links To** — outgoing edges from this node (what this page references)
2. **Referenced By** — incoming edges to this node (what links here)
3. **Related** — 2nd-degree connections (neighbors of neighbors, minus direct connections)

## F.5 — Universal Command Palette (GPT #10)

Extend Cmd+K modal to show **two sections**:
1. **Actions** (always visible at top):
   - Create Page, Create Task, Create Project
   - Open Universe, Open Tasks, Open Settings
   - Toggle Sidebar, Toggle Dark Mode
2. **Search Results** (appears when typing):
   - Same as Claude's search implementation

```typescript
// src/hooks/useCommands.ts
export function useCommands(): CommandAction[] {
  const navigate = useNavigate();
  const { createNode } = useNodes();

  return useMemo(() => [
    { id: 'new-page', label: 'New Page', category: 'create', icon: 'FilePlus',
      keywords: ['create', 'add', 'page', 'document'],
      execute: async () => { const n = await createNode({...}); navigate(`/page/${n.id}`); }
    },
    { id: 'new-task', label: 'New Task', category: 'create', icon: 'CheckSquare',
      keywords: ['create', 'add', 'task', 'todo'],
      execute: async () => { const n = await createNode({type: 'task',...}); navigate(`/page/${n.id}`); }
    },
    { id: 'open-universe', label: 'Open Universe', category: 'navigation', icon: 'Globe',
      keywords: ['graph', 'map', 'explore'],
      execute: () => navigate('/universe')
    },
    // ... more commands
  ], [navigate, createNode]);
}
```

---

# PART G: BUILD PHASES

## Phase 1: Project Scaffolding *(~15 min)*

- Create Vite project with `react-ts` template
- Install all dependencies (corrected list from Part B)
- Configure Tailwind, PostCSS, Google Fonts
- Create `src/styles/index.css` + `src/styles/tiptap.css`
- Create `public/favicon.svg`
- **Verify:** `npm run dev` → dark blank page with correct fonts

## Phase 2: Types & Database *(~30 min)*

- Create all type files (enhanced versions from Part C)
- Create Dexie DB class with corrected schema (`isArchived` as number)
- Create all CRUD modules (nodes, edges, collections, universe, revisions)
- Create template definitions
- **Verify:** `npm run dev` → no TS errors, DB initializes in DevTools

## Phase 3: Utilities, Graph & Search *(~30 min)*

- `cn.ts`, `id.ts`, `date.ts`, `color.ts` (same as Claude)
- `extractText.ts`, `parseWikiLinks.ts` (same as Claude)
- Graph algorithms: `layout.ts`, `louvain.ts`, `pathfinding.ts`, `metrics.ts` (same as Claude)
- Search engine with corrected FlexSearch dedup (Part D.7)
- Search operator parser (enhanced with typed relationship operators)
- **Verify:** TS compiles clean

## Phase 4: Stores & Hooks *(~30 min)*

- Zustand stores (corrected — `string[]` not `Set`, added `editor.store.ts`)
- Hooks with `useLiveQuery` (corrected — Part D.2)
- `useCommands` hook (new — Part F.5)
- `useAutoSave`, `useKeyboard`, `useGraph` (from Claude, with fixes)
- **Verify:** Stores initialize, hooks return defaults

## Phase 5: UI Primitives & Layout Shell *(~45 min)*

- All `components/ui/` components (Button, Input, Modal, Tooltip, Badge, etc.)
- Layout shell: AppShell, Sidebar, SidebarHeader, SidebarNav, SidebarCollections, SidebarPageItem, SidebarFooter, ResizeHandle
- React Router setup in App.tsx
- Entry point in main.tsx with DB init
- **Verify:** App renders with working sidebar navigation

## Phase 6: Document View & Editor *(~60 min)*

- TipTap extensions: WikiLink (FIXED), SlashCommand (NEW)
- Suggestion renderer with Floating UI (NEW — Part D.4)
- BlockEditor with all extensions properly wired
- SlashCommandMenu, WikiLinkSuggestion components
- EditorToolbar (floating bubble menu)
- BacklinkPanel (enhanced — Part F.4)
- DocumentPage components (header, meta, actions)
- TemplatePicker component (new — Part F.2)
- HomePage with KnowledgeHealth card
- **Verify:** Create page → type → slash commands → wiki links → auto-save → reload → persists

## Phase 7: Universe View *(~60 min)*

- UniverseView with React Flow v12 (corrected imports — Part D.6)
- Custom node component (memo'd, cluster-colored, sized by degree)
- Custom edge component (typed, labeled, animated)
- EdgeTypeSelector dropdown (new — Part F.1)
- Universe toolbar, minimap, layer controls, node detail hover card
- D3-force layout integration (run-once, then persist positions)
- **Verify:** Graph renders, nodes draggable, edges show types, click navigates to page

## Phase 8: Search, Tasks, Projects, History & Polish *(~60 min)*

- CommandPalette (enhanced Cmd+K — Part F.5)
- Tasks view + filters
- Projects view + cards
- Collections view
- Revision history
- Settings page
- 404 page
- Final polish: animations, hover states, empty states
- **Verify:** Full end-to-end flow works. `npm run build` succeeds.

---

# PART H: VERIFICATION PLAN

### After Each Phase
```bash
npm run dev     # Must compile with no errors
```

### After Phase 6 (Editor)
- Create 3 pages with different templates
- Add wiki-links between them using `[[`
- Verify auto-save persists on reload
- Check IndexedDB in DevTools for correct data

### After Phase 7 (Universe)
- Navigate to Universe → all 3 pages visible as nodes
- Wiki-link edges visible with correct types
- Drag nodes → positions persist on reload
- Community detection colors clusters
- Click node → navigates to document

### After Phase 8 (Final)
- Cmd+K → search by title → navigate
- Cmd+K → "Create Page" action → creates and opens
- Search with operators: `is:task`, `related:PageName`
- Knowledge health card shows correct orphan/hub counts
- Tasks view shows only task-type nodes with status toggles
- Revision history shows snapshots
- `npm run build` → production bundle succeeds

### Browser Compatibility
- Chrome/Edge (primary — IndexedDB)
- Firefox (secondary)

---

# PART I: PHASE 2 ROADMAP (Not Building Now)

Features deferred from GPT's suggestions, to be built later:

| Feature | Complexity | Prerequisite |
|---|---|---|
| Graph query language | High | Search engine v2 |
| Spatial regions (zones) | Medium | Universe View v2 |
| Knowledge evolution timeline | High | Snapshot infrastructure |
| Embedded subgraphs in documents | High | Custom TipTap node |
| Node references (synced blocks) | Very High | TipTap plugin system |
| Plugin system runtime | Very High | Stable core API |
| Graph snapshots / diff | Medium | Bulk DB export |
| Multi-universe / workspaces | High | DB partitioning |
| Additional layout modes (hierarchical, circular, timeline) | Medium | Layout engine abstraction |
