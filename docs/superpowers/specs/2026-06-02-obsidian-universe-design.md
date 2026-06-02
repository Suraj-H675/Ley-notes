# Obsidian-Style Universe View

**Date:** 2026-06-02
**Status:** Approved
**Supersedes:** Phase 7 of `implementation_plan.md` (additive — does not contradict)

---

## Goal

Replace the placeholder Universe View with a full-featured graph view that matches Obsidian's look, feel, and feature set — without violating the local-first / React Flow / Graphology / Dexie architecture constraints.

After this ships:

- Global graph and local graph (1- or 2-hop neighborhood) both work.
- Right-side collapsible settings panel with Groups, Filters, Display, Physics.
- Continuous force simulation; physics sliders move the graph live.
- Hover highlights node + 1-hop neighbors; everything else fades.
- All settings persist across reloads via Dexie.
- Color schemes: untyped, tag, collection, link-count, community.

## Non-Goals

- Plugin system for graph views.
- Time-travel / graph snapshots.
- 3D graph.
- Spatial regions / zones.
- Anything that requires a backend.

---

## Architecture & Data Flow

Continuous `d3-force` simulation drives node positions. React Flow owns pan/zoom/select/interactions. A new `graphSettings` Dexie table persists all user-tunable graph state via `useLiveQuery` for reactivity.

```
                ┌────────────────────┐
                │  IndexedDB         │
                │  graphSettings     │  primary key: scope ('global' | 'local')
                └─────────┬──────────┘
                          │ useLiveQuery
                ┌─────────▼──────────┐
                │ useGraphSettings   │
                └─────────┬──────────┘
                          │
   ┌──────────────────────┼──────────────────────┐
   │                      │                      │
┌──▼──────────┐   ┌───────▼───────┐   ┌─────────▼─────────┐
│ useGraph    │   │ useFiltered-  │   │ useColoredGraph   │
│ (Graphology)│   │   Graph       │   │  (scheme picker)  │
└──────┬──────┘   └───────┬───────┘   └─────────┬─────────┘
       │                  │                     │
       └──────────────────┼─────────────────────┘
                          │
                ┌─────────▼──────────┐
                │ useGraphSimulation │  RAF tick loop
                │  throttled flush   │  reads physics cfg
                └─────────┬──────────┘
                          │ live positions
                ┌─────────▼──────────┐
                │ <UniverseView>     │
                │  ├─ <ReactFlow>    │
                │  ├─ <UniverseNode> │  hover-aware
                │  ├─ <UniverseEdge> │  color/thickness
                │  └─ <ColorLegend>  │
                └─────────┬──────────┘
                          │
                ┌─────────▼──────────┐
                │ <GraphSettings-    │  Groups / Filters /
                │   Panel>           │  Display / Physics
                │   writes Dexie     │
                └────────────────────┘

        ┌──────────────────────────────────┐
        │ <LocalGraphView>                 │  DocumentPage right
        │  reuses UniverseView logic       │  sidebar. N-hop
        │  settings: scope='local'         │  neighborhood of
        └──────────────────────────────────┘  active node
```

---

## Component Inventory

### New files

```
src/types/graph-settings.types.ts
src/lib/db/graphSettings.ts                    # CRUD for graphSettings table
src/lib/db/defaultGraphSettings.ts            # factory for default rows
src/lib/graph/colors.ts                        # color schemes
src/lib/graph/tagColor.ts                      # deterministic HSL from tag/collection name
src/lib/graph/simulation.ts                    # d3-force factory + tick loop
src/lib/graph/localGraph.ts                    # N-hop subgraph utility
src/hooks/useGraphSimulation.ts
src/hooks/useFilteredGraph.ts
src/hooks/useColoredGraph.ts
src/hooks/useGraphSettings.ts
src/components/universe/UniverseView.tsx       # extracted from UniversePage
src/components/universe/LocalGraphView.tsx
src/components/universe/GraphSettingsPanel.tsx
src/components/universe/sections/GroupsSection.tsx
src/components/universe/sections/FiltersSection.tsx
src/components/universe/sections/DisplaySection.tsx
src/components/universe/sections/PhysicsSection.tsx
src/components/universe/ColorLegend.tsx
src/components/universe/UniverseNode.tsx       # rewrite (was passthrough)
src/components/universe/UniverseEdge.tsx       # rewrite (was passthrough)
src/components/universe/Slider.tsx             # shared slider primitive
src/components/universe/CollapsibleSection.tsx # shared panel section header
```

### Modified files

