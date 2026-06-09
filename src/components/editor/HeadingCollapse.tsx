import { gutter, GutterMarker } from '@codemirror/view';
import { EditorView, type BlockInfo } from '@codemirror/view';
import { foldEffect } from '@codemirror/language';

// Gutter marker for collapsible heading indicators
class HeadingCollapseMarker extends GutterMarker {
  constructor(
    readonly level: number,
    readonly foldTo: number
  ) {
    super();
  }

  toDOM(): HTMLElement {
    const marker = document.createElement('span');
    marker.className = 'cm-collapse-marker';
    marker.textContent = '▶';
    marker.style.cssText = `
      cursor: pointer;
      color: hsl(220 10% 55%);
      font-size: 10px;
      line-height: 1;
      user-select: none;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 14px;
      height: 14px;
    `;
    return marker;
  }

  eq(other: HeadingCollapseMarker): boolean {
    return other.level === this.level && other.foldTo === this.foldTo;
  }

  destroy() {}
}

// Helper to find the fold range for a heading at a given line number
function findFoldRange(
  doc: { lines: number; line(n: number): { from: number; text: string } },
  lineNumber: number,
  level: number
): number {
  let foldTo = doc.lines;
  for (let l = lineNumber + 1; l <= doc.lines; l++) {
    const nextLine = doc.line(l);
    const nextMatch = nextLine.text.match(/^(#{1,3})\s/);
    if (nextMatch) {
      const nextLevel = nextMatch[1].length;
      if (nextLevel <= level) {
        foldTo = nextLine.from;
        break;
      }
    }
  }
  return foldTo;
}

// Build the heading collapse gutter extension
export const headingCollapseGutter = gutter({
  class: 'cm-heading-collapse-gutter',
  lineMarker(view: EditorView, line: BlockInfo) {
    const lineText = view.state.doc.lineAt(line.from).text;
    const match = lineText.match(/^(#{1,3})\s/);
    if (!match) return null;

    const level = match[1].length;
    const lineNumber = view.state.doc.lineAt(line.from).number;
    const foldTo = findFoldRange(view.state.doc, lineNumber, level);

    return new HeadingCollapseMarker(level, foldTo);
  },
  initialSpacer() {
    return new HeadingCollapseMarker(0, 0);
  },
  domEventHandlers: {
    mousedown(_view: EditorView, line: BlockInfo, event: Event) {
      const target = event.target as HTMLElement | null;
      if (!target?.classList.contains('cm-collapse-marker')) return false;

      const lineText = _view.state.doc.lineAt(line.from).text;
      const match = lineText.match(/^(#{1,3})\s/);
      if (!match) return false;

      const level = match[1].length;
      const lineNumber = _view.state.doc.lineAt(line.from).number;
      const foldTo = findFoldRange(_view.state.doc, lineNumber, level);

      // Dispatch fold effect to collapse/expand
      _view.dispatch({
        effects: foldEffect.of({ from: line.from, to: foldTo }),
      });
      return true;
    },
  },
});