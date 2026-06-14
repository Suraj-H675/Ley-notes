import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/lib/db';
import { extractPlainText } from '@/lib/markdown';

export interface HoverPreviewAnchor {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface HoverPreviewProps {
  /** Anchor of the hovered wikilink. Null hides the preview. */
  anchor: HoverPreviewAnchor | null;
  /** Title of the wikilink target. */
  title: string | null;
  /** Max preview length in characters. */
  maxLength?: number;
}

const DEFAULT_MAX_LENGTH = 200;
const HIDE_DELAY_MS = 150;
const SHOW_DELAY_MS = 250;

export function HoverPreview({
  anchor,
  title,
  maxLength = DEFAULT_MAX_LENGTH,
}: HoverPreviewProps) {
  const [delayedAnchor, setDelayedAnchor] = useState<HoverPreviewAnchor | null>(null);

  // Debounce show/hide so the preview doesn't flicker on quick mouse moves.
  useEffect(() => {
    if (!anchor || !title) {
      const t = setTimeout(() => setDelayedAnchor(null), HIDE_DELAY_MS);
      return () => clearTimeout(t);
    }
    const t = setTimeout(() => setDelayedAnchor(anchor), SHOW_DELAY_MS);
    return () => clearTimeout(t);
  }, [anchor, title]);

  const target = useLiveQuery(
    async () => {
      if (!title) return null;
      return (await db.nodes.where('title').equals(title).first()) ?? null;
    },
    [title],
    null
  );

  if (!delayedAnchor || !title) return null;

  // Find node from cache if live query hasn't resolved.
  const displayTitle = target?.title ?? title;
  const preview = target
    ? extractPlainText(target.content ?? '').slice(0, maxLength)
    : '';
  const isMissing = target === null; // live query has resolved and no match

  const style: React.CSSProperties = {
    position: 'fixed',
    top: delayedAnchor.y + delayedAnchor.height + 6,
    left: delayedAnchor.x,
    zIndex: 60,
    maxWidth: 360,
    background: 'hsl(220 14% 11%)',
    color: 'hsl(220 15% 90%)',
    border: '1px solid hsl(220 10% 22%)',
    borderRadius: 8,
    padding: '10px 12px',
    boxShadow: '0 8px 24px rgba(0,0,0,0.35)',
    fontSize: 12,
    lineHeight: 1.5,
    pointerEvents: 'none',
  };

  return createPortal(
    <div data-hover-preview="" style={style} role="tooltip">
      <div style={{ fontWeight: 600, marginBottom: 4 }}>{displayTitle}</div>
      {isMissing ? (
        <div style={{ color: 'hsl(220 10% 65%)', fontStyle: 'italic' }}>
          Note not found
        </div>
      ) : (
        <div style={{ color: 'hsl(220 12% 78%)' }}>{preview}</div>
      )}
    </div>,
    document.body
  );
}
