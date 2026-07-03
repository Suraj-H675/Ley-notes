# Archived: TipTap Extensions Experiment

These files were a partial attempt to wire up TipTap as the editor. They were never connected to `MarkdownEditor.tsx` (which uses CodeMirror 6) and were dormant at the time of the v2 block-level refactor.

**Why archived, not deleted:**

- Preserves the implementation work in case TipTap migration is ever pursued.
- Documents the alternative approach considered for v2.
- Cheap to keep (no runtime cost — files are not imported by anything).

**Why we kept CodeMirror instead:**

- `MarkdownEditor.tsx` is the actual user-facing editor and uses CodeMirror 6.
- It has all features (autocomplete via `[[`, find-bar, status bar, hover preview, slash commands via text decorations, callout/task widgets) wired up and tested.
- Bringing TipTap online would require rewriting these extensions from scratch (2-3 weeks of side work).
- The block-level refactor (v2 plan) is editor-agnostic; both can achieve it.

**Files:**

- `WikiLink.extension.ts` — ProseMirror mark + suggestion plugin for `[[Title]]`.
- `SlashCommand.extension.ts` — ProseMirror suggestion plugin for `/` commands.
- `index.ts` — barrel that re-exports both.

If you ever want to revive these:

1. Move them back to `src/components/editor/extensions/`.
2. Update import paths (relative paths to `../suggestion-renderer`, etc., may have changed).
3. Wire into `MarkdownEditor.tsx` (or a new `BlockEditor.tsx`) using `@tiptap/react`'s `useEditor` hook.
4. Delete this README.