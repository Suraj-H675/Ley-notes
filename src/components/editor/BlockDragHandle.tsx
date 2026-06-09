import {
  EditorView,
  Decoration,
  type DecorationSet,
  WidgetType,
  ViewPlugin,
  type ViewUpdate,
} from '@codemirror/view';
import { RangeSetBuilder, StateEffect, StateField } from '@codemirror/state';

// ─── Drag State ────────────────────────────────────────────────────────────────

interface DragState {
  dragging: boolean;
  blockFrom: number;
  blockTo: number;
  blockLine: number; // 1-based line number
}

const dragStateChanged = StateEffect.define<Partial<DragState>>();

// ─── Drop Indicator Decoration ────────────────────────────────────────────────

const dropIndicatorMark = Decoration.line({ class: 'cm-drag-drop-indicator' });

function buildDropIndicator(view: EditorView, targetLine: number): DecorationSet {
  if (targetLine < 1) return Decoration.none;
  const line = view.state.doc.line(targetLine);
  const builder = new RangeSetBuilder<Decoration>();
  builder.add(line.from, line.from, dropIndicatorMark);
  return builder.finish();
}

const dropIndicatorField = StateField.define<DecorationSet>({
  create() {
    return Decoration.none;
  },
  update(set, tr) {
    set = set.map(tr.changes);
    for (const effect of tr.effects) {
      if (effect.is(setDropIndicator)) {
        return effect.value;
      }
    }
    return set;
  },
  provide: (f) => EditorView.decorations.from(f),
});

const setDropIndicator = StateEffect.define<DecorationSet>();

// ─── Drag Handle Widget ───────────────────────────────────────────────────────

class BlockDragHandleWidget extends WidgetType {
  constructor(
    readonly blockFrom: number,
    readonly blockTo: number,
    readonly lineNumber: number,
    readonly view: EditorView
  ) {
    super();
  }

  toDOM(): HTMLElement {
    const handle = document.createElement('span');
    handle.className = 'cm-drag-handle';
    handle.setAttribute('contenteditable', 'false');
    handle.setAttribute('draggable', 'true');
    handle.innerHTML =
      '<svg width="6" height="14" viewBox="0 0 6 14" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="1" cy="1.5" r="1.2" fill="currentColor"/><circle cx="5" cy="1.5" r="1.2" fill="currentColor"/><circle cx="1" cy="5" r="1.2" fill="currentColor"/><circle cx="5" cy="5" r="1.2" fill="currentColor"/><circle cx="1" cy="8.5" r="1.2" fill="currentColor"/><circle cx="5" cy="8.5" r="1.2" fill="currentColor"/><circle cx="1" cy="12" r="1.2" fill="currentColor"/><circle cx="5" cy="12" r="1.2" fill="currentColor"/></svg>';

    // Track the hovered line when mouse enters the handle
    handle.addEventListener('mouseenter', () => {
      (this.view as any)._blockDragLine = this.lineNumber;
      (this.view as any)._blockDragging = true;
      this.view.dispatch({
        effects: dragStateChanged.of({
          blockFrom: this.blockFrom,
          blockTo: this.blockTo,
          blockLine: this.lineNumber,
        }),
      });
    });

    // ── Drag events ─────────────────────────────────────────────────────────
    handle.addEventListener('dragstart', (e) => {
      const dt = e.dataTransfer!;
      dt.effectAllowed = 'move';
      dt.setData('text/plain', ''); // Required for Firefox

      (this.view as any)._blockDragLine = this.lineNumber;
      (this.view as any)._blockDragging = true;

      this.view.dispatch({
        effects: dragStateChanged.of({
          dragging: true,
          blockFrom: this.blockFrom,
          blockTo: this.blockTo,
          blockLine: this.lineNumber,
        }),
      });

      // Set a transparent drag image so CodeMirror's cursor stays visible
      const ghost = document.createElement('div');
      ghost.style.cssText =
        'position:absolute;width:1px;height:1px;overflow:hidden;opacity:0;pointer-events:none;';
      document.body.appendChild(ghost);
      dt.setDragImage(ghost, 0, 0);
      requestAnimationFrame(() => ghost.remove());
    });

    handle.addEventListener('dragend', () => {
      this.view.dispatch({
        effects: [
          dragStateChanged.of({ dragging: false }),
          setDropIndicator.of(Decoration.none),
        ],
      });
      (this.view as any)._blockDragLine = undefined;
      (this.view as any)._blockDragging = false;
      (this.view as any)._dropTargetLine = undefined;
      (this.view as any)._lastDragOverY = undefined;
    });

    return handle;
  }

  eq(other: BlockDragHandleWidget): boolean {
    return (
      other.blockFrom === this.blockFrom &&
      other.blockTo === this.blockTo &&
      other.lineNumber === this.lineNumber
    );
  }

  ignoreEvents(): boolean {
    return false; // We handle drag events ourselves
  }
}

// ─── Block Decorations ────────────────────────────────────────────────────────

function buildBlockDecorations(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const doc = view.state.doc;

  const dragLine = (view as any)._blockDragLine as number | undefined;
  const isDragging = (view as any)._blockDragging as boolean;

  for (let i = 1; i <= doc.lines; i++) {
    const line = doc.line(i);

    // Skip the line that is currently being dragged (it keeps its handle)
    if (isDragging && dragLine === i) {
      continue;
    }

    builder.add(
      line.from,
      line.from,
      Decoration.widget({
        widget: new BlockDragHandleWidget(line.from, line.to, i, view),
        side: -1, // Place in the left gutter
        block: true,
      })
    );
  }

  return builder.finish();
}

const blockDecorationsPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = buildBlockDecorations(view);
    }
    update(update: ViewUpdate) {
      if (
        update.docChanged ||
        update.selectionSet ||
        update.transactions.some((t) =>
          t.effects.some((e) => e.is(dragStateChanged))
        )
      ) {
        this.decorations = buildBlockDecorations(update.view);
      }
    }
  },
  { decorations: (v) => v.decorations }
);

// ─── Block Reordering ─────────────────────────────────────────────────────────

function moveBlock(view: EditorView, fromLine: number, toLine: number): void {
  if (fromLine === toLine) return;

  const doc = view.state.doc;
  const from = doc.line(fromLine);
  const to = doc.line(toLine);

  // Range to delete: from line start to the start of the next line (or end of doc)
  const fromBlockStart = from.from;
  const afterFrom =
    fromLine < doc.lines ? doc.line(fromLine + 1).from : doc.length + 1;

  // Determine insertion point based on mouse Y relative to target line
  const targetLine = doc.line(toLine);
  const mouseY = (view as any)._lastDragOverY as number | undefined;

  let insertPos: number;
  if (mouseY !== undefined) {
    const targetTop = viewLineTop(view, targetLine);
    const targetBottom = targetTop + viewLineHeight(view, targetLine);
    const targetMid = (targetTop + targetBottom) / 2;
    insertPos = mouseY < targetMid ? targetLine.from : targetLine.from + targetLine.length + 1;
  } else {
    insertPos = toLine > fromLine ? to.from + to.length + 1 : from.from;
  }

  // Get the content being moved
  const movingText = doc.sliceString(fromBlockStart, afterFrom - 1);

  // Calculate adjusted insertion position accounting for deletion
  let adjustedInsert = insertPos;
  if (insertPos > fromBlockStart) {
    adjustedInsert -= movingText.length;
  }
  adjustedInsert = Math.max(0, Math.min(adjustedInsert, doc.length));

  // Dispatch deletion
  view.dispatch({
    changes: { from: fromBlockStart, to: afterFrom - 1, insert: '' },
  });

  // Re-fetch doc after first change (positions shifted)
  const doc2 = view.state.doc;
  const finalPos = Math.max(0, Math.min(adjustedInsert, doc2.length));

  // Dispatch insertion
  view.dispatch({
    changes: { from: finalPos, insert: movingText },
    selection: { anchor: finalPos },
  });
}

// ─── Helpers to approximate line top/height from coords ─────────────────────

function viewLineTop(view: EditorView, line: { number: number; from: number; to: number }): number {
  const viewRect = view.contentDOM.getBoundingClientRect();
  const top = view.coordsAtPos(line.from, -1);
  if (top) return top.top - viewRect.top;
  // Fallback: estimate from line number
  return (line.number - 1) * 22; // rough estimate
}

function viewLineHeight(view: EditorView, line: { number: number; from: number; to: number }): number {
  const from = view.coordsAtPos(line.from, -1);
  const to = view.coordsAtPos(line.to, 1);
  if (from && to) return to.bottom - from.top;
  return 22; // fallback
}

// ─── Main Extension ────────────────────────────────────────────────────────────

export const blockDragHandleExtension = [
  dropIndicatorField,
  blockDecorationsPlugin,
  EditorView.domEventHandlers({
    dragover(event, view) {
      if (!(view as any)._blockDragging) return false;
      (view as any)._lastDragOverY = event.clientY;

      // Determine target line from mouse Y
      const viewRect = view.contentDOM.getBoundingClientRect();
      const y = event.clientY - viewRect.top;
      const height = viewRect.height;
      const lineIndex = Math.floor((y / height) * view.state.doc.lines);
      const targetLine = Math.max(1, Math.min(lineIndex + 1, view.state.doc.lines));

      const indicator = buildDropIndicator(view, targetLine);
      view.dispatch({ effects: setDropIndicator.of(indicator) });
      (view as any)._dropTargetLine = targetLine;

      return true;
    },
    drop(event, view) {
      const fromLine = (view as any)._blockDragLine as number | undefined;
      const toLine = (view as any)._dropTargetLine as number | undefined;

      if (fromLine === undefined || toLine === undefined) return false;
      if (fromLine === toLine) return false;

      event.preventDefault();
      moveBlock(view, fromLine, toLine);

      // Reset drag state
      view.dispatch({
        effects: [
          dragStateChanged.of({ dragging: false }),
          setDropIndicator.of(Decoration.none),
        ],
      });
      (view as any)._blockDragging = false;
      (view as any)._blockDragLine = undefined;
      (view as any)._dropTargetLine = undefined;
      (view as any)._lastDragOverY = undefined;

      return true;
    },
    dragenter(event, view) {
      if (!(view as any)._blockDragging) return false;
      (view as any)._lastDragOverY = event.clientY;
      return true;
    },
  }),
  EditorView.baseTheme({
    '.cm-drag-handle': {
      display: 'inline-block',
      width: '14px',
      height: '100%',
      minHeight: '20px',
      cursor: 'grab',
      color: 'hsl(0 0% 55%)',
      opacity: '0',
      transition: 'opacity 0.15s ease',
      verticalAlign: 'middle',
      flexShrink: '0',
      userSelect: 'none',
      marginRight: '4px',
    },
    '.cm-line:hover .cm-drag-handle, .cm-drag-handle:hover': {
      opacity: '1',
    },
    '.cm-block-dragging': {
      opacity: '0.45',
      border: '1px dashed hsl(220 60% 60%)',
      borderRadius: '3px',
      background: 'hsl(220 30% 22%)',
    },
    '.cm-drag-drop-indicator': {
      display: 'block',
      height: '2px',
      background: 'hsl(220 70% 55%)',
      borderRadius: '1px',
      margin: '0 4px',
      boxShadow: '0 0 6px hsl(220 70% 55%)',
    },
    '&.cm-dragging .cm-drag-handle': {
      opacity: '1',
    },
  }),
];
