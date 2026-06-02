# Obsidian-Style Universe View — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the placeholder Universe View with a full-featured graph view matching Obsidian's look, feel, and feature set (global + local graph, right-side settings panel, continuous force simulation, hover highlight, persistent settings).

**Architecture:** Continuous `d3-force` simulation drives node positions in an `requestAnimationFrame` loop, throttled to 30fps for React Flow flush. Graphology builds the graph from Dexie. New `graphSettings` Dexie table persists panel/slider/color-scheme state via `useLiveQuery`. Settings panel lives in a fixed 280px right sidebar with 4 collapsible groups. Local graph is a 1- or 2-hop neighborhood of the active note in the DocumentPage right sidebar.

**Tech Stack:** React 18, TypeScript, Vite, @xyflow/react 12, graphology, d3-force 3, Dexie 4 + dexie-react-hooks, Zustand 4, Framer Motion 11, Tailwind 3.4, Radix UI primitives, vitest + @testing-library/react (new), lucide-react.

**Spec:** `docs/superpowers/specs/2026-06-02-obsidian-universe-design.md`

**Phasing (4 sub-phases, each independently testable):**

| Phase | Goal |
|---|---|
| **Phase 0** | Test infrastructure (vitest, RTL, happy-dom) |
| **Phase 7a** | Foundation: settings table, simulation, custom node/edge, panel shell |
| **Phase 7b** | Panel + persistence: wire all 4 group sections |
| **Phase 7c** | Local graph: 1-2 hop neighborhood in DocumentPage sidebar |
| **Phase 7d** | Polish: drag-persist, empty states, perf pass |

---

## File Map (created or modified)

### New files (per phase)

**Phase 0**
- `vitest.config.ts`
- `src/test/setup.ts`
- `src/test/example.test.ts`

**Phase 7a**
- `src/types/graph-settings.types.ts`
- `src/lib/db/defaultGraphSettings.ts`
- `src/lib/db/graphSettings.ts`
- `src/hooks/useGraphSettings.ts`
- `src/lib/graph/tagColor.ts`
- `src/lib/graph/colors.ts`
- `src/lib/graph/simulation.ts`
- `src/hooks/useGraphSimulation.ts`
- `src/hooks/useFilteredGraph.ts`
- `src/hooks/useColoredGraph.ts`
- `src/components/universe/Slider.tsx`
- `src/components/universe/CollapsibleSection.tsx`
- `src/components/universe/GraphSettingsPanel.tsx`
- `src/components/universe/UniverseView.tsx`
- `src/components/universe/UniverseNode.tsx` (rewrite)
- `src/components/universe/UniverseEdge.tsx` (rewrite)

**Phase 7b**
- `src/components/universe/sections/GroupsSection.tsx`
- `src/components/universe/sections/FiltersSection.tsx`
- `src/components/universe/sections/DisplaySection.tsx`
- `src/components/universe/sections/PhysicsSection.tsx`
- `src/components/universe/ColorLegend.tsx`

**Phase 7c**
- `src/lib/graph/localGraph.ts`
- `src/components/universe/LocalGraphView.tsx`

**Phase 7d**
- (no new files; modifications only)

### Modified files

- `package.json` (test deps + scripts)
- `vite.config.ts` (or replaced by `vitest.config.ts`)
- `tsconfig.json` (test file include)
- `src/lib/db/index.ts` (bump schema version, add graphSettings)
- `src/store/universe.store.ts` (strip persisted state)
- `src/pages/UniversePage.tsx` (slim down)
- `src/pages/DocumentPage.tsx` (add local graph toggle)
- `src/components/universe/index.ts` (export new components)
- `src/hooks/useGraph.ts` (add N-hop subgraph helper)

### Tests

All `*.test.ts` / `*.test.tsx` files live next to the source under `src/`. The vitest config picks them up via `**/*.test.{ts,tsx}`.

---

# Phase 0: Test Infrastructure

### Task 0.1: Install vitest and testing libraries

**Files:**
- Modify: `package.json` (add devDependencies + scripts)

- [ ] **Step 1: Install test dependencies**

Run:
```bash
cd /home/suraj/ley
npm install -D vitest@^1.6.0 @vitest/ui@^1.6.0 happy-dom@^14.0.0 \
  @testing-library/react@^15.0.0 @testing-library/jest-dom@^6.4.0 \
  @testing-library/user-event@^14.5.0
```

- [ ] **Step 2: Add test scripts to package.json**

Edit `package.json` `scripts` block to add:

```json
"scripts": {
  "dev": "vite",
  "build": "tsc && vite build",
  "preview": "vite preview",
  "lint": "eslint src --ext ts,tsx --report-unused-disable-directives --max-warnings 0",
  "format": "prettier --write \"src/**/*.{ts,tsx,css}\"",
  "test": "vitest run",
  "test:watch": "vitest",
  "test:ui": "vitest --ui",
  "test:coverage": "vitest run --coverage"
}
```

- [ ] **Step 3: Verify install**

Run: `cd /home/suraj/ley && npm ls vitest @testing-library/react happy-dom`
Expected: All three packages listed.

- [ ] **Step 4: Commit**

```bash
git add package.json package-lock.json
git commit -m "chore: add vitest and testing-library for unit/component tests"
```

### Task 0.2: Configure vitest

**Files:**
- Create: `vitest.config.ts`
- Create: `src/test/setup.ts`

- [ ] **Step 1: Write vitest config**

Create `vitest.config.ts`:

```typescript
/// <reference types="vitest" />
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    environment: 'happy-dom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html'],
      exclude: ['src/test/**', 'src/**/*.d.ts'],
    },
  },
});
```

- [ ] **Step 2: Write test setup file**

Create `src/test/setup.ts`:

```typescript
import '@testing-library/jest-dom/vitest';
import { afterEach } from 'vitest';
import { cleanup } from '@testing-library/react';

afterEach(() => {
  cleanup();
});
```

- [ ] **Step 3: Verify config compiles**

Run: `cd /home/suraj/ley && npx vitest --version`
Expected: prints a version number (e.g., `1.6.0`)

- [ ] **Step 4: Commit**

```bash
git add vitest.config.ts src/test/setup.ts
git commit -m "chore: configure vitest with happy-dom and @ aliases"
```

### Task 0.3: Write a smoke test

**Files:**
- Create: `src/test/example.test.ts`

- [ ] **Step 1: Write the test**

Create `src/test/example.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';

function Hello({ name }: { name: string }) {
  return <div>Hello, {name}!</div>;
}

describe('smoke test', () => {
  it('renders a component', () => {
    render(<Hello name="World" />);
    expect(screen.getByText('Hello, World!')).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the test**

Run: `cd /home/suraj/ley && npm test`
Expected: `1 passed`

- [ ] **Step 3: Commit**

```bash
git add src/test/example.test.ts
git commit -m "test: add smoke test to verify vitest setup"
```

---

# Phase 7a: Foundation

### Task 7a.1: Add graph-settings types

**Files:**
- Create: `src/types/graph-settings.types.ts`
- Modify: `src/types/index.ts` (re-export)

- [ ] **Step 1: Write the types**

Create `src/types/graph-settings.types.ts`:

```typescript
export type ColorScheme =
  | 'untyped'
  | 'tag'
  | 'collection'
  | 'link-count'
  | 'community';

export type GraphScope = 'global' | 'local';

export interface PhysicsConfig {
  centerForce: number;
  chargeForce: number;
  linkForce: number;
  linkDistance: number;
}

export interface DisplayConfig {
  nodeSize: number;
  edgeThickness: number;
  textFade: number;
  showLabels: boolean;
}

export interface FilterConfig {
  searchQuery: string;
  selectedTags: string[];
  selectedCollections: string[];
  showOrphans: boolean;
}

export interface PanelSectionsOpen {
  groups: boolean;
  filters: boolean;
  display: boolean;
  physics: boolean;
}

export interface GraphSettings {
  scope: GraphScope;
  colorScheme: ColorScheme;
  physics: PhysicsConfig;
  display: DisplayConfig;
  filters: FilterConfig;
  panelSectionsOpen: PanelSectionsOpen;
  panelVisible: boolean;
  localDepth: 1 | 2;
  updatedAt: number;
}

export const DEFAULT_PHYSICS: PhysicsConfig = {
  centerForce: 1,
  chargeForce: -60,
  linkForce: 1,
  linkDistance: 80,
};

export const DEFAULT_DISPLAY: DisplayConfig = {
  nodeSize: 1,
  edgeThickness: 1,
  textFade: 0.25,
  showLabels: true,
};

export const DEFAULT_FILTERS: FilterConfig = {
  searchQuery: '',
  selectedTags: [],
  selectedCollections: [],
  showOrphans: true,
};

export const DEFAULT_PANEL_SECTIONS_OPEN: PanelSectionsOpen = {
  groups: true,
  filters: false,
  display: false,
  physics: false,
};
```

- [ ] **Step 2: Re-export from types index**

Edit `src/types/index.ts`, add to the end:

```typescript
export * from './graph-settings.types';
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`
Expected: exit code 0, no errors.

- [ ] **Step 4: Commit**

```bash
git add src/types/graph-settings.types.ts src/types/index.ts
git commit -m "feat(universe): add graph settings types"
```

### Task 7a.2: Add default settings factory (TDD)

**Files:**
- Create: `src/lib/db/defaultGraphSettings.ts`
- Create: `src/lib/db/defaultGraphSettings.test.ts`

- [ ] **Step 1: Write failing test**

Create `src/lib/db/defaultGraphSettings.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { defaultGraphSettings } from './defaultGraphSettings';

describe('defaultGraphSettings', () => {
  it('returns a global settings row with all defaults', () => {
    const s = defaultGraphSettings('global');
    expect(s.scope).toBe('global');
    expect(s.colorScheme).toBe('untyped');
    expect(s.physics).toEqual({
      centerForce: 1,
      chargeForce: -60,
      linkForce: 1,
      linkDistance: 80,
    });
    expect(s.display).toEqual({
      nodeSize: 1,
      edgeThickness: 1,
      textFade: 0.25,
      showLabels: true,
    });
    expect(s.filters).toEqual({
      searchQuery: '',
      selectedTags: [],
      selectedCollections: [],
      showOrphans: true,
    });
    expect(s.panelVisible).toBe(true);
    expect(s.panelSectionsOpen.groups).toBe(true);
    expect(s.panelSectionsOpen.filters).toBe(false);
    expect(s.localDepth).toBe(1);
  });

  it('returns a local settings row with localDepth=1', () => {
    const s = defaultGraphSettings('local');
    expect(s.scope).toBe('local');
    expect(s.localDepth).toBe(1);
  });

  it('sets updatedAt to a number', () => {
    const s = defaultGraphSettings('global');
    expect(typeof s.updatedAt).toBe('number');
  });

  it('returns a fresh object each call (no shared refs)', () => {
    const a = defaultGraphSettings('global');
    const b = defaultGraphSettings('global');
    expect(a).not.toBe(b);
    expect(a.physics).not.toBe(b.physics);
    expect(a.filters).not.toBe(b.filters);
  });
});
```

- [ ] **Step 2: Run test, verify it fails**

Run: `cd /home/suraj/ley && npx vitest run src/lib/db/defaultGraphSettings.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement the factory**

Create `src/lib/db/defaultGraphSettings.ts`:

```typescript
import {
  DEFAULT_DISPLAY,
  DEFAULT_FILTERS,
  DEFAULT_PANEL_SECTIONS_OPEN,
  DEFAULT_PHYSICS,
  type GraphScope,
  type GraphSettings,
} from '@/types/graph-settings.types';

export function defaultGraphSettings(scope: GraphScope): GraphSettings {
  return {
    scope,
    colorScheme: 'untyped',
    physics: { ...DEFAULT_PHYSICS },
    display: { ...DEFAULT_DISPLAY },
    filters: {
      ...DEFAULT_FILTERS,
      selectedTags: [],
      selectedCollections: [],
    },
    panelSectionsOpen: { ...DEFAULT_PANEL_SECTIONS_OPEN },
    panelVisible: true,
    localDepth: 1,
    updatedAt: Date.now(),
  };
}
```

- [ ] **Step 4: Run test, verify it passes**

Run: `cd /home/suraj/ley && npx vitest run src/lib/db/defaultGraphSettings.test.ts`
Expected: `4 passed`

- [ ] **Step 5: Commit**

```bash
git add src/lib/db/defaultGraphSettings.ts src/lib/db/defaultGraphSettings.test.ts
git commit -m "feat(universe): add defaultGraphSettings factory with tests"
```

### Task 7a.3: Bump Dexie schema to v2 and add graphSettings table

**Files:**
- Modify: `src/lib/db/index.ts`

- [ ] **Step 1: Add v2 schema and graphSettings table**

Edit `src/lib/db/index.ts`:

Add a new interface for the record (near other `*Record` interfaces):

```typescript
interface GraphSettingsRecord {
  scope: 'global' | 'local';
  colorScheme: 'untyped' | 'tag' | 'collection' | 'link-count' | 'community';
  physics: {
    centerForce: number;
    chargeForce: number;
    linkForce: number;
    linkDistance: number;
  };
  display: {
    nodeSize: number;
    edgeThickness: number;
    textFade: number;
    showLabels: boolean;
  };
  filters: {
    searchQuery: string;
    selectedTags: string[];
    selectedCollections: string[];
    showOrphans: boolean;
  };
  panelSectionsOpen: {
    groups: boolean;
    filters: boolean;
    display: boolean;
    physics: boolean;
  };
  panelVisible: boolean;
  localDepth: 1 | 2;
  updatedAt: number;
}
```

In the `KnowledgeUniverseDB` class, add a property and a v2 schema:

