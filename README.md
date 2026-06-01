# Knowledge Universe

A local-first workspace where documents, tasks, projects, and concepts automatically become part of a navigable knowledge graph.

**This is not a Notion clone. This is not a chatbot. This is not an AI product.**

The core idea: **Workspace First. Universe Second.** You stay productive without ever opening the Universe view — the graph emerges naturally from the content you create.

## What it does

- Write documents in a rich editor with TipTap (headings, lists, code blocks, tables, task lists, highlights)
- Link pages together using `[[wiki-links]]` — they become typed edges in the graph
- Manage tasks with status, projects with members, and concepts with definitions
- See the **Universe** as a force-directed graph of everything you've created
- Search across all content with operators: `is:task`, `tag:research`, `related:React`
- Every page keeps a full revision history you can restore from

## Tech stack

- **Vite 5** + **React 18** + **TypeScript 5**
- **TipTap** — editor (JSON storage)
- **Dexie** — IndexedDB wrapper (local-first persistence)
- **Graphology** + **@xyflow/react** — graph engine + rendering
- **FlexSearch** — full-text search
- **Zustand** — UI state
- **Tailwind CSS** + **shadcn-style components**

All data lives in your browser's IndexedDB. No backend, no accounts, no telemetry.

## Running locally

Requires Node.js 18+.

```bash
npm install
npm run dev      # starts dev server on http://localhost:5173
npm run build    # production build
npm run preview  # preview the production build
```

No `.env` file is needed — the app is fully local.

## Project structure

```
src/
├── components/    # React components (editor, search, universe, layout, ui)
├── pages/         # Route components (Home, Document, Universe, Tasks, ...)
├── hooks/         # Custom hooks (useNodes, useEdges, useGraph, useCommands, ...)
├── lib/
│   ├── db/        # Dexie schema + CRUD (nodes, edges, collections, revisions)
│   ├── editor/    # TipTap helpers (extractText, parseWikiLinks)
│   ├── graph/     # Graphology layout, metrics, louvain, pathfinding
│   ├── search/    # FlexSearch index + operator parser
│   └── utils/     # cn, id, date, color helpers
├── store/         # Zustand stores
└── types/         # TypeScript types
```

## License

MIT (or your preferred license)
