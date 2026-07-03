# CLAUDE.md

Always use Context7 whenever generating code, setup instructions, API usage, or library documentation.
## Rules

- Always use Context7 for library documentation.
- Use Tavily when information may be outdated.
- Never guess APIs.
- Verify package versions before writing code.
- Prefer official documentation over blogs.
- Search GitHub issues if an error appears unusual.

## Project

Knowledge Universe

A local-first workspace where documents, tasks, projects, and concepts automatically become part of a navigable knowledge graph.

This is NOT an AI product.

This is NOT a chatbot.

This is NOT a Notion clone.

The core idea is:

Workspace First.
Universe Second.

Users should be productive without ever opening the Universe view.

The Universe exists to reveal relationships between knowledge, not replace traditional note-taking.

---

## Read Before Coding

Before making any changes, read:

1. implementation_plan.md
2. PROJECT_VISION.md

Do not invent architecture that contradicts these files.

---

## Core Principles

### 1. Local First

All data should work locally without a backend.

Primary storage:

* IndexedDB
* Dexie

Never introduce a server dependency unless explicitly requested.

---

### 2. Graph Derived From Knowledge

The graph should emerge naturally from content.

Preferred:

* Wiki links
* References
* Relationships

Avoid requiring users to manually build graphs.

---

### 3. Simplicity Before Features

Do not add new features when the existing architecture already solves the problem.

Prefer:

* Simple
* Predictable
* Maintainable

over

* Clever
* Complex
* Abstract

---

### 4. Performance Matters

Assume future workspaces may contain:

* 10,000+ nodes
* 50,000+ relationships

Avoid unnecessary rerenders.

Avoid O(n²) operations where possible.

Memoize expensive calculations.

---

### 5. No Premature Abstractions

Do not create:

* generic frameworks
* plugin systems
* complex factories
* over-engineered abstractions

unless there is an actual need.

Build the simplest solution that supports the current requirements.

---

## Architecture Rules

### Editor

Use CodeMirror 6.

The TipTap extensions under `archive/tiptap-experiments/` are dormant — they were never wired up. Bringing TipTap online is a 2-3 week side quest and not worth it.

Editor content is stored as **Markdown string** (not JSON). This makes notes round-trippable to Obsidian, diffable in git, and inspectable with `cat`.

### Database

Use Dexie.

Use useLiveQuery for reactive reads.

Avoid duplicating database state in React state.

IndexedDB is the source of truth.

---

### Graph

Use:

* Graphology
* React Flow

React Flow is a rendering layer.

Graphology is the graph engine.

Do not mix responsibilities.

---

### State Management

Use Zustand.

Use local component state when appropriate.

Do not put everything into Zustand.

---

## Code Quality Rules

### Prefer

* small files
* explicit naming
* typed interfaces
* predictable state flow

### Avoid

* any
* deeply nested conditionals
* giant components
* unnecessary custom hooks

---

## When Adding Features

Ask:

1. Does this align with the product vision?
2. Does this already exist elsewhere in the architecture?
3. Is this Phase 1 or Phase 2?
4. Does this increase complexity unnecessarily?

If unsure, choose the simpler implementation.

---

## Decision Making

When architecture choices are required:

 Follow implementation_plan.md
 Preserve local-first architecture
 Preserve workspace-first UX

Never introduce architectural changes without documenting them in DECISIONS.md.

---

## Current Goal

Focus on building the product described in implementation_plan.md.

Do not redesign the application.

Do not add AI features.

Do not add cloud features.

Do not add collaboration features.

Do not add backend infrastructure.

Build the planned system first.