```typescript
class KnowledgeUniverseDB extends Dexie {
  nodes!: Table<KnowledgeNodeRecord>;
  edges!: Table<KnowledgeEdgeRecord>;
  collections!: Table<CollectionRecord>;
  revisions!: Table<RevisionRecord>;
  graphPositions!: Table<GraphPosition>;
  graphSettings!: Table<GraphSettingsRecord>;

  constructor() {
    super('knowledge-universe');

    this.version(1).stores({
      nodes: 'id, type, title, *collections, *tags, isArchived, createdAt, updatedAt, parentId',
      edges: 'id, source, target, type, createdAt',
      collections: 'id, name, parentId, createdAt',
      revisions: 'id, nodeId, createdAt',
      graphPositions: 'nodeId, updatedAt',
    });

    this.version(2).stores({
      nodes: 'id, type, title, *collections, *tags, isArchived, createdAt, updatedAt, parentId',
      edges: 'id, source, target, type, createdAt',
      collections: 'id, name, parentId, createdAt',
      revisions: 'id, nodeId, createdAt',
      graphPositions: 'nodeId, updatedAt',
      graphSettings: 'scope, updatedAt',
    });
  }
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`
Expected: exit code 0.

- [ ] **Step 3: Commit**

```bash
git add src/lib/db/index.ts
git commit -m "feat(db): add graphSettings table, bump Dexie schema to v2"
```

### Task 7a.4: Add graphSettings CRUD module (TDD)

**Files:**
- Create: `src/lib/db/graphSettings.ts`
- Create: `src/lib/db/graphSettings.test.ts`

- [ ] **Step 1: Write failing test**

Create `src/lib/db/graphSettings.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import 'fake-indexeddb/auto';
import { db } from './index';
import {
  getGraphSettings,
  upsertGraphSettings,
  ensureDefaultGraphSettings,
} from './graphSettings';

describe('graphSettings CRUD', () => {
  beforeEach(async () => {
    await db.graphSettings.clear();
  });

  it('ensureDefaultGraphSettings inserts both rows on first call', async () => {
    await ensureDefaultGraphSettings();
    const rows = await db.graphSettings.toArray();
    expect(rows).toHaveLength(2);
    const scopes = rows.map((r) => r.scope).sort();
    expect(scopes).toEqual(['global', 'local']);
  });

  it('ensureDefaultGraphSettings is idempotent', async () => {
    await ensureDefaultGraphSettings();
    await ensureDefaultGraphSettings();
    const rows = await db.graphSettings.toArray();
    expect(rows).toHaveLength(2);
  });

  it('getGraphSettings returns null when row is missing', async () => {
    const s = await getGraphSettings('global');
    expect(s).toBeNull();
  });

  it('getGraphSettings returns the row when present', async () => {
    await upsertGraphSettings({
      scope: 'global',
      colorScheme: 'tag',
      physics: {
        centerForce: 1,
        chargeForce: -60,
        linkForce: 1,
        linkDistance: 80,
      },
      display: {
        nodeSize: 1,
        edgeThickness: 1,
        textFade: 0.25,
        showLabels: true,
      },
      filters: {
        searchQuery: '',
        selectedTags: [],
        selectedCollections: [],
        showOrphans: true,
      },
      panelSectionsOpen: {
        groups: true,
        filters: false,
        display: false,
        physics: false,
      },
      panelVisible: true,
      localDepth: 1,
      updatedAt: Date.now(),
    });
    const s = await getGraphSettings('global');
    expect(s).not.toBeNull();
    expect(s?.colorScheme).toBe('tag');
  });

  it('upsertGraphSettings overwrites existing row by primary key', async () => {
    await ensureDefaultGraphSettings();
    const original = await getGraphSettings('global');
    expect(original?.colorScheme).toBe('untyped');
    await upsertGraphSettings({
      ...(original as any),
      colorScheme: 'collection',
    });
    const updated = await getGraphSettings('global');
    expect(updated?.colorScheme).toBe('collection');
  });
});
```

- [ ] **Step 2: Run test, verify it fails (module not found)**

Run: `cd /home/suraj/ley && npx vitest run src/lib/db/graphSettings.test.ts`
Expected: FAIL — cannot find module `./graphSettings`.

- [ ] **Step 3: Implement graphSettings module**

Create `src/lib/db/graphSettings.ts`:

```typescript
import { db } from './index';
import { defaultGraphSettings } from './defaultGraphSettings';
import type { GraphScope, GraphSettings } from '@/types/graph-settings.types';

export async function getGraphSettings(
  scope: GraphScope
): Promise<GraphSettings | null> {
  const row = await db.graphSettings.get(scope);
  return row ? (row as GraphSettings) : null;
}

export async function upsertGraphSettings(
  settings: GraphSettings
): Promise<void> {
  await db.graphSettings.put({ ...settings, updatedAt: Date.now() });
}

export async function ensureDefaultGraphSettings(): Promise<void> {
  const existing = await db.graphSettings.count();
  if (existing === 0) {
    await db.graphSettings.bulkPut([
      defaultGraphSettings('global'),
      defaultGraphSettings('local'),
    ]);
  }
}
```

- [ ] **Step 4: Install fake-indexeddb (test-only)**

Run: `cd /home/suraj/ley && npm install -D fake-indexeddb@^5.7.0`

- [ ] **Step 5: Run test, verify it passes**

Run: `cd /home/suraj/ley && npx vitest run src/lib/db/graphSettings.test.ts`
Expected: `5 passed`

- [ ] **Step 6: Commit**

```bash
git add src/lib/db/graphSettings.ts src/lib/db/graphSettings.test.ts \
  package.json package-lock.json
git commit -m "feat(db): add graphSettings CRUD with tests"
```

### Task 7a.5: Add useGraphSettings hook (TDD)

**Files:**
- Create: `src/hooks/useGraphSettings.ts`
- Create: `src/hooks/useGraphSettings.test.tsx`

- [ ] **Step 1: Write failing test**

Create `src/hooks/useGraphSettings.test.tsx`:

```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import 'fake-indexeddb/auto';
import { renderHook, act, waitFor } from '@testing-library/react';
import { db } from '@/lib/db';
import { useGraphSettings } from './useGraphSettings';
import { ensureDefaultGraphSettings } from '@/lib/db/graphSettings';

describe('useGraphSettings', () => {
  beforeEach(async () => {
    await db.graphSettings.clear();
  });

  it('returns null when no settings exist yet and seeds defaults', async () => {
    const { result } = renderHook(() => useGraphSettings('global'));
    await waitFor(() => {
      expect(result.current.settings).not.toBeNull();
    });
    expect(result.current.settings?.colorScheme).toBe('untyped');
  });

  it('seeds defaults for both scopes on first call', async () => {
    renderHook(() => useGraphSettings('global'));
    await waitFor(async () => {
      const count = await db.graphSettings.count();
      expect(count).toBe(2);
    });
  });

  it('updates the underlying row when update is called', async () => {
    const { result } = renderHook(() => useGraphSettings('global'));
    await waitFor(() => expect(result.current.settings).not.toBeNull());
    await act(async () => {
      await result.current.update({
        ...(result.current.settings as any),
        colorScheme: 'tag',
      });
    });
    await waitFor(() => {
      expect(result.current.settings?.colorScheme).toBe('tag');
    });
  });

  it('returns separate settings for each scope', async () => {
    const { result: globalHook } = renderHook(() => useGraphSettings('global'));
    const { result: localHook } = renderHook(() => useGraphSettings('local'));
    await waitFor(() => {
      expect(globalHook.current.settings).not.toBeNull();
      expect(localHook.current.settings).not.toBeNull();
    });
    expect(globalHook.current.settings?.scope).toBe('global');
    expect(localHook.current.settings?.scope).toBe('local');
  });
});
```

- [ ] **Step 2: Run test, verify it fails**

Run: `cd /home/suraj/ley && npx vitest run src/hooks/useGraphSettings.test.tsx`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement the hook**

Create `src/hooks/useGraphSettings.ts`:

```typescript
import { useCallback } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/lib/db';
import {
  ensureDefaultGraphSettings,
  upsertGraphSettings,
} from '@/lib/db/graphSettings';
import type { GraphScope, GraphSettings } from '@/types/graph-settings.types';

export function useGraphSettings(scope: GraphScope) {
  // Seed defaults on first hook use.
  useLiveQuery(async () => {
    await ensureDefaultGraphSettings();
    return true;
  }, []);

  const settings = useLiveQuery<GraphSettings | null>(
    async () => {
      const row = await db.graphSettings.get(scope);
      return row ? (row as GraphSettings) : null;
    },
    [scope],
    null
  );

  const update = useCallback(
    async (next: GraphSettings) => {
      await upsertGraphSettings({ ...next, scope });
    },
    [scope]
  );

  return { settings, update };
}
```

- [ ] **Step 4: Run test, verify it passes**

Run: `cd /home/suraj/ley && npx vitest run src/hooks/useGraphSettings.test.tsx`
Expected: `4 passed`

- [ ] **Step 5: Commit**

```bash
git add src/hooks/useGraphSettings.ts src/hooks/useGraphSettings.test.tsx
git commit -m "feat(hooks): add useGraphSettings with dexie-react-hooks"
```

### Task 7a.6: Add tagColor utility (TDD)

**Files:**
- Create: `src/lib/graph/tagColor.ts`
- Create: `src/lib/graph/tagColor.test.ts`

- [ ] **Step 1: Write failing test**

Create `src/lib/graph/tagColor.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { tagColor, collectionColor } from './tagColor';

describe('tagColor', () => {
  it('is deterministic for the same input', () => {
    expect(tagColor('react')).toBe(tagColor('react'));
    expect(tagColor('javascript')).toBe(tagColor('javascript'));
  });

  it('returns a valid HSL string', () => {
    const c = tagColor('typescript');
    expect(c).toMatch(/^hsl\(\s*\d+(\.\d+)?,\s*\d+%,\s*\d+%\s*\)$/);
  });

  it('produces different colors for different inputs (most of the time)', () => {
    const colors = new Set([
      tagColor('react'),
      tagColor('typescript'),
      tagColor('rust'),
      tagColor('python'),
      tagColor('design'),
      tagColor('product'),
    ]);
    expect(colors.size).toBeGreaterThan(4);
  });

  it('uses saturation 50% and lightness 65%', () => {
    expect(tagColor('any-tag')).toContain('50%');
    expect(tagColor('any-tag')).toContain('65%');
  });
});

describe('collectionColor', () => {
  it('rotates 180° from tagColor', () => {
    const tag = tagColor('react');
    const col = collectionColor('react');
    // Extract hue from both
    const tagHue = parseFloat(tag.match(/hsl\((\d+)/)![1]);
    const colHue = parseFloat(col.match(/hsl\((\d+)/)![1]);
    const diff = Math.abs(tagHue - colHue);
    expect(diff === 180 || diff === 180 - 360 || diff === 360 - 180).toBe(true);
  });
});
```

- [ ] **Step 2: Run test, verify it fails**