- `src/lib/db/index.ts` — add `graphSettings: 'scope'` table; bump version to 3.
- `src/store/universe.store.ts` — remove persisted state; keep only ephemeral (selection, hover, zoom, drag).
- `src/pages/UniversePage.tsx` — strip down to `<PageHeader/> + <UniverseView/> + <GraphSettingsPanel/>`.
- `src/pages/DocumentPage.tsx` — add toggle in toolbar that mounts `<LocalGraphView/>` in right sidebar.
- `src/hooks/useGraph.ts` — add helper to compute N-hop subgraph for local view.
- `src/index.css` or new `universe.css` — custom node/edge styles, slider styling.

### Deleted

- None. The current `UniversePage` is restructured, not removed.

---

## Visual Design

### Background

- Flat `hsl(220 14% 9%)`. No dots, no grid.
- Matches Obsidian exactly.

### Nodes

- Filled circle, no border by default.
- Size: `clamp(8, 6 + log(1+degree)*8, 32) * nodeSizeMultiplier`.
- Hover: 1.5px outline, full opacity.
- Selected: 2px outline.
- Non-hovered, non-neighbor (when something is hovered): 0.15 opacity.
- Label sits below node, `text-fade` controls non-connected label visibility (default 0.25).

### Edges

- 1.5px stroke, curved bezier (curvature 0.25).
- In untyped/tag mode, edge inherits source node color.
- In typed mode, edge uses existing palette desaturated 15%.
- Hovering an edge or its endpoints: full opacity, +1px; otherwise 0.1.

### Page header (Universe page)

- Title "Universe" + small `X pages, Y edges` subtitle.
- Right-side: small icon button (sliders icon) to toggle the right panel.
- No top toolbar (replaced by right panel).

### Color palette

| Element | Color |
|---|---|
| Background | `hsl(220 14% 9%)` |
| Panel background | `hsl(220 14% 11%)` |
| Panel border | `1px solid hsl(220 10% 18%)` |
| Node text | `hsl(220 15% 88%)` |
| Untyped default | `hsl(220 8% 55%)` |
| Tag | `hsl( (hash(tag)*360) mod 360, 50%, 65% )` — deterministic |
| Collection | same hash, hue rotated 180° from tags |
| Link-count | gradient `hsl(220 8% 50%)` → `hsl(265 50% 70%)` mapped to `degree/maxDegree` |
| Community | 8-color fixed palette (high-saturation, WCAG-AA on dark) |
| Edge typed | existing palette, desaturated 15% |

### Right panel

- Fixed 280px, slides in from right. Persisted `panelVisible` state.
- Four collapsible groups. Default order: **Groups → Filters → Display → Physics**.
- Section open/closed state persists to Dexie.

---

## Settings Data Model

```typescript
// src/types/graph-settings.types.ts

export type ColorScheme = 'untyped' | 'tag' | 'collection' | 'link-count' | 'community';
export type GraphScope = 'global' | 'local';

export interface PhysicsConfig {
  centerForce: number;     // 0..2,     default 1
  chargeForce: number;     // -200..0,  default -60
  linkForce: number;       // 0..2,     default 1
  linkDistance: number;    // 20..200,  default 80
}

export interface DisplayConfig {
  nodeSize: number;        // 0.5..2.5, default 1
  edgeThickness: number;   // 0.5..3,   default 1
  textFade: number;        // 0..1,     default 0.25
  showLabels: boolean;     // default true
}

export interface FilterConfig {
  searchQuery: string;
  selectedTags: string[];
  selectedCollections: string[];
  showOrphans: boolean;    // default true
}

export interface GraphSettings {
  scope: GraphScope;        // primary key
  colorScheme: ColorScheme;
  physics: PhysicsConfig;
  display: DisplayConfig;
  filters: FilterConfig;
  panelSectionsOpen: {
    groups: boolean;
    filters: boolean;
    display: boolean;
    physics: boolean;
  };
  panelVisible: boolean;
  localDepth: 1 | 2;        // local graph only
  updatedAt: number;
}
```

### Dexie migration (v2 → v3)

```typescript
this.version(3).stores({
  nodes: 'id, type, updatedAt, isArchived',
  edges: 'id, source, target, type',
  collections: 'id, name, parentId',
  revisions: 'id, nodeId, createdAt',
  graphSettings: 'scope',  // NEW
});
```

On first run after migration, two rows are inserted:

- `{ scope: 'global' }` with all defaults.
- `{ scope: 'local' }` with all defaults + `localDepth: 1`.

Migration is additive — no existing data is touched.

---

## Phasing

The full scope is ~3-4x what Phase 7 originally called for. Ship in 4 sub-phases so each is independently testable.

