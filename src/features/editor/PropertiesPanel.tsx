import { useState } from 'react';
import { Plus, Tags, X } from 'lucide-react';
import { updatePageFrontmatter } from '@/core/vault/pages';

export function PropertiesPanel({ pageId, frontmatter }: { pageId: string; frontmatter: Record<string, unknown> }) {
  const [adding, setAdding] = useState(false);

  async function setProperty(key: string, value: unknown) {
    await updatePageFrontmatter(pageId, { ...frontmatter, [key]: value });
  }

  async function removeProperty(key: string) {
    const next = { ...frontmatter };
    delete next[key];
    await updatePageFrontmatter(pageId, next);
  }

  const entries = Object.entries(frontmatter).filter(([key]) => key !== 'title');
  return (
    <div className="mx-auto w-full max-w-[820px] px-10 pt-5">
      <div className="rounded-lg border border-border bg-surface-1/55">
        <div className="flex items-center gap-2 px-3 py-2 text-micro font-medium uppercase tracking-wide text-muted-foreground">
          <Tags size={12} /> Properties
          <button type="button" onClick={() => setAdding(true)} className="ml-auto rounded p-1 hover:bg-surface-3" aria-label="Add property"><Plus size={12} /></button>
        </div>
        {(entries.length > 0 || adding) && <div className="border-t border-border px-3 py-2">
          {entries.map(([key, value]) => <PropertyRow key={key} name={key} value={value} onSave={(next) => void setProperty(key, next)} onRemove={() => void removeProperty(key)} />)}
          {adding && <NewPropertyRow onCancel={() => setAdding(false)} onCreate={(key, value) => { setAdding(false); void setProperty(key, value); }} />}
        </div>}
      </div>
    </div>
  );
}

function PropertyRow({ name, value, onSave, onRemove }: { name: string; value: unknown; onSave: (value: unknown) => void; onRemove: () => void }) {
  const [draft, setDraft] = useState(formatValue(value));
  return <div className="group grid grid-cols-[140px_1fr_24px] items-center gap-2 rounded px-1 py-1 text-meta hover:bg-surface-2">
    <span className="truncate text-muted-foreground">{name}</span>
    <input value={draft} onChange={(event) => setDraft(event.target.value)} onBlur={() => onSave(parseValue(draft, value))} onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur(); }} className="min-w-0 bg-transparent text-foreground outline-none" />
    <button type="button" onClick={onRemove} className="rounded p-1 text-muted-foreground opacity-0 hover:bg-surface-3 hover:text-destructive group-hover:opacity-100" aria-label={`Remove ${name}`}><X size={11} /></button>
  </div>;
}

function NewPropertyRow({ onCancel, onCreate }: { onCancel: () => void; onCreate: (key: string, value: string) => void }) {
  const [key, setKey] = useState('');
  const [value, setValue] = useState('');
  function commit() { if (key.trim()) onCreate(key.trim(), value.trim()); }
  return <div className="grid grid-cols-[140px_1fr_24px] items-center gap-2 rounded bg-surface-2 px-1 py-1 text-meta">
    <input autoFocus value={key} onChange={(event) => setKey(event.target.value)} placeholder="property" className="min-w-0 bg-transparent text-muted-foreground outline-none" />
    <input value={value} onChange={(event) => setValue(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') commit(); if (event.key === 'Escape') onCancel(); }} placeholder="value" className="min-w-0 bg-transparent text-foreground outline-none" />
    <button type="button" onClick={onCancel} className="p-1 text-muted-foreground"><X size={11} /></button>
  </div>;
}

function formatValue(value: unknown): string {
  if (Array.isArray(value)) return value.join(', ');
  if (value === null || value === undefined) return '';
  if (typeof value === 'object') return JSON.stringify(value);
  return String(value);
}

function parseValue(value: string, original: unknown): unknown {
  if (Array.isArray(original)) return value.split(',').map((item) => item.trim()).filter(Boolean);
  if (typeof original === 'boolean') return value.toLowerCase() === 'true';
  if (typeof original === 'number' && Number.isFinite(Number(value))) return Number(value);
  return value;
}