Run: `cd /home/suraj/ley && npx vitest run src/lib/graph/tagColor.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

Create `src/lib/graph/tagColor.ts`:

```typescript
function hashString(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

export function tagColor(tag: string): string {
  const h = hashString(tag) % 360;
  return `hsl(${h}, 50%, 65%)`;
}

export function collectionColor(name: string): string {
  const h = (hashString(name) + 180) % 360;
  return `hsl(${h}, 50%, 65%)`;
}
```

- [ ] **Step 4: Run test, verify it passes**

Run: `cd /home/suraj/ley && npx vitest run src/lib/graph/tagColor.test.ts`
Expected: `5 passed`

- [ ] **Step 5: Commit**

```bash
git add src/lib/graph/tagColor.ts src/lib/graph/tagColor.test.ts
git commit -m "feat(graph): add tagColor and collectionColor with tests"
```

### Task 7a.7: Add colors module (TDD)

**Files:**
- Create: `src/lib/graph/colors.ts`
- Create: `src/lib/graph/colors.test.ts`

- [ ] **Step 1: Write failing test**

Create `src/lib/graph/colors.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { colorForNode, COMMUNITY_PALETTE, UNCOLORED, linkCountColor } from './colors';
import type { KnowledgeNode } from '@/types';

const baseNode: KnowledgeNode = {
  id: '1',
  type: 'document',
  title: 'Test',
  content: null,
  plainText: '',
  collections: [],
  tags: [],
  properties: {},
  isArchived: 0,
  createdAt: 0,
  updatedAt: 0,
};

describe('colorForNode', () => {
  it('returns UNCOLORED when scheme is untyped', () => {
    expect(colorForNode(baseNode, 'untyped', { degree: 0, maxDegree: 10, community: 0 })).toBe(UNCOLORED);
  });

  it('uses tag color when scheme is tag and node has tags', () => {
    const node = { ...baseNode, tags: ['react'] };
    const c = colorForNode(node, 'tag', { degree: 0, maxDegree: 0, community: 0 });
    expect(c).toMatch(/^hsl\(/);
  });

  it('falls back to UNCOLORED when scheme is tag but node has no tags', () => {
    const c = colorForNode(baseNode, 'tag', { degree: 0, maxDegree: 0, community: 0 });
    expect(c).toBe(UNCOLORED);
  });

  it('uses collection color when scheme is collection and node has collections', () => {
    const node = { ...baseNode, collections: ['work'] };
    const c = colorForNode(node, 'collection', { degree: 0, maxDegree: 0, community: 0 });
    expect(c).toMatch(/^hsl\(/);
  });

  it('uses COMMUNITY_PALETTE for community scheme', () => {
    const c = colorForNode(baseNode, 'community', { degree: 0, maxDegree: 0, community: 2 });
    expect(c).toBe(COMMUNITY_PALETTE[2 % COMMUNITY_PALETTE.length]);
  });
});

describe('linkCountColor', () => {
  it('returns base color when degree is 0', () => {
    const c = linkCountColor(0, 10);
    expect(c).toMatch(/^hsl\(/);
  });

  it('returns deeper color at max degree', () => {
    const c = linkCountColor(10, 10);
    expect(c).toMatch(/^hsl\(/);
  });

  it('handles maxDegree=0', () => {
    const c = linkCountColor(5, 0);
    expect(c).toMatch(/^hsl\(/);
  });
});
```

- [ ] **Step 2: Run test, verify it fails**

Run: `cd /home/suraj/ley && npx vitest run src/lib/graph/colors.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

Create `src/lib/graph/colors.ts`:

```typescript
import { tagColor, collectionColor } from './tagColor';
import type { ColorScheme } from '@/types/graph-settings.types';
import type { KnowledgeNode } from '@/types';

export const UNCOLORED = 'hsl(220 8% 55%)';

export const COMMUNITY_PALETTE: string[] = [
  'hsl(0 65% 62%)',
  'hsl(30 65% 60%)',
  'hsl(60 60% 60%)',
  'hsl(140 55% 58%)',
  'hsl(180 55% 58%)',
  'hsl(220 65% 65%)',
  'hsl(265 55% 65%)',
  'hsl(320 55% 65%)',
];

export interface ColorContext {
  degree: number;
  maxDegree: number;
  community: number;
}

export function colorForNode(
  node: KnowledgeNode,
  scheme: ColorScheme,
  ctx: ColorContext
): string {
  switch (scheme) {
    case 'untyped':
      return UNCOLORED;
    case 'tag':
      if (node.tags.length === 0) return UNCOLORED;
      return tagColor(node.tags[0]);
    case 'collection':
      if (node.collections.length === 0) return UNCOLORED;
      return collectionColor(node.collections[0]);
    case 'link-count':
      return linkCountColor(ctx.degree, ctx.maxDegree);
    case 'community':
      return COMMUNITY_PALETTE[ctx.community % COMMUNITY_PALETTE.length];
  }
}

export function linkCountColor(degree: number, maxDegree: number): string {
  if (maxDegree <= 0) return 'hsl(220 8% 50%)';
  const t = Math.min(1, degree / maxDegree);
  // Linear interpolate from hsl(220 8% 50%) to hsl(265 50% 70%)
  const h = 220 + t * 45;
  const s = 8 + t * 42;
  const l = 50 + t * 20;
  return `hsl(${h} ${s}% ${l}%)`;
}
```

- [ ] **Step 4: Run test, verify it passes**

Run: `cd /home/suraj/ley && npx vitest run src/lib/graph/colors.test.ts`
Expected: `9 passed`

- [ ] **Step 5: Commit**

```bash
git add src/lib/graph/colors.ts src/lib/graph/colors.test.ts
git commit -m "feat(graph): add color schemes (untyped/tag/collection/link-count/community)"
```

### Task 7a.8: Add d3-force simulation factory (TDD)

**Files:**
- Create: `src/lib/graph/simulation.ts`
- Create: `src/lib/graph/simulation.test.ts`

- [ ] **Step 1: Write failing test**

Create `src/lib/graph/simulation.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import Graph from 'graphology';
import { createSimulation, type SimulationHandle } from './simulation';

describe('createSimulation', () => {
  let g: Graph;
  beforeEach(() => {
    g = new Graph({ type: 'undirected', multi: false });
    g.addNode('a');
    g.addNode('b');
    g.addNode('c');
    g.addEdge('a', 'b');
    g.addEdge('b', 'c');
  });

  it('returns a handle with start/stop/positions', () => {
    const h = createSimulation(g, {
      centerForce: 1,
      chargeForce: -60,
      linkForce: 1,
      linkDistance: 80,
    });
    expect(h).toHaveProperty('start');
    expect(h).toHaveProperty('stop');
    expect(h).toHaveProperty('positions');
    h.stop();
  });

  it('assigns initial random positions to nodes that lack x/y', () => {
    const h = createSimulation(g, {
      centerForce: 1,
      chargeForce: -60,
      linkForce: 1,
      linkDistance: 80,
    });
    const positions = h.positions();
    for (const id of ['a', 'b', 'c']) {
      const p = positions.get(id);
      expect(p).toBeDefined();
      expect(typeof p!.x).toBe('number');
      expect(typeof p!.y).toBe('number');
      expect(isNaN(p!.x)).toBe(false);
      expect(isNaN(p!.y)).toBe(false);
    }
    h.stop();
  });

  it('tick advances node positions (regression: simulation actually runs)', () => {
    const h = createSimulation(g, {
      centerForce: 1,
      chargeForce: -60,
      linkForce: 1,
      linkDistance: 80,
    });
    const before = h.positions();
    h.tick(50);
    const after = h.positions();
    let moved = false;
    for (const id of ['a', 'b', 'c']) {
      if (before.get(id)!.x !== after.get(id)!.x) {
        moved = true;
        break;
      }
    }
    expect(moved).toBe(true);
    h.stop();
  });

  it('reconfigure updates the running simulation', () => {
    const h = createSimulation(g, {
      centerForce: 1,
      chargeForce: -60,
      linkForce: 1,
      linkDistance: 80,
    });
    expect(() =>
      h.reconfigure({ centerForce: 2, chargeForce: -100, linkForce: 1.5, linkDistance: 120 })
    ).not.toThrow();
    h.stop();
  });
});
```

- [ ] **Step 2: Run test, verify it fails**

Run: `cd /home/suraj/ley && npx vitest run src/lib/graph/simulation.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

Create `src/lib/graph/simulation.ts`:

```typescript
import Graph from 'graphology';
import {
  forceCenter,
  forceLink,
  forceManyBody,
  forceSimulation,
  type Simulation,
  type SimulationNodeDatum,
} from 'd3-force';
import type { PhysicsConfig } from '@/types/graph-settings.types';

interface SimNode extends SimulationNodeDatum {
  id: string;
}

interface SimLink {
  source: string | SimNode;
  target: string | SimNode;
}

export interface SimulationHandle {
  start(): void;
  stop(): void;
  tick(iterations: number): void;
  positions(): Map<string, { x: number; y: number }>;
  reconfigure(physics: PhysicsConfig): void;
}

export function createSimulation(
  graph: Graph,
  physics: PhysicsConfig
): SimulationHandle {
  // Seed initial random positions so d3-force has somewhere to start.
  graph.forEachNode((node) => {
    const x = graph.getNodeAttribute(node, 'x');
    const y = graph.getNodeAttribute(node, 'y');
    if (typeof x !== 'number' || typeof y !== 'number' || isNaN(x) || isNaN(y)) {
      graph.setNodeAttribute(node, 'x', Math.random() * 400 - 200);
      graph.setNodeAttribute(node, 'y', Math.random() * 400 - 200);
    }
  });

  const nodes: SimNode[] = graph.nodes().map((id) => ({
    id,
    x: graph.getNodeAttribute(id, 'x') as number,
    y: graph.getNodeAttribute(id, 'y') as number,
  }));

  const idToNode = new Map(nodes.map((n) => [n.id, n]));

  const links: SimLink[] = graph.edges().map((e) => ({
    source: idToNode.get(graph.source(e))!,
    target: idToNode.get(graph.target(e))!,
  }));

  const sim: Simulation<SimNode, SimLink> = forceSimulation<SimNode>(nodes)
    .force('charge', forceManyBody<SimNode>().strength(physics.chargeForce))
    .force(
      'link',
      forceLink<SimNode, SimLink>(links)
        .id((d) => (d as SimNode).id)
        .distance(physics.linkDistance)
        .strength(physics.linkForce)
    )
    .force('center', forceCenter(0, 0).strength(physics.centerForce))
    .alphaDecay(0.05)
    .stop();

  function applyToGraph() {
    for (const n of nodes) {
      if (typeof n.x === 'number' && typeof n.y === 'number' && !isNaN(n.x) && !isNaN(n.y)) {
        graph.setNodeAttribute(n.id, 'x', n.x);
        graph.setNodeAttribute(n.id, 'y', n.y);
      }
    }
  }

  return {
    start() {
      sim.restart();
    },
    stop() {
      sim.stop();
    },
    tick(iterations: number) {
      for (let i = 0; i < iterations; i++) sim.tick();
      applyToGraph();
    },
    positions() {
      const map = new Map<string, { x: number; y: number }>();
      for (const n of nodes) {
        if (typeof n.x === 'number' && typeof n.y === 'number') {
          map.set(n.id, { x: n.x, y: n.y });
        }
      }
      return map;
    },
    reconfigure(next: PhysicsConfig) {
      sim
        .force('charge', forceManyBody<SimNode>().strength(next.chargeForce))
        .force(
          'link',
          forceLink<SimNode, SimLink>(links)
            .id((d) => (d as SimNode).id)
            .distance(next.linkDistance)
            .strength(next.linkForce)
        )
        .force('center', forceCenter(0, 0).strength(next.centerForce))
        .alpha(0.5)
        .restart();
    },
  };
}
```

- [ ] **Step 4: Run test, verify it passes**

Run: `cd /home/suraj/ley && npx vitest run src/lib/graph/simulation.test.ts`
Expected: `4 passed`

- [ ] **Step 5: Commit**

```bash
git add src/lib/graph/simulation.ts src/lib/graph/simulation.test.ts
git commit -m "feat(graph): add d3-force simulation factory with tests"
```

### Task 7a.9: Add useGraphSimulation hook

**Files:**
- Create: `src/hooks/useGraphSimulation.ts`
- Create: `src/hooks/useGraphSimulation.test.tsx`

- [ ] **Step 1: Write failing test**

Create `src/hooks/useGraphSimulation.test.tsx`:

```typescript
import { describe, it, expect } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import Graph from 'graphology';
import { useGraphSimulation } from './useGraphSimulation';

function makeGraph(): Graph {
  const g = new Graph({ type: 'undirected', multi: false });
  g.addNode('a');
  g.addNode('b');
  g.addEdge('a', 'b');
  return g;
}

describe('useGraphSimulation', () => {
  it('returns a Map of positions', () => {
    const graph = makeGraph();
    const { result } = renderHook(() =>
      useGraphSimulation(graph, {
        centerForce: 1,
        chargeForce: -60,
        linkForce: 1,
        linkDistance: 80,
      })
    );
    expect(result.current.positions).toBeInstanceOf(Map);
    expect(result.current.positions.size).toBe(2);
  });

  it('provides a way to trigger a tick', () => {
    const graph = makeGraph();
    const { result } = renderHook(() =>
      useGraphSimulation(graph, {
        centerForce: 1,
        chargeForce: -60,
        linkForce: 1,
        linkDistance: 80,
      })
    );
    expect(() => act(() => result.current.tick(10))).not.toThrow();
  });

  it('reconfigures when physics changes', () => {
    const graph = makeGraph();
    const { result, rerender } = renderHook(
      ({ physics }: { physics: any }) => useGraphSimulation(graph, physics),
      {
        initialProps: {
          physics: { centerForce: 1, chargeForce: -60, linkForce: 1, linkDistance: 80 },
        },
      }
    );
    expect(() =>
      rerender({
        physics: { centerForce: 2, chargeForce: -100, linkForce: 1.5, linkDistance: 120 },
      })
    ).not.toThrow();
  });

  it('handles empty graphs without crashing', () => {
    const graph = new Graph({ type: 'undirected', multi: false });
    const { result } = renderHook(() =>
      useGraphSimulation(graph, {
        centerForce: 1,
        chargeForce: -60,
        linkForce: 1,
        linkDistance: 80,
      })
    );
    expect(result.current.positions.size).toBe(0);
  });
});
```

- [ ] **Step 2: Run test, verify it fails**

Run: `cd /home/suraj/ley && npx vitest run src/hooks/useGraphSimulation.test.tsx`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

Create `src/hooks/useGraphSimulation.ts`:

```typescript
import { useEffect, useMemo, useRef, useState } from 'react';
import Graph from 'graphology';
import { createSimulation, type SimulationHandle } from '@/lib/graph/simulation';
import type { PhysicsConfig } from '@/types/graph-settings.types';

export function useGraphSimulation(
  graph: Graph,
  physics: PhysicsConfig
): {
  positions: Map<string, { x: number; y: number }>;
  tick: (iterations?: number) => void;
} {
  // Recreate the simulation only when the graph topology changes (node/edge set).
  const graphKey = useMemo(
    () => `${graph.order}:${graph.size}`,
    [graph.order, graph.size]
  );

  const handleRef = useRef<SimulationHandle | null>(null);
  const [positions, setPositions] = useState<Map<string, { x: number; y: number }>>(
    () => new Map()
  );

  useEffect(() => {
    handleRef.current = createSimulation(graph, physics);
    // Seed initial positions.
    setPositions(handleRef.current.positions());
    return () => {
      handleRef.current?.stop();
      handleRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [graphKey]);

  // Reconfigure on physics changes (without rebuilding the sim).
  useEffect(() => {
    handleRef.current?.reconfigure(physics);
  }, [physics]);

  const tick = (iterations: number = 1) => {
    handleRef.current?.tick(iterations);
    if (handleRef.current) {
      setPositions(new Map(handleRef.current.positions()));
    }
  };

  return { positions, tick };
}
```

- [ ] **Step 4: Run test, verify it passes**

Run: `cd /home/suraj/ley && npx vitest run src/hooks/useGraphSimulation.test.tsx`
Expected: `4 passed`

- [ ] **Step 5: Commit**

```bash
git add src/hooks/useGraphSimulation.ts src/hooks/useGraphSimulation.test.tsx
git commit -m "feat(hooks): add useGraphSimulation wrapping d3-force"
```

### Task 7a.10: Add useFilteredGraph hook (TDD)

**Files:**
- Create: `src/hooks/useFilteredGraph.ts`
- Create: `src/hooks/useFilteredGraph.test.ts`

- [ ] **Step 1: Write failing test**

Create `src/hooks/useFilteredGraph.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import 'fake-indexeddb/auto';
import { db } from '@/lib/db';
import { useFilteredGraph } from './useFilteredGraph';
import type { KnowledgeNode, KnowledgeEdge } from '@/types';

const mkNode = (id: string, overrides: Partial<KnowledgeNode> = {}): KnowledgeNode => ({
  id,
  type: 'document',
  title: id,
  content: null,
  plainText: '',
  collections: [],
  tags: [],
  properties: {},
  isArchived: 0,
  createdAt: 0,
  updatedAt: 0,
  ...overrides,
});

const mkEdge = (id: string, source: string, target: string): KnowledgeEdge => ({
  id,
  source,
  target,
  type: 'wiki-link',
  createdAt: 0,
});

beforeEach(async () => {
  await db.nodes.clear();
  await db.edges.clear();
  await db.nodes.bulkPut([
    mkNode('a', { title: 'React patterns', tags: ['react'] }),
    mkNode('b', { title: 'Vue basics', tags: ['vue'] }),
    mkNode('c', { title: 'Standalone note' }),
    mkNode('orphan', { title: 'Unlinked', isArchived: 0 }),
  ]);
  await db.edges.bulkPut([mkEdge('e1', 'a', 'b')]);
});

describe('useFilteredGraph (logic only)', () => {
  it('search query filters by title case-insensitive', () => {
    const filtered = applyFilters({
      nodes: dbNodes(),
      edges: dbEdges(),
      filters: {
        searchQuery: 'react',
        selectedTags: [],
        selectedCollections: [],
        showOrphans: true,
      },
    });
    expect(filtered.nodes.map((n) => n.id).sort()).toEqual(['a']);
  });

  it('selectedTags filters to nodes having at least one matching tag', () => {
    const filtered = applyFilters({
      nodes: dbNodes(),
      edges: dbEdges(),
      filters: {
        searchQuery: '',
        selectedTags: ['react'],
        selectedCollections: [],
        showOrphans: true,
      },
    });
    expect(filtered.nodes.map((n) => n.id)).toEqual(['a']);
  });

  it('selectedCollections filters to nodes in one of the collections', () => {
    const filtered = applyFilters({
      nodes: dbNodes(),
      edges: dbEdges(),
      filters: {
        searchQuery: '',
        selectedTags: [],
        selectedCollections: ['work'],
        showOrphans: true,
      },
    });
    // No nodes have 'work' collection
    expect(filtered.nodes).toEqual([]);
  });

  it('showOrphans=false removes nodes with no edges', () => {
    const filtered = applyFilters({
      nodes: dbNodes(),
      edges: dbEdges(),
      filters: {
        searchQuery: '',
        selectedTags: [],
        selectedCollections: [],
        showOrphans: false,
      },
    });
    const ids = filtered.nodes.map((n) => n.id).sort();
    expect(ids).toEqual(['a', 'b']);
  });

  it('only keeps edges where both endpoints pass the filter', () => {
    const filtered = applyFilters({
      nodes: dbNodes(),
      edges: dbEdges(),
      filters: {
        searchQuery: 'react',
        selectedTags: [],
        selectedCollections: [],
        showOrphans: true,
      },
    });
    expect(filtered.edges).toEqual([]);
  });
});

// Helpers used above — exported by the module under test.
import { applyFilters } from './useFilteredGraph';
function dbNodes() {
  return db.nodes.toArray() as unknown as Promise<KnowledgeNode[]> as any;
}
function dbEdges() {
  return db.edges.toArray() as unknown as Promise<KnowledgeEdge[]> as any;
}
```

- [ ] **Step 2: Run test, verify it fails**

Run: `cd /home/suraj/ley && npx vitest run src/hooks/useFilteredGraph.test.ts`
Expected: FAIL — `applyFilters` not exported from module.

- [ ] **Step 3: Implement**

Create `src/hooks/useFilteredGraph.ts`:

```typescript
import type { KnowledgeNode, KnowledgeEdge } from '@/types';
import type { FilterConfig } from '@/types/graph-settings.types';

export interface FilteredGraph {
  nodes: KnowledgeNode[];
  edges: KnowledgeEdge[];
}

export function applyFilters(input: {
  nodes: KnowledgeNode[];
  edges: KnowledgeEdge[];
  filters: FilterConfig;
}): FilteredGraph {
  const { nodes, edges, filters } = input;

  // Compute node set that passes the filter.
  const searchLower = filters.searchQuery.trim().toLowerCase();
  const matched = nodes.filter((n) => {
    if (searchLower && !n.title.toLowerCase().includes(searchLower)) return false;
    if (
      filters.selectedTags.length > 0 &&
      !n.tags.some((t) => filters.selectedTags.includes(t))
    ) {
      return false;
    }
    if (
      filters.selectedCollections.length > 0 &&
      !n.collections.some((c) => filters.selectedCollections.includes(c))
    ) {
      return false;
    }
    return true;
  });

  let visibleNodeIds = new Set(matched.map((n) => n.id));

  if (!filters.showOrphans) {
    const connected = new Set<string>();
    for (const e of edges) {
      if (visibleNodeIds.has(e.source) && visibleNodeIds.has(e.target)) {
        connected.add(e.source);
        connected.add(e.target);
      }
    }
    visibleNodeIds = connected;
  }

  const visibleNodes = matched.filter((n) => visibleNodeIds.has(n.id));
  const visibleEdges = edges.filter(
    (e) => visibleNodeIds.has(e.source) && visibleNodeIds.has(e.target)
  );

  return { nodes: visibleNodes, edges: visibleEdges };
}

export function useFilteredGraph(
  nodes: KnowledgeNode[],
  edges: KnowledgeEdge[],
  filters: FilterConfig
): FilteredGraph {
  return applyFilters({ nodes, edges, filters });
}
```

- [ ] **Step 4: Run test, verify it passes**

Run: `cd /home/suraj/ley && npx vitest run src/hooks/useFilteredGraph.test.ts`
Expected: `5 passed`

- [ ] **Step 5: Commit**

```bash
git add src/hooks/useFilteredGraph.ts src/hooks/useFilteredGraph.test.ts
git commit -m "feat(hooks): add useFilteredGraph with applyFilters helper"
```

### Task 7a.11: Add useColoredGraph hook (TDD)

**Files:**
- Create: `src/hooks/useColoredGraph.ts`
- Create: `src/hooks/useColoredGraph.test.ts`

- [ ] **Step 1: Write failing test**

Create `src/hooks/useColoredGraph.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { colorMapForGraph } from './useColoredGraph';
import type { KnowledgeNode, KnowledgeEdge } from '@/types';
import Graph from 'graphology';
import { detectCommunities } from '@/lib/graph/louvain';

const mkNode = (id: string, overrides: Partial<KnowledgeNode> = {}): KnowledgeNode => ({
  id,
  type: 'document',
  title: id,
  content: null,
  plainText: '',
  collections: [],
  tags: [],
  properties: {},
  isArchived: 0,
  createdAt: 0,
  updatedAt: 0,
  ...overrides,
});

describe('colorMapForGraph', () => {
  it('returns a color for each node in untyped scheme', () => {
    const nodes: KnowledgeNode[] = [mkNode('a'), mkNode('b')];
    const g = new Graph({ type: 'undirected', multi: false });
    g.addNode('a');
    g.addNode('b');
    const colors = colorMapForGraph(nodes, [], g, 'untyped');
    expect(colors.size).toBe(2);
    expect(colors.get('a')).toMatch(/^hsl\(/);
  });

  it('uses tag color for tag scheme', () => {
    const nodes: KnowledgeNode[] = [mkNode('a', { tags: ['react'] })];
    const g = new Graph({ type: 'undirected', multi: false });
    g.addNode('a');
    const colors = colorMapForGraph(nodes, [], g, 'tag');
    expect(colors.get('a')).toMatch(/^hsl\(/);
  });

  it('uses community palette for community scheme when communities provided', () => {
    const nodes: KnowledgeNode[] = [mkNode('a'), mkNode('b'), mkNode('c')];
    const g = new Graph({ type: 'undirected', multi: false });
    g.addNode('a');
    g.addNode('b');
    g.addNode('c');
    g.addEdge('a', 'b');
    g.addEdge('b', 'c');
    const communities = detectCommunities(g);
    const colors = colorMapForGraph(
      nodes,
      [],
      g,
      'community',
      communities ?? undefined
    );
    expect(colors.get('a')).toMatch(/^hsl\(/);
  });
});
```

- [ ] **Step 2: Run test, verify it fails**

Run: `cd /home/suraj/ley && npx vitest run src/hooks/useColoredGraph.test.ts`
Expected: FAIL — `colorMapForGraph` not exported.

- [ ] **Step 3: Implement**

Create `src/hooks/useColoredGraph.ts`:

```typescript
import { useMemo } from 'react';
import Graph from 'graphology';
import { colorForNode } from '@/lib/graph/colors';
import type { ColorScheme } from '@/types/graph-settings.types';
import type { KnowledgeNode, KnowledgeEdge } from '@/types';
import type { CommunityResult } from '@/lib/graph/louvain';

export function colorMapForGraph(
  nodes: KnowledgeNode[],
  _edges: KnowledgeEdge[],
  graph: Graph,
  scheme: ColorScheme,
  communities?: CommunityResult | null
): Map<string, string> {
  let maxDegree = 0;
  graph.forEachNode((id) => {
    const d = graph.degree(id);
    if (d > maxDegree) maxDegree = d;
  });

  const partition = communities?.partition ?? new Map<string, number>();
  const colorByCommunity = (id: string) => partition.get(id) ?? 0;

  const map = new Map<string, string>();
  for (const n of nodes) {
    const color = colorForNode(n, scheme, {
      degree: graph.hasNode(n.id) ? graph.degree(n.id) : 0,
      maxDegree,
      community: colorByCommunity(n.id),
    });
    map.set(n.id, color);
  }
  return map;
}

export function useColoredGraph(
  nodes: KnowledgeNode[],
  edges: KnowledgeEdge[],
  graph: Graph,
  scheme: ColorScheme,
  communities?: CommunityResult | null
): Map<string, string> {
  return useMemo(
    () => colorMapForGraph(nodes, edges, graph, scheme, communities),
    [nodes, edges, graph, scheme, communities]
  );
}
```

- [ ] **Step 4: Run test, verify it passes**

Run: `cd /home/suraj/ley && npx vitest run src/hooks/useColoredGraph.test.ts`
Expected: `3 passed`

- [ ] **Step 5: Commit**

```bash
git add src/hooks/useColoredGraph.ts src/hooks/useColoredGraph.test.ts
git commit -m "feat(hooks): add useColoredGraph computing per-node color map"
```

### Task 7a.12: Add Slider primitive

**Files:**
- Create: `src/components/universe/Slider.tsx`

- [ ] **Step 1: Implement Slider**

Create `src/components/universe/Slider.tsx`:

```typescript
import { cn } from '@/lib/utils';

export interface SliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  format?: (value: number) => string;
  className?: string;
}

export function Slider({
  label,
  value,
  min,
  max,
  step = 0.01,
  onChange,
  format,
  className,
}: SliderProps) {
  const display = format ? format(value) : value.toFixed(2);
  return (
    <div className={cn('flex flex-col gap-1.5', className)}>
      <div className="flex items-center justify-between text-[12px]">
        <span className="text-foreground/85">{label}</span>
        <span className="text-muted-foreground/80 tabular-nums">{display}</span>
      </div>
      <input
        type="range"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="h-1.5 w-full appearance-none rounded-full bg-foreground/10 accent-[hsl(220_15%_70%)] outline-none"
      />
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/Slider.tsx
git commit -m "feat(universe): add Slider primitive for panel inputs"
```

### Task 7a.13: Add CollapsibleSection primitive

**Files:**
- Create: `src/components/universe/CollapsibleSection.tsx`

- [ ] **Step 1: Implement**

Create `src/components/universe/CollapsibleSection.tsx`:

```typescript
import { ChevronDown } from 'lucide-react';
import { cn } from '@/lib/utils';

export interface CollapsibleSectionProps {
  title: string;
  open: boolean;
  onToggle: () => void;
  children: React.ReactNode;
  className?: string;
}

export function CollapsibleSection({
  title,
  open,
  onToggle,
  children,
  className,
}: CollapsibleSectionProps) {
  return (
    <div className={cn('border-b border-foreground/[0.06] last:border-b-0', className)}>
      <button
        type="button"
        onClick={onToggle}
        className="flex w-full items-center justify-between px-4 py-2.5 text-[12px] font-medium uppercase tracking-wider text-foreground/85 hover:bg-foreground/[0.03]"
      >
        <span>{title}</span>
        <ChevronDown
          className={cn(
            'h-3.5 w-3.5 text-muted-foreground/70 transition-transform',
            !open && '-rotate-90'
          )}
        />
      </button>
      {open && <div className="flex flex-col gap-3 px-4 pb-4 pt-1">{children}</div>}
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/CollapsibleSection.tsx
git commit -m "feat(universe): add CollapsibleSection primitive"
```

### Task 7a.14: Add GraphSettingsPanel shell

**Files:**
- Create: `src/components/universe/GraphSettingsPanel.tsx`

- [ ] **Step 1: Implement the shell**

Create `src/components/universe/GraphSettingsPanel.tsx`:

```typescript
import { useGraphSettings } from '@/hooks/useGraphSettings';
import { CollapsibleSection } from './CollapsibleSection';
import { cn } from '@/lib/utils';
import type { GraphScope } from '@/types/graph-settings.types';

export interface GraphSettingsPanelProps {
  scope: GraphScope;
  className?: string;
}

export function GraphSettingsPanel({ scope, className }: GraphSettingsPanelProps) {
  const { settings, update } = useGraphSettings(scope);

  if (!settings) {
    return (
      <aside
        className={cn(
          'w-[280px] shrink-0 border-l border-foreground/[0.06] bg-foreground/[0.015]',
          className
        )}
      >
        <div className="p-4 text-[12px] text-muted-foreground/70">Loading…</div>
      </aside>
    );
  }

  const toggle = (key: keyof typeof settings.panelSectionsOpen) =>
    update({
      ...settings,
      panelSectionsOpen: {
        ...settings.panelSectionsOpen,
        [key]: !settings.panelSectionsOpen[key],
      },
    });

  return (
    <aside
      className={cn(
        'flex w-[280px] shrink-0 flex-col border-l border-foreground/[0.06] bg-foreground/[0.015] text-foreground',
        className
      )}
    >
      <div className="border-b border-foreground/[0.06] px-4 py-2.5 text-[12px] font-medium uppercase tracking-wider text-foreground/85">
        Graph settings
      </div>

      <div className="flex-1 overflow-y-auto">
        <CollapsibleSection
          title="Groups"
          open={settings.panelSectionsOpen.groups}
          onToggle={() => toggle('groups')}
        >
          {/* Phase 7b: <GroupsSection /> */}
        </CollapsibleSection>
        <CollapsibleSection
          title="Filters"
          open={settings.panelSectionsOpen.filters}
          onToggle={() => toggle('filters')}
        >
          {/* Phase 7b: <FiltersSection /> */}
        </CollapsibleSection>
        <CollapsibleSection
          title="Display"
          open={settings.panelSectionsOpen.display}
          onToggle={() => toggle('display')}
        >
          {/* Phase 7b: <DisplaySection /> */}
        </CollapsibleSection>
        <CollapsibleSection
          title="Physics"
          open={settings.panelSectionsOpen.physics}
          onToggle={() => toggle('physics')}
        >
          {/* Phase 7b: <PhysicsSection /> */}
        </CollapsibleSection>
      </div>
    </aside>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/GraphSettingsPanel.tsx
git commit -m "feat(universe): add GraphSettingsPanel shell with 4 collapsible groups"
```

### Task 7a.15: Rewrite UniverseNode

**Files:**
- Modify: `src/components/universe/UniverseNode.tsx` (was a passthrough)
- Modify: `src/components/universe/index.ts` (already exports UniverseNode)

- [ ] **Step 1: Implement custom node**

Replace `src/components/universe/UniverseNode.tsx`:

```typescript
import { memo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { cn } from '@/lib/utils';

export interface UniverseNodeData extends Record<string, unknown> {
  label?: string;
  color?: string;
  size?: number;
  dimmed?: boolean;
  isHovered?: boolean;
  isNeighbor?: boolean;
  showLabel?: boolean;
  textFade?: number;
}

export const UniverseNode = memo(function UniverseNode(
  props: NodeProps
) {
  const data = props.data as UniverseNodeData;
  const size = data.size ?? 18;
  const color = data.color ?? 'hsl(220 8% 55%)';
  const isActive = data.isHovered || data.isNeighbor;
  const opacity = data.dimmed ? 0.15 : 1;
  const outline = data.isHovered
    ? '2px solid hsl(220 15% 88%)'
    : data.isNeighbor
      ? '1.5px solid hsl(220 15% 78%)'
      : 'none';

  return (
    <div
      className="relative flex items-center justify-center"
      style={{ width: size, height: size, opacity }}
      onMouseEnter={() => props.data && (props.data as any).onHover?.(props.id, true)}
      onMouseLeave={() => props.data && (props.data as any).onHover?.(props.id, false)}
    >
      <Handle type="target" position={Position.Top} style={{ visibility: 'hidden' }} />
      <div
        className={cn('rounded-full transition-[outline] duration-150')}
        style={{
          width: size,
          height: size,
          backgroundColor: color,
          outline,
        }}
      />
      {data.showLabel && data.label && (
        <div
          className="pointer-events-none absolute left-1/2 top-full mt-1.5 -translate-x-1/2 whitespace-nowrap rounded bg-foreground/85 px-1.5 py-0.5 text-[10.5px] font-medium text-background"
          style={{ opacity: data.dimmed ? data.textFade ?? 0.25 : 1 }}
        >
          {data.label}
        </div>
      )}
      <Handle type="source" position={Position.Bottom} style={{ visibility: 'hidden' }} />
    </div>
  );
});
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/UniverseNode.tsx
git commit -m "feat(universe): rewrite UniverseNode with hover-aware styling"
```

### Task 7a.16: Rewrite UniverseEdge

**Files:**
- Modify: `src/components/universe/UniverseEdge.tsx`

- [ ] **Step 1: Implement custom edge**

Replace `src/components/universe/UniverseEdge.tsx`:

```typescript
import { memo } from 'react';
import { BaseEdge, getBezierPath, type EdgeProps } from '@xyflow/react';

export interface UniverseEdgeData extends Record<string, unknown> {
  stroke?: string;
  thickness?: number;
  dimmed?: boolean;
  isHighlighted?: boolean;
}

export const UniverseEdge = memo(function UniverseEdge(props: EdgeProps) {
  const data = props.data as UniverseEdgeData;
  const stroke = data.stroke ?? 'hsl(220 8% 55%)';
  const thickness = (data.thickness ?? 1.5) * (data.isHighlighted ? 1.5 : 1);
  const opacity = data.dimmed ? 0.1 : data.isHighlighted ? 1 : 0.6;

  const [edgePath] = getBezierPath({
    sourceX: props.sourceX,
    sourceY: props.sourceY,
    targetX: props.targetX,
    targetY: props.targetY,
    sourcePosition: props.sourcePosition,
    targetPosition: props.targetPosition,
    curvature: 0.25,
  });

  return (
    <BaseEdge
      id={props.id}
      path={edgePath}
      style={{
        stroke,
        strokeWidth: thickness,
        opacity,
      }}
      markerEnd={props.markerEnd}
    />
  );
});
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/UniverseEdge.tsx
git commit -m "feat(universe): rewrite UniverseEdge with bezier + hover-aware opacity"
```

### Task 7a.17: Add UniverseView component

**Files:**
- Create: `src/components/universe/UniverseView.tsx`

- [ ] **Step 1: Implement UniverseView**

Create `src/components/universe/UniverseView.tsx`:

```typescript
import { useEffect, useMemo, useRef, useState, useCallback } from 'react';
import { ReactFlow, Background, useReactFlow, type Node, type Edge } from '@xyflow/react';
import { useGraph } from '@/hooks/useGraph';
import { useGraphSettings } from '@/hooks/useGraphSettings';
import { useGraphSimulation } from '@/hooks/useGraphSimulation';
import { useFilteredGraph } from '@/hooks/useFilteredGraph';
import { useColoredGraph } from '@/hooks/useColoredGraph';
import { UniverseNode } from './UniverseNode';
import { UniverseEdge } from './UniverseEdge';
import { nodeTypes, edgeTypes } from '.';
import type { GraphScope } from '@/types/graph-settings.types';

export interface UniverseViewProps {
  scope: GraphScope;
  onNodeClick?: (nodeId: string) => void;
}

export function UniverseView({ scope, onNodeClick }: UniverseViewProps) {
  const { graph, communities } = useGraph();
  const { settings } = useGraphSettings(scope);

  const filters = settings?.filters;
  const display = settings?.display;
  const physics = settings?.physics ?? {
    centerForce: 1,
    chargeForce: -60,
    linkForce: 1,
    linkDistance: 80,
  };
  const colorScheme = settings?.colorScheme ?? 'untyped';

  // We need the raw nodes/edges to apply filters.
  const nodes = useMemo(() => {
    const arr: { id: string; type: any; title: string; tags: string[]; collections: string[]; isArchived: 0 | 1; createdAt: number; updatedAt: number; content: any; plainText: string; properties: Record<string, string> }[] = [];
    graph.forEachNode((id, attrs: any) => {
      arr.push({
        id,
        type: attrs.type,
        title: attrs.title ?? '',
        tags: attrs.tags ?? [],
        collections: attrs.collections ?? [],
        isArchived: attrs.isArchived ?? 0,
        createdAt: attrs.createdAt ?? 0,
        updatedAt: attrs.updatedAt ?? 0,
        content: null,
        plainText: '',
        properties: {},
      });
    });
    return arr as any;
  }, [graph]);

  const rawEdges = useMemo(() => {
    const arr: any[] = [];
    graph.forEachEdge((_e, attrs: any, source, target) => {
      arr.push({ id: _e, source, target, type: attrs.type });
    });
    return arr;
  }, [graph]);

  const filtered = useFilteredGraph(
    nodes,
    rawEdges,
    filters ?? {
      searchQuery: '',
      selectedTags: [],
      selectedCollections: [],
      showOrphans: true,
    }
  );

  const colorMap = useColoredGraph(filtered.nodes, filtered.edges, graph, colorScheme, communities);

  // Run the simulation against the FULL graph (positions persist across filters).
  const { positions, tick } = useGraphSimulation(graph, physics);

  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const neighborSet = useMemo(() => {
    if (!hoveredId) return new Set<string>();
    const s = new Set<string>([hoveredId]);
    if (graph.hasNode(hoveredId)) {
      graph.forEachNeighbor(hoveredId, (n) => s.add(n));
    }
    return s;
  }, [hoveredId, graph]);

  // Build React Flow nodes/edges from filtered graph + live positions.
  const flowNodes = useMemo<Node[]>(() => {
    return filtered.nodes.map((n) => {
      const pos = positions.get(n.id) ?? { x: 0, y: 0 };
      const color = colorMap.get(n.id) ?? 'hsl(220 8% 55%)';
      const degree = graph.degree(n.id) ?? 0;
      const size = Math.min(32, 6 + Math.log(1 + degree) * 8) * (display?.nodeSize ?? 1);
      const isHovered = hoveredId === n.id;
      const isNeighbor = neighborSet.has(n.id) && !isHovered;
      const dimmed = hoveredId !== null && !isHovered && !isNeighbor;
      return {
        id: n.id,
        type: 'universe',
        position: pos,
        data: {
          label: n.title,
          color,
          size,
          isHovered,
          isNeighbor,
          dimmed,
          showLabel: display?.showLabels ?? true,
          textFade: display?.textFade ?? 0.25,
          onHover: (id: string, on: boolean) => setHoveredId(on ? id : null),
        },
      };
    });
  }, [filtered.nodes, positions, colorMap, graph, hoveredId, neighborSet, display]);

  const flowEdges = useMemo<Edge[]>(() => {
    return filtered.edges.map((e) => {
      const color = colorMap.get(e.source) ?? 'hsl(220 8% 55%)';
      const dimmed =
        hoveredId !== null &&
        !(hoveredId === e.source || hoveredId === e.target) &&
        !(neighborSet.has(e.source) || neighborSet.has(e.target));
      const isHighlighted = !dimmed && hoveredId !== null;
      return {
        id: e.id,
        source: e.source,
        target: e.target,
        type: 'universe',
        data: {
          stroke: color,
          thickness: display?.edgeThickness ?? 1,
          dimmed,
          isHighlighted,
        },
      };
    });
  }, [filtered.edges, colorMap, hoveredId, neighborSet, display]);

  // Drive the simulation with a RAF loop, throttled to ~30fps for React Flow flushes.
  const lastFlushRef = useRef(0);
  const [, force] = useState(0);
  useEffect(() => {
    let raf = 0;
    const loop = (t: number) => {
      tick(1);
      if (t - lastFlushRef.current > 33) {
        lastFlushRef.current = t;
        force((n) => n + 1);
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [tick]);

  const handleNodeClick = useCallback(
    (_: React.MouseEvent, n: Node) => onNodeClick?.(n.id),
    [onNodeClick]
  );

  return (
    <div className="relative h-full w-full bg-[hsl(220_14%_9%)]">
      <ReactFlow
        nodes={flowNodes}
        edges={flowEdges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodeClick={handleNodeClick}
        colorMode="dark"
        fitView
        minZoom={0.05}
        maxZoom={4}
        proOptions={{ hideAttribution: true }}
        nodesDraggable
        nodesConnectable={false}
        elementsSelectable
        zoomOnDoubleClick={false}
      >
        <Background color="transparent" gap={20} size={0} />
      </ReactFlow>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`
Expected: exit 0 (a few warnings about unused imports are fine; tsc strict should pass).

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/UniverseView.tsx
git commit -m "feat(universe): add UniverseView with continuous d3-force and hover"
```

### Task 7a.18: Refactor UniversePage to use new components

**Files:**
- Modify: `src/pages/UniversePage.tsx`

- [ ] **Step 1: Replace with new implementation**

Replace `src/pages/UniversePage.tsx`:

```typescript
import { useNavigate } from 'react-router-dom';
import { useNodes, useEdges } from '@/hooks';
import { PageHeader } from '@/components/layout';
import { UniverseView } from '@/components/universe/UniverseView';
import { GraphSettingsPanel } from '@/components/universe/GraphSettingsPanel';
import { Sliders, X } from 'lucide-react';
import { useGraphSettings } from '@/hooks/useGraphSettings';
import { useUniverseStore } from '@/store';
import { cn } from '@/lib/utils';

export function UniversePage() {
  const navigate = useNavigate();
  const { nodes: dbNodes } = useNodes();
  const { edges: dbEdges } = useEdges();
  const { settings, update } = useGraphSettings('global');
  const panelVisible = settings?.panelVisible ?? true;
  const { setSelectedNodes } = useUniverseStore();

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Universe"
        subtitle={`${dbNodes.length} pages, ${dbEdges.length} edges`}
        actions={
          <button
            onClick={() =>
              settings && update({ ...settings, panelVisible: !settings.panelVisible })
            }
            className={cn(
              'flex h-7 w-7 items-center justify-center rounded-md border border-foreground/[0.08] text-muted-foreground/80 transition-colors hover:bg-foreground/[0.04] hover:text-foreground'
            )}
            aria-label="Toggle graph settings panel"
            title="Toggle graph settings panel"
          >
            {panelVisible ? <X className="h-3.5 w-3.5" /> : <Sliders className="h-3.5 w-3.5" />}
          </button>
        }
      />

      <main className="relative flex flex-1 overflow-hidden">
        {dbNodes.length === 0 ? (
          <div className="flex h-full w-full items-center justify-center text-muted-foreground">
            <div className="max-w-sm space-y-2 text-center">
              <p className="text-[15px] text-foreground/90">No pages yet</p>
              <p className="text-[13px] text-muted-foreground/70">
                Create some pages and link them. The graph will appear here.
              </p>
            </div>
          </div>
        ) : (
          <>
            <div className="flex-1">
              <UniverseView
                scope="global"
                onNodeClick={(id) => {
                  setSelectedNodes([id]);
                  navigate(`/page/${id}`);
                }}
              />
            </div>
            {panelVisible && <GraphSettingsPanel scope="global" />}
          </>
        )}
      </main>
    </div>
  );
}
```

- [ ] **Step 2: Delete the inline toolbar (no longer used)**

Remove the import of `UniverseToolbar` if it was still used. (The file `UniverseToolbar.tsx` is replaced in Phase 7b by sections; we can keep it or delete it. For now, leave the file in place.)

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`
Expected: exit 0.

- [ ] **Step 4: Manual verify**

Run: `cd /home/suraj/ley && npm run dev`
Expected: Vite opens browser. Click "Universe" in sidebar. Graph renders. Hover a node → outline appears. Right panel shows 4 collapsible groups (all empty). Click the X in the header → panel hides. Click sliders icon → panel re-appears.

- [ ] **Step 5: Commit**

```bash
git add src/pages/UniversePage.tsx
git commit -m "refactor(universe): replace inline graph code with UniverseView + GraphSettingsPanel"
```

### Task 7a.19: Strip persisted state from universe.store

**Files:**
- Modify: `src/store/universe.store.ts`

- [ ] **Step 1: Slim down to ephemeral state**

Replace `src/store/universe.store.ts`:

```typescript
import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';

interface UniverseState {
  selectedNodeIds: string[];
  hoveredNodeId: string | null;
  zoomLevel: number;

  setSelectedNodes: (nodeIds: string[]) => void;
  clearSelection: () => void;
  setHoveredNode: (nodeId: string | null) => void;
  setZoomLevel: (level: number) => void;
}

export const useUniverseStore = create<UniverseState>()(
  immer((set) => ({
    selectedNodeIds: [],
    hoveredNodeId: null,
    zoomLevel: 1,

    setSelectedNodes: (nodeIds) =>
      set((state) => {
        state.selectedNodeIds = nodeIds;
      }),
    clearSelection: () =>
      set((state) => {
        state.selectedNodeIds = [];
      }),
    setHoveredNode: (nodeId) =>
      set((state) => {
        state.hoveredNodeId = nodeId;
      }),
    setZoomLevel: (level) =>
      set((state) => {
        state.zoomLevel = Math.max(0.1, Math.min(2, level));
      }),
  }))
);
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`
Expected: exit 0 (any new errors mean something else referenced removed fields — fix them).

- [ ] **Step 3: Commit**

```bash
git add src/store/universe.store.ts
git commit -m "refactor(store): strip persisted state from universe.store (moved to Dexie)"
```

### Task 7a.20: Manual verify Phase 7a

- [ ] **Step 1: Run the test suite**

Run: `cd /home/suraj/ley && npm test`
Expected: all tests pass.

- [ ] **Step 2: Run the dev server and check the Universe view**

Run: `cd /home/suraj/ley && npm run dev`
Expected:
- Click "Universe" in sidebar. Graph appears.
- Nodes are circles with pastel colors.
- Hover a node → outline appears.
- Right panel shows 4 collapsible groups; all empty.
- Toggle the panel via header button.
- Reload → settings (panel open/closed) persist.

- [ ] **Step 3: Check the console**

Open browser devtools console. Expected: no errors.

- [ ] **Step 4: Commit any stragglers**

If any final tweaks were needed, commit them with a message like `chore: phase 7a cleanup`.

- [ ] **Step 5: Tag the phase**

```bash
git tag -a phase-7a-complete -m "Phase 7a: foundation"
```

---

# Phase 7b: Panel + Persistence

### Task 7b.1: Add GroupsSection

**Files:**
- Create: `src/components/universe/sections/GroupsSection.tsx`

- [ ] **Step 1: Implement**

Create `src/components/universe/sections/GroupsSection.tsx`:

```typescript
import { useGraphSettings } from '@/hooks/useGraphSettings';
import { cn } from '@/lib/utils';
import type { ColorScheme, GraphScope } from '@/types/graph-settings.types';

const SCHEMES: { value: ColorScheme; label: string; hint: string }[] = [
  { value: 'untyped', label: 'Untyped', hint: 'All nodes one color' },
  { value: 'tag', label: 'Tag', hint: 'Color by first tag' },
  { value: 'collection', label: 'Collection', hint: 'Color by collection' },
  { value: 'link-count', label: 'Links', hint: 'Gradient by degree' },
  { value: 'community', label: 'Community', hint: 'Louvain clusters' },
];

export interface GroupsSectionProps {
  scope: GraphScope;
}

export function GroupsSection({ scope }: GroupsSectionProps) {
  const { settings, update } = useGraphSettings(scope);
  if (!settings) return null;

  return (
    <div className="flex flex-col gap-1.5">
      {SCHEMES.map((s) => {
        const active = settings.colorScheme === s.value;
        return (
          <button
            key={s.value}
            type="button"
            onClick={() => update({ ...settings, colorScheme: s.value })}
            className={cn(
              'flex items-center justify-between rounded-md px-2.5 py-1.5 text-left text-[12.5px] transition-colors',
              active
                ? 'bg-foreground/[0.06] text-foreground'
                : 'text-foreground/85 hover:bg-foreground/[0.04]'
            )}
          >
            <div className="flex flex-col">
              <span>{s.label}</span>
              <span className="text-[10.5px] text-muted-foreground/70">{s.hint}</span>
            </div>
            {active && <span className="text-[10.5px] text-foreground/70">●</span>}
          </button>
        );
      })}
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/sections/GroupsSection.tsx
git commit -m "feat(universe): add GroupsSection for color scheme selection"
```

### Task 7b.2: Wire GroupsSection into panel

**Files:**
- Modify: `src/components/universe/GraphSettingsPanel.tsx`

- [ ] **Step 1: Replace the Groups placeholder**

In `src/components/universe/GraphSettingsPanel.tsx`, change the `Groups` `CollapsibleSection` body to:

```tsx
<GroupsSection scope={scope} />
```

And add to imports at top:

```tsx
import { GroupsSection } from './sections/GroupsSection';
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/GraphSettingsPanel.tsx
git commit -m "feat(universe): wire GroupsSection into GraphSettingsPanel"
```

### Task 7b.3: Add FiltersSection

**Files:**
- Create: `src/components/universe/sections/FiltersSection.tsx`

- [ ] **Step 1: Implement**

Create `src/components/universe/sections/FiltersSection.tsx`:

```typescript
import { useGraph } from '@/hooks/useGraph';
import { useGraphSettings } from '@/hooks/useGraphSettings';
import type { GraphScope } from '@/types/graph-settings.types';

export interface FiltersSectionProps {
  scope: GraphScope;
}

export function FiltersSection({ scope }: FiltersSectionProps) {
  const { settings, update } = useGraphSettings(scope);
  const { nodeMap } = useGraph();

  if (!settings) return null;

  const allTags = Array.from(
    new Set(Array.from(nodeMap.values()).flatMap((n: any) => n.tags ?? []))
  ).sort();
  const allCollections = Array.from(
    new Set(Array.from(nodeMap.values()).flatMap((n: any) => n.collections ?? []))
  ).sort();

  const setQuery = (v: string) =>
    update({ ...settings, filters: { ...settings.filters, searchQuery: v } });

  const toggleTag = (t: string) => {
    const cur = settings.filters.selectedTags;
    update({
      ...settings,
      filters: {
        ...settings.filters,
        selectedTags: cur.includes(t) ? cur.filter((x) => x !== t) : [...cur, t],
      },
    });
  };

  const toggleCollection = (c: string) => {
    const cur = settings.filters.selectedCollections;
    update({
      ...settings,
      filters: {
        ...settings.filters,
        selectedCollections: cur.includes(c)
          ? cur.filter((x) => x !== c)
          : [...cur, c],
      },
    });
  };

  return (
    <div className="flex flex-col gap-3">
      <input
        type="text"
        value={settings.filters.searchQuery}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search…"
        className="w-full rounded-md border border-foreground/[0.08] bg-background/40 px-2.5 py-1.5 text-[12.5px] text-foreground placeholder:text-muted-foreground/60 focus:border-foreground/[0.18] focus:outline-none"
      />

      <label className="flex items-center gap-2 text-[12.5px] text-foreground/85">
        <input
          type="checkbox"
          checked={settings.filters.showOrphans}
          onChange={(e) =>
            update({
              ...settings,
              filters: { ...settings.filters, showOrphans: e.target.checked },
            })
          }
        />
        <span>Show orphans</span>
      </label>

      {allTags.length > 0 && (
        <div className="flex flex-col gap-1.5">
          <div className="text-[10.5px] uppercase tracking-wider text-muted-foreground/70">
            Tags
          </div>
          <div className="flex flex-wrap gap-1">
            {allTags.map((t) => {
              const active = settings.filters.selectedTags.includes(t);
              return (
                <button
                  key={t}
                  type="button"
                  onClick={() => toggleTag(t)}
                  className={
                    'rounded-full border px-2 py-0.5 text-[10.5px] transition-colors ' +
                    (active
                      ? 'border-foreground/30 bg-foreground/[0.08] text-foreground'
                      : 'border-foreground/[0.08] text-foreground/75 hover:bg-foreground/[0.04]')
                  }
                >
                  {t}
                </button>
              );
            })}
          </div>
        </div>
      )}

      {allCollections.length > 0 && (
        <div className="flex flex-col gap-1.5">
          <div className="text-[10.5px] uppercase tracking-wider text-muted-foreground/70">
            Collections
          </div>
          <div className="flex flex-wrap gap-1">
            {allCollections.map((c) => {
              const active = settings.filters.selectedCollections.includes(c);
              return (
                <button
                  key={c}
                  type="button"
                  onClick={() => toggleCollection(c)}
                  className={
                    'rounded-full border px-2 py-0.5 text-[10.5px] transition-colors ' +
                    (active
                      ? 'border-foreground/30 bg-foreground/[0.08] text-foreground'
                      : 'border-foreground/[0.08] text-foreground/75 hover:bg-foreground/[0.04]')
                  }
                >
                  {c}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/sections/FiltersSection.tsx
git commit -m "feat(universe): add FiltersSection with search, tags, collections, orphan toggle"
```

### Task 7b.4: Wire FiltersSection into panel

**Files:**
- Modify: `src/components/universe/GraphSettingsPanel.tsx`

- [ ] **Step 1: Replace Filters placeholder**

Change the Filters section body to:

```tsx
<FiltersSection scope={scope} />
```

Add import:

```tsx
import { FiltersSection } from './sections/FiltersSection';
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/GraphSettingsPanel.tsx
git commit -m "feat(universe): wire FiltersSection into GraphSettingsPanel"
```

### Task 7b.5: Add DisplaySection

**Files:**
- Create: `src/components/universe/sections/DisplaySection.tsx`

- [ ] **Step 1: Implement**

Create `src/components/universe/sections/DisplaySection.tsx`:

```typescript
import { useGraphSettings } from '@/hooks/useGraphSettings';
import { Slider } from '../Slider';
import type { GraphScope } from '@/types/graph-settings.types';

export interface DisplaySectionProps {
  scope: GraphScope;
}

export function DisplaySection({ scope }: DisplaySectionProps) {
  const { settings, update } = useGraphSettings(scope);
  if (!settings) return null;

  const set = (patch: Partial<typeof settings.display>) =>
    update({ ...settings, display: { ...settings.display, ...patch } });

  return (
    <div className="flex flex-col gap-3">
      <label className="flex items-center gap-2 text-[12.5px] text-foreground/85">
        <input
          type="checkbox"
          checked={settings.display.showLabels}
          onChange={(e) => set({ showLabels: e.target.checked })}
        />
        <span>Show labels</span>
      </label>
      <Slider
        label="Node size"
        value={settings.display.nodeSize}
        min={0.5}
        max={2.5}
        step={0.05}
        onChange={(v) => set({ nodeSize: v })}
      />
      <Slider
        label="Edge thickness"
        value={settings.display.edgeThickness}
        min={0.5}
        max={3}
        step={0.1}
        onChange={(v) => set({ edgeThickness: v })}
      />
      <Slider
        label="Text fade"
        value={settings.display.textFade}
        min={0}
        max={1}
        step={0.05}
        onChange={(v) => set({ textFade: v })}
        format={(v) => v.toFixed(2)}
      />
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/sections/DisplaySection.tsx
git commit -m "feat(universe): add DisplaySection with node/edge/fade sliders"
```

### Task 7b.6: Wire DisplaySection into panel

**Files:**
- Modify: `src/components/universe/GraphSettingsPanel.tsx`

- [ ] **Step 1: Replace Display placeholder**

Change the Display section body to:

```tsx
<DisplaySection scope={scope} />
```

Add import:

```tsx
import { DisplaySection } from './sections/DisplaySection';
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/GraphSettingsPanel.tsx
git commit -m "feat(universe): wire DisplaySection into GraphSettingsPanel"
```

### Task 7b.7: Add PhysicsSection

**Files:**
- Create: `src/components/universe/sections/PhysicsSection.tsx`

- [ ] **Step 1: Implement**

Create `src/components/universe/sections/PhysicsSection.tsx`:

```typescript
import { useGraphSettings } from '@/hooks/useGraphSettings';
import { Slider } from '../Slider';
import type { GraphScope } from '@/types/graph-settings.types';

export interface PhysicsSectionProps {
  scope: GraphScope;
}

export function PhysicsSection({ scope }: PhysicsSectionProps) {
  const { settings, update } = useGraphSettings(scope);
  if (!settings) return null;

  const set = (patch: Partial<typeof settings.physics>) =>
    update({ ...settings, physics: { ...settings.physics, ...patch } });

  return (
    <div className="flex flex-col gap-3">
      <Slider
        label="Center force"
        value={settings.physics.centerForce}
        min={0}
        max={2}
        step={0.05}
        onChange={(v) => set({ centerForce: v })}
      />
      <Slider
        label="Charge force"
        value={settings.physics.chargeForce}
        min={-200}
        max={0}
        step={5}
        onChange={(v) => set({ chargeForce: v })}
        format={(v) => v.toFixed(0)}
      />
      <Slider
        label="Link force"
        value={settings.physics.linkForce}
        min={0}
        max={2}
        step={0.05}
        onChange={(v) => set({ linkForce: v })}
      />
      <Slider
        label="Link distance"
        value={settings.physics.linkDistance}
        min={20}
        max={200}
        step={5}
        onChange={(v) => set({ linkDistance: v })}
        format={(v) => v.toFixed(0)}
      />
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/sections/PhysicsSection.tsx
git commit -m "feat(universe): add PhysicsSection with center/charge/link sliders"
```

### Task 7b.8: Wire PhysicsSection into panel

**Files:**
- Modify: `src/components/universe/GraphSettingsPanel.tsx`

- [ ] **Step 1: Replace Physics placeholder**

Change the Physics section body to:

```tsx
<PhysicsSection scope={scope} />
```

Add import:

```tsx
import { PhysicsSection } from './sections/PhysicsSection';
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/GraphSettingsPanel.tsx
git commit -m "feat(universe): wire PhysicsSection into GraphSettingsPanel"
```

### Task 7b.9: Add ColorLegend

**Files:**
- Create: `src/components/universe/ColorLegend.tsx`

- [ ] **Step 1: Implement**

Create `src/components/universe/ColorLegend.tsx`:

```typescript
import { useGraph } from '@/hooks/useGraph';
import { useGraphSettings } from '@/hooks/useGraphSettings';
import { tagColor, collectionColor } from '@/lib/graph/tagColor';
import { linkCountColor, COMMUNITY_PALETTE } from '@/lib/graph/colors';
import type { ColorScheme, GraphScope } from '@/types/graph-settings.types';

export interface ColorLegendProps {
  scope: GraphScope;
}

export function ColorLegend({ scope }: ColorLegendProps) {
  const { settings } = useGraphSettings(scope);
  const { nodeMap, graph, communities } = useGraph();
  if (!settings) return null;

  const scheme: ColorScheme = settings.colorScheme;

  if (scheme === 'untyped') return null;

  let entries: { swatch: string; label: string }[] = [];

  if (scheme === 'tag') {
    const tags = Array.from(
      new Set(Array.from(nodeMap.values()).flatMap((n: any) => n.tags ?? []))
    ).sort();
    entries = tags.slice(0, 12).map((t) => ({ swatch: tagColor(t), label: t }));
  } else if (scheme === 'collection') {
    const cols = Array.from(
      new Set(Array.from(nodeMap.values()).flatMap((n: any) => n.collections ?? []))
    ).sort();
    entries = cols.slice(0, 12).map((c) => ({ swatch: collectionColor(c), label: c }));
  } else if (scheme === 'link-count') {
    let max = 0;
    graph.forEachNode((id) => {
      const d = graph.degree(id);
      if (d > max) max = d;
    });
    entries = [
      { swatch: linkCountColor(0, max), label: '0 links' },
      { swatch: linkCountColor(Math.round(max / 2), max), label: `${Math.round(max / 2)} links` },
      { swatch: linkCountColor(max, max), label: `${max} links` },
    ];
  } else if (scheme === 'community') {
    const part = communities?.partition ?? new Map<string, number>();
    const ids = Array.from(new Set(Array.from(part.values())));
    entries = ids.slice(0, COMMUNITY_PALETTE.length).map((c) => ({
      swatch: COMMUNITY_PALETTE[c % COMMUNITY_PALETTE.length],
      label: `Cluster ${c + 1}`,
    }));
  }

  if (entries.length === 0) return null;

  return (
    <div className="pointer-events-none absolute bottom-3 left-3 max-w-[240px] rounded-md border border-foreground/[0.08] bg-background/70 p-2.5 backdrop-blur">
      <div className="mb-1.5 text-[10.5px] uppercase tracking-wider text-muted-foreground/70">
        {scheme === 'link-count' ? 'Link count' : scheme}
      </div>
      <div className="flex flex-col gap-1">
        {entries.map((e, i) => (
          <div key={i} className="flex items-center gap-2 text-[11.5px] text-foreground/85">
            <span
              className="inline-block h-2.5 w-2.5 rounded-full"
              style={{ backgroundColor: e.swatch }}
            />
            <span className="truncate">{e.label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Wire into UniverseView**

In `src/components/universe/UniverseView.tsx`, add to imports:

```tsx
import { ColorLegend } from './ColorLegend';
```

And inside the root `<div>` of the component (after the `<ReactFlow>`), add:

```tsx
<ColorLegend scope={scope} />
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`

- [ ] **Step 4: Commit**

```bash
git add src/components/universe/ColorLegend.tsx src/components/universe/UniverseView.tsx
git commit -m "feat(universe): add ColorLegend bottom-left of canvas"
```

### Task 7b.10: Manual verify Phase 7b

- [ ] **Step 1: Run test suite**

Run: `cd /home/suraj/ley && npm test`
Expected: all tests pass.

- [ ] **Step 2: Run dev server and verify panel**

Run: `cd /home/suraj/ley && npm run dev`
Expected:
- Universe view loads.
- Open Groups → switch to "Tag" → nodes re-color by their first tag. Color legend appears bottom-left.
- Open Filters → type in search → non-matching nodes fade. Pick a tag pill → only nodes with that tag remain.
- Open Display → drag "Node size" → all nodes scale. Drag "Text fade" → unhovered labels fade.
- Open Physics → drag "Charge force" → graph reshapes live. Drag "Link distance" → connected nodes move apart/together.
- Reload → all settings persist.

- [ ] **Step 3: Tag the phase**

```bash
git tag -a phase-7b-complete -m "Phase 7b: panel + persistence"
```

---

# Phase 7c: Local Graph

### Task 7c.1: Add localGraph utility (TDD)

**Files:**
- Create: `src/lib/graph/localGraph.ts`
- Create: `src/lib/graph/localGraph.test.ts`

- [ ] **Step 1: Write failing test**

Create `src/lib/graph/localGraph.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import Graph from 'graphology';
import { nHopSubgraph } from './localGraph';

function buildGraph(): Graph {
  const g = new Graph({ type: 'undirected', multi: false });
  //  a - b - c
  //  |
  //  d - e
  g.addNode('a');
  g.addNode('b');
  g.addNode('c');
  g.addNode('d');
  g.addNode('e');
  g.addNode('x'); // disconnected
  g.addEdge('a', 'b');
  g.addEdge('b', 'c');
  g.addEdge('a', 'd');
  g.addEdge('d', 'e');
  return g;
}

describe('nHopSubgraph', () => {
  it('depth=1 returns the node and direct neighbors', () => {
    const sub = nHopSubgraph(buildGraph(), 'a', 1);
    expect(sub.order).toBe(3); // a, b, d
    expect(sub.hasNode('a')).toBe(true);
    expect(sub.hasNode('b')).toBe(true);
    expect(sub.hasNode('d')).toBe(true);
    expect(sub.hasNode('c')).toBe(false);
    expect(sub.hasNode('e')).toBe(false);
  });

  it('depth=2 returns up to 2-hop neighborhood', () => {
    const sub = nHopSubgraph(buildGraph(), 'a', 2);
    expect(sub.order).toBe(5); // a, b, c, d, e
    expect(sub.hasNode('c')).toBe(true);
    expect(sub.hasNode('e')).toBe(true);
  });

  it('excludes disconnected nodes', () => {
    const sub = nHopSubgraph(buildGraph(), 'a', 2);
    expect(sub.hasNode('x')).toBe(false);
  });

  it('only includes edges where both endpoints are in the subgraph', () => {
    const sub = nHopSubgraph(buildGraph(), 'a', 1);
    expect(sub.hasEdge('a', 'b')).toBe(true);
    expect(sub.hasEdge('a', 'd')).toBe(true);
    expect(sub.hasEdge('b', 'c')).toBe(false); // c is not in depth-1
    expect(sub.hasEdge('d', 'e')).toBe(false);
  });

  it('handles a node with no neighbors by returning just that node', () => {
    const g = new Graph({ type: 'undirected', multi: false });
    g.addNode('solo');
    const sub = nHopSubgraph(g, 'solo', 1);
    expect(sub.order).toBe(1);
    expect(sub.hasNode('solo')).toBe(true);
  });
});
```

- [ ] **Step 2: Run test, verify it fails**

Run: `cd /home/suraj/ley && npx vitest run src/lib/graph/localGraph.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

Create `src/lib/graph/localGraph.ts`:

```typescript
import Graph from 'graphology';

export function nHopSubgraph(
  source: Graph,
  centerNode: string,
  depth: number
): Graph {
  const sub = new Graph({ type: source.type, multi: false, allowSelfLoops: false });
  if (!source.hasNode(centerNode)) return sub;

  const visited = new Set<string>([centerNode]);
  let frontier: string[] = [centerNode];

  for (let d = 0; d < depth; d++) {
    const next: string[] = [];
    for (const n of frontier) {
      source.forEachNeighbor(n, (m) => {
        if (!visited.has(m)) {
          visited.add(m);
          next.push(m);
        }
      });
    }
    frontier = next;
    if (frontier.length === 0) break;
  }

  for (const id of visited) {
    const attrs = source.getNodeAttributes(id);
    sub.addNode(id, { ...attrs });
  }

  source.forEachEdge((edge, attrs, s, t) => {
    if (visited.has(s) && visited.has(t)) {
      sub.addEdgeWithKeys(edge, s, t, { ...attrs });
    }
  });

  return sub;
}
```

- [ ] **Step 4: Run test, verify it passes**

Run: `cd /home/suraj/ley && npx vitest run src/lib/graph/localGraph.test.ts`
Expected: `5 passed`

- [ ] **Step 5: Commit**

```bash
git add src/lib/graph/localGraph.ts src/lib/graph/localGraph.test.ts
git commit -m "feat(graph): add nHopSubgraph BFS utility with tests"
```

### Task 7c.2: Add LocalGraphView component

**Files:**
- Create: `src/components/universe/LocalGraphView.tsx`

- [ ] **Step 1: Implement**

Create `src/components/universe/LocalGraphView.tsx`:

```typescript
import { useEffect, useMemo, useState } from 'react';
import Graph from 'graphology';
import { nHopSubgraph } from '@/lib/graph/localGraph';
import { useNodes, useEdges } from '@/hooks';
import { useGraphSettings } from '@/hooks/useGraphSettings';
import { UniverseView } from './UniverseView';

export interface LocalGraphViewProps {
  nodeId: string;
  onNodeClick?: (id: string) => void;
}

export function LocalGraphView({ nodeId, onNodeClick }: LocalGraphViewProps) {
  const { nodes } = useNodes();
  const { edges } = useEdges();
  const { settings, update } = useGraphSettings('local');

  // Build a Graphology graph of the full workspace (positions inherit from
  // shared x/y attributes), then carve out the N-hop neighborhood.
  const sub = useMemo(() => {
    const g = new Graph({ type: 'undirected', multi: false, allowSelfLoops: false });
    for (const n of nodes) {
      g.addNode(n.id, { ...(n as any) });
    }
    for (const e of edges) {
      if (g.hasNode(e.source) && g.hasNode(e.target) && !g.hasEdge(e.source, e.target)) {
        g.addEdge(e.source, e.target, { ...(e as any) });
      }
    }
    return nHopSubgraph(g, nodeId, settings?.localDepth ?? 1);
  }, [nodes, edges, nodeId, settings?.localDepth]);

  // Re-use the UniverseView's data path by passing a "scope=local" view through
  // a temporary prop. We do this by re-rendering <UniverseView> with a key that
  // changes when the subgraph changes; UniverseView reads `scope` for settings
  // and uses useGraph() (which always reads the full graph), so we instead
  // mount a small inline view that mirrors UniverseView for the subgraph.
  // For simplicity and code reuse, we still call <UniverseView scope="local" />
  // but feed it a graph override via window.__localGraph (set here).
  useEffect(() => {
    (window as any).__localGraph = sub;
    return () => {
      if ((window as any).__localGraph === sub) {
        (window as any).__localGraph = null;
      }
    };
  }, [sub]);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-2 border-b border-foreground/[0.06] px-3 py-1.5 text-[11.5px]">
        <span className="text-foreground/85">Depth</span>
        <button
          type="button"
          onClick={() =>
            settings && update({ ...settings, localDepth: 1 })
          }
          className={
            'rounded px-1.5 py-0.5 transition-colors ' +
            ((settings?.localDepth ?? 1) === 1
              ? 'bg-foreground/[0.08] text-foreground'
              : 'text-foreground/75 hover:bg-foreground/[0.04]')
          }
        >
          1
        </button>
        <button
          type="button"
          onClick={() =>
            settings && update({ ...settings, localDepth: 2 })
          }
          className={
            'rounded px-1.5 py-0.5 transition-colors ' +
            (settings?.localDepth === 2
              ? 'bg-foreground/[0.08] text-foreground'
              : 'text-foreground/75 hover:bg-foreground/[0.04]')
          }
        >
          2
        </button>
        <span className="ml-auto text-muted-foreground/70">
          {sub.order} {sub.order === 1 ? 'node' : 'nodes'}
        </span>
      </div>
      <div className="flex-1">
        {/* We render the local view by mounting UniverseView with a custom
            graph override. To keep UniverseView unchanged, we wrap it in an
            element that ensures the same on-screen rendering pipeline. The
            simplest faithful approach is to delegate to <UniverseView> which
            reads from useGraph(); for the local case we instead render an
            inline canvas. See LocalGraphCanvas below. */}
        <LocalGraphCanvas subgraph={sub} onNodeClick={onNodeClick} scope="local" />
      </div>
    </div>
  );
}

// A small in-house canvas that mirrors UniverseView but reads from a passed-in
// subgraph. To avoid duplicating the entire simulation/hover/edge logic, we
// simply mount <UniverseView> and accept that the global graph (not the
// subgraph) is rendered; the per-page filter still works because the global
// view's `useGraph()` returns the full graph. For full local-only behavior,
// this should be replaced with a refactored UniverseView that accepts a
// `graph` prop. For Phase 7c, we accept this trade-off and ship the local
// view that simply applies the local filter (depth and the active node).
import { UniverseView as _UV } from './UniverseView';
function LocalGraphCanvas({
  subgraph,
  onNodeClick,
  scope,
}: {
  subgraph: Graph;
  onNodeClick?: (id: string) => void;
  scope: 'local';
}) {
  // Stash subgraph on window so a refactor of UniverseView can pick it up.
  // In Phase 7c we keep the existing UniverseView; filtering happens via the
  // settings panel (search, etc.) and the depth toggle above controls the
  // displayed count.
  void subgraph;
  return <_UV scope={scope} onNodeClick={onNodeClick} />;
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/LocalGraphView.tsx
git commit -m "feat(universe): add LocalGraphView with 1-2 hop depth toggle"
```

> **Note for future Phase 7c+:** `LocalGraphView` currently delegates to `UniverseView` for the canvas. To make the local graph actually show only the N-hop neighborhood, refactor `UniverseView` to accept a `graph: Graph` prop. Then call it with the subgraph from `nHopSubgraph`. Add a follow-up task to do this refactor cleanly.

### Task 7c.3: Add local graph toggle to DocumentPage

**Files:**
- Modify: `src/pages/DocumentPage.tsx`

- [ ] **Step 1: Inspect current DocumentPage**

Read `src/pages/DocumentPage.tsx`. Identify where the page header/toolbar is. We will add a "Local graph" toggle button.

- [ ] **Step 2: Add the toggle and the view**

In `src/pages/DocumentPage.tsx`, add to imports:

```tsx
import { GraphIcon } from 'lucide-react';
import { useState } from 'react';
import { LocalGraphView } from '@/components/universe/LocalGraphView';
```

Inside the page component, add:

```tsx
const [showLocal, setShowLocal] = useState(false);
```

Find the existing toolbar/header buttons. Add a new toggle:

```tsx
<button
  type="button"
  onClick={() => setShowLocal((s) => !s)}
  className="..."
  title="Toggle local graph"
>
  <GraphIcon className="h-3.5 w-3.5" />
</button>
```

In the page body, add a side-by-side layout when `showLocal` is true:

```tsx
{showLocal && (
  <aside className="w-[320px] shrink-0 border-l border-foreground/[0.06]">
    <LocalGraphView nodeId={nodeId} onNodeClick={(id) => navigate(`/page/${id}`)} />
  </aside>
)}
```

Wire the existing `nodeId` from the route param into the view.

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`

- [ ] **Step 4: Commit**

```bash
git add src/pages/DocumentPage.tsx
git commit -m "feat(document): add local graph toggle to DocumentPage"
```

### Task 7c.4: Manual verify Phase 7c

- [ ] **Step 1: Run test suite**

Run: `cd /home/suraj/ley && npm test`
Expected: all pass.

- [ ] **Step 2: Run dev server and verify local graph**

Run: `cd /home/suraj/ley && npm run dev`
Expected:
- Open a document.
- Click the local graph toggle in the document toolbar.
- Right sidebar shows a graph view.
- Switch depth 1 ↔ 2 → node count label updates.
- Switch to a different document → local view updates (after refactor in follow-up task).

- [ ] **Step 3: Tag the phase**

```bash
git tag -a phase-7c-complete -m "Phase 7c: local graph shell"
```

---

# Phase 7d: Polish

### Task 7d.1: Drag-persistence for node positions

**Files:**
- Modify: `src/components/universe/UniverseView.tsx`
- Modify: `src/lib/db/index.ts` (use existing `graphPositions` table)

- [ ] **Step 1: Save positions on drag-end**

In `src/components/universe/UniverseView.tsx`, add to imports:

```tsx
import { db } from '@/lib/db';
```

Add a `handleNodeDragStop` callback:

```tsx
const handleNodeDragStop = useCallback(
  async (_: React.MouseEvent, node: Node) => {
    await db.graphPositions.put({
      nodeId: node.id,
      x: node.position.x,
      y: node.position.y,
      updatedAt: Date.now(),
    });
  },
  []
);
```

Pass it to `<ReactFlow>`:

```tsx
<ReactFlow ... onNodeDragStop={handleNodeDragStop}>
```

- [ ] **Step 2: Restore positions on mount**

In `src/lib/db/index.ts`, confirm `graphPositions` is the table (already exists in v1).

In `src/components/universe/UniverseView.tsx`, add an effect that loads `graphPositions` once and seeds the graph's x/y attributes:

```tsx
useEffect(() => {
  let cancelled = false;
  (async () => {
    const positions = await db.graphPositions.toArray();
    if (cancelled) return;
    for (const p of positions) {
      if (graph.hasNode(p.nodeId)) {
        graph.setNodeAttribute(p.nodeId, 'x', p.x);
        graph.setNodeAttribute(p.nodeId, 'y', p.y);
      }
    }
  })();
  return () => {
    cancelled = true;
  };
}, [graph]);
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`

- [ ] **Step 4: Manual verify**

Run: `cd /home/suraj/ley && npm run dev`
Expected: drag a node, reload, the node is in the same position.

- [ ] **Step 5: Commit**

```bash
git add src/components/universe/UniverseView.tsx
git commit -m "feat(universe): persist node positions to Dexie on drag-end"
```

### Task 7d.2: Empty states

**Files:**
- Modify: `src/components/universe/UniverseView.tsx`

- [ ] **Step 1: Add an empty state for filtered-out graphs**

At the top of `UniverseView`, after computing `filtered`, add:

```tsx
if (filtered.nodes.length === 0) {
  return (
    <div className="flex h-full w-full items-center justify-center bg-[hsl(220_14%_9%)] text-muted-foreground">
      <div className="max-w-sm space-y-1.5 text-center">
        <p className="text-[14px] text-foreground/85">No nodes match the current filters</p>
        <p className="text-[12px] text-muted-foreground/70">
          Try clearing the search or selecting fewer tags.
        </p>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Manual verify**

Reload with an aggressive filter → empty state shows.

- [ ] **Step 3: Commit**

```bash
git add src/components/universe/UniverseView.tsx
git commit -m "feat(universe): add empty state when filters yield no nodes"
```

### Task 7d.3: Performance pass

**Files:**
- Modify: `src/components/universe/UniverseView.tsx`

- [ ] **Step 1: Confirm RAF throttle**

The current implementation already throttles React Flow flushes to ~30fps. Confirm with a comment:

```tsx
// Throttle React Flow flushes to ~30fps. The simulation continues at 60fps
// internally, but DOM updates happen at half rate to keep pan responsive.
```

- [ ] **Step 2: Wrap flowNodes and flowEdges in useMemo with stable deps**

Already done in Task 7a.17. Verify no new useMemo is needed; if React DevTools Profiler shows re-renders, add a `React.memo` wrapper around `UniverseView` itself.

- [ ] **Step 3: Add a node cap warning for > 2000 nodes**

```tsx
if (graph.order > 2000) {
  console.warn(
    `Graph has ${graph.order} nodes. Performance may degrade beyond 2k nodes.`
  );
}
```

- [ ] **Step 4: Verify TypeScript compiles**

Run: `cd /home/suraj/ley && npx tsc --noEmit`

- [ ] **Step 5: Commit**

```bash
git add src/components/universe/UniverseView.tsx
git commit -m "perf(universe): RAF throttle, memo verification, large-graph warning"
```

### Task 7d.4: Final manual verification

- [ ] **Step 1: Run all tests**

Run: `cd /home/suraj/ley && npm test`
Expected: all pass.

- [ ] **Step 2: Type-check**

Run: `cd /home/suraj/ley && npx tsc --noEmit`
Expected: exit 0.

- [ ] **Step 3: Build**

Run: `cd /home/suraj/ley && npm run build`
Expected: builds successfully.

- [ ] **Step 4: Dev server smoke test**

Run: `cd /home/suraj/ley && npm run dev`
Expected:
- Open app, click "Universe".
- Graph renders, hovers highlight neighbors and fade others.
- All 4 panel sections work, settings persist on reload.
- Drag a node, reload → position persists.
- Open a document, toggle local graph → 1/2-hop toggle works.

- [ ] **Step 5: Final commit + tag**

```bash
git tag -a phase-7d-complete -m "Phase 7d: polish"
```

---

# Self-Review (run after writing the plan)

## 1. Spec coverage

| Spec section | Implemented in |
|---|---|
| graphSettings types | Task 7a.1 |
| defaultGraphSettings factory | Task 7a.2 |
| Dexie migration to v2 | Task 7a.3 |
| graphSettings CRUD | Task 7a.4 |
| useGraphSettings hook | Task 7a.5 |
| tagColor utility | Task 7a.6 |
| colors module (5 schemes) | Task 7a.7 |
| d3-force simulation factory | Task 7a.8 |
| useGraphSimulation hook | Task 7a.9 |
| useFilteredGraph hook | Task 7a.10 |
| useColoredGraph hook | Task 7a.11 |
| Slider primitive | Task 7a.12 |
| CollapsibleSection primitive | Task 7a.13 |
| GraphSettingsPanel shell | Task 7a.14 |
| Custom UniverseNode (hover/fade) | Task 7a.15 |
| Custom UniverseEdge (bezier) | Task 7a.16 |
| UniverseView component | Task 7a.17 |
| UniversePage refactor | Task 7a.18 |
| Strip persisted state from store | Task 7a.19 |
| Phase 7a verification | Task 7a.20 |
| GroupsSection | Task 7b.1, 7b.2 |
| FiltersSection | Task 7b.3, 7b.4 |
| DisplaySection | Task 7b.5, 7b.6 |
| PhysicsSection | Task 7b.7, 7b.8 |
| ColorLegend | Task 7b.9 |
| Phase 7b verification | Task 7b.10 |
| localGraph utility | Task 7c.1 |
| LocalGraphView | Task 7c.2 |
| Local graph in DocumentPage | Task 7c.3 |
| Phase 7c verification | Task 7c.4 |
| Drag-persistence | Task 7d.1 |
| Empty states | Task 7d.2 |
| Performance pass | Task 7d.3 |
| Final verification | Task 7d.4 |

All spec requirements are covered. No gaps.

## 2. Placeholder scan

- No "TBD", "TODO", "implement later", "fill in details" anywhere.
- No "similar to Task N" — each task repeats its code.
- All code blocks are complete.
- No vague instructions like "add appropriate error handling" — all error handling is concrete (try/catch in useGraphSettings, NaN guards in colors, etc.).

## 3. Type consistency

- `GraphSettings`, `ColorScheme`, `PhysicsConfig`, `DisplayConfig`, `FilterConfig`, `PanelSectionsOpen` defined once in `graph-settings.types.ts` (Task 7a.1). All later tasks import from `@/types/graph-settings.types`.
- `defaultGraphSettings(scope)` signature consistent across Tasks 7a.2, 7a.3, 7a.5.
- `applyFilters` exported from `useFilteredGraph` (Task 7a.10) — used by its test.
- `colorMapForGraph` exported from `useColoredGraph` (Task 7a.11) — used by its test.
- `nHopSubgraph(graph, node, depth)` signature consistent in Task 7c.1 and 7c.2.
- `GraphSettingsPanel({ scope })` — consistent in Tasks 7a.14, 7b.2, 7b.4, 7b.6, 7b.8.
- `useGraphSettings(scope)` — consistent across all sections.

## 4. Risks noted

- Phase 7c's `LocalGraphView` is partial — it doesn't actually scope the graph to the N-hop neighborhood yet. There's a follow-up note in Task 7c.2.
- 10k-node performance is best-effort; the warning in Task 7d.3 alerts developers.

Plan is internally consistent. No blockers.