### Phase 7a — Foundation

**Goal:** Graph looks like Obsidian. Hover works. Settings table + panel shell exist (empty).

- `useGraphSettings` hook + Dexie migration to v3.
- `defaultGraphSettings()` factory.
- Continuous `useGraphSimulation` (RAF, throttled to 30fps flush to React Flow).
- Custom `UniverseNode` (hover-aware, no fade yet — just full color and outline on hover).
- Custom `UniverseEdge` (bezier, color by type).
- `UniverseView` extracted from `UniversePage` (the existing inline code becomes a reusable component).
- `GraphSettingsPanel` shell — 4 collapsible groups, all empty for now.
- Strip `UniversePage` to header + view + panel.

**Verify:** Open Universe → graph renders, looks like Obsidian. Hover shows outline. Panel opens/closes.

### Phase 7b — Panel + persistence

**Goal:** All 4 panel groups are wired. Sliders move the graph live. Color schemes work. Filters work.

- `GroupsSection` — radio for color scheme.
- `FiltersSection` — search input, tag multi-select, collection multi-select, orphan toggle.
- `DisplaySection` — sliders for node size, edge thickness, text fade; toggle for showLabels.
- `PhysicsSection` — sliders for centerForce, chargeForce, linkForce, linkDistance. Live updates the running simulation.
- `useFilteredGraph` + `useColoredGraph` hooks.
- `ColorLegend` at bottom-left of the canvas.
- `lib/graph/colors.ts` + `tagColor.ts`.

**Verify:** Drag physics sliders → graph reshapes live. Pick "by tag" → nodes re-color. Type in search → non-matching nodes fade. Reload → settings persist.

### Phase 7c — Local graph

**Goal:** DocumentPage has a local graph in the right sidebar.

- `lib/graph/localGraph.ts` — BFS/DFS to compute N-hop subgraph from active node.
- `LocalGraphView` — reuses `UniverseView` internals, smaller canvas, centered on the active node.
- Add "Show local graph" toggle to DocumentPage toolbar.
- Toggle persists in workspace store (not graphSettings).
- `localDepth` setting in the local graph settings row controls 1 vs 2 hops.

**Verify:** Open a note → right sidebar shows 1-hop graph of that note. Toggle 2-hop → neighbors-of-neighbors appear. Switch notes → local graph updates.

### Phase 7d — Polish

**Goal:** Production-quality feel.

- Drag-persistence: save node positions to Dexie on drag-end, restore on next mount.
- Smooth pan/zoom (React Flow defaults are fine; tweak if needed).
- Click-to-open (already works) + middle-click / Cmd-click to open in new tab.
- Search highlight (matched nodes pulse).
- Empty states (no nodes, no edges, no filter matches).
- Performance pass:
  - RAF throttle to 30fps for position flushes.
  - Memoize node data, edge data.
  - Edge culling beyond N visible hops.
  - Verify <2s load for 10k-node workspace and 60fps pan.

**Verify:** Load demo data, hit 10k nodes, confirm load + pan perf. Click-around for empty/edge states.

---

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Continuous simulation thrashes React Flow at 10k nodes | RAF tick runs at 60fps but position flush to React Flow is throttled to 30fps. Memoize node data so unaffected nodes don't re-render. |
| Dexie migration fails on user with existing v2 data | Migration is additive (new table). Default rows inserted on first read after migration. Wrapped in try/catch. |
| Tag-color hash produces two tags with the same color | Hash space is large (2^32). For <=100 tags, collisions are vanishingly rare. De-dup logic uses HSL hue rotation as a fallback. |
| Local graph on a 10k-node workspace is slow | BFS is O(V+E), bounded by the active node's neighborhood. UI shows "Computing…" if it takes >100ms. |
| Physics sliders feel laggy on weak machines | Sliders write to Dexie live, but the simulation re-reads on RAF tick — not on every keystroke. |

---

## Testing Strategy

- **Unit tests** for pure utilities: `tagColor.ts`, `colors.ts`, `localGraph.ts`, `defaultGraphSettings.ts`.
- **Hook tests** with `dexie-react-hooks` test wrapper: `useGraphSettings`, `useFilteredGraph`, `useColoredGraph`.
- **Component tests** for `GraphSettingsPanel` (renders, toggles, sliders work), `UniverseNode` (hover state, opacity).
- **Manual verification per phase** (per the verify steps in each phase).
- **No e2e** — this is local-first, all in-browser. Manual + component tests are the right level.

---

## Open Questions

None. All major decisions resolved during brainstorming.
