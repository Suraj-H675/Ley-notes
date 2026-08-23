import { useState } from 'react';
import { Plus, Tags, X } from 'lucide-react';
import { removePageProperty, updatePageProperty } from '@/core/vault/pages';
import {
  formatPropertyValue,
  parsePropertyValue,
  propertyValueError,
} from '@/core/parser/property-values';

export function PropertiesPanel({ pageId, frontmatter }: { pageId: string; frontmatter: Record<string, unknown> }) {
  const [adding, setAdding] = useState(false);

  async function setProperty(key: string, value: unknown) {
    await updatePageProperty(pageId, key, value);
  }

  async function removeProperty(key: string) {
    await removePageProperty(pageId, key);
  }

  const entries = Object.entries(frontmatter).filter(([key]) => key !== 'title');
  return (
    <div className="mx-auto w-full max-w-[820px] px-4 pt-4 sm:px-10 sm:pt-5">
      <div className="rounded-sm border border-border/70 bg-surface-1/45">
        <div className="flex items-center gap-2 border-b border-border/50 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.12em] text-muted-foreground">
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
  const [draft, setDraft] = useState(formatPropertyValue(value));
  const error = draft === formatPropertyValue(value) ? null : propertyValueError(draft, value);

  async function commit() {
    if (error) return;
    onSave(parsePropertyValue(draft, value));
  }

  return (
    <div className="group grid grid-cols-[140px_1fr_24px] items-center gap-2 rounded-sm px-1 py-1 text-meta hover:bg-white/[0.035]">
      <label htmlFor={`property-${name}`} className="truncate text-muted-foreground">{name}</label>
      <div className="min-w-0">
        <input
          id={`property-${name}`}
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={() => { if (!error) void commit(); }}
          onKeyDown={(event) => {
            if (event.key === 'Enter' && !error) event.currentTarget.blur();
          }}
          aria-invalid={Boolean(error)}
          title={error ?? `Edit ${name}`}
          className={`min-w-full bg-transparent text-foreground outline-none ${error ? 'text-destructive' : ''}`}
        />
        {error && <p className="mt-0.5 text-micro leading-4 text-destructive" role="alert">{error}</p>}
      </div>
      <button type="button" onClick={onRemove} className="rounded p-1 text-muted-foreground opacity-0 hover:bg-surface-3 hover:text-destructive group-hover:opacity-100" aria-label={`Remove ${name}`}><X size={11} /></button>
    </div>
  );
}

function NewPropertyRow({ onCancel, onCreate }: { onCancel: () => void; onCreate: (key: string, value: string) => void }) {
  const [key, setKey] = useState('');
  const [value, setValue] = useState('');
  const keyError = key.trim() === '' ? null : /[\p{C}<>:"\\|?*]/u.test(key.trim()) ? 'Use a portable property name.' : null;

  function commit() {
    if (!key.trim() || keyError) return;
    onCreate(key.trim(), value.trim());
  }

  return <div className="grid grid-cols-[140px_1fr_24px] items-center gap-2 rounded bg-surface-2 px-1 py-1 text-meta">
    <div className="min-w-0">
      <input autoFocus aria-label="Property name" value={key} onChange={(event) => setKey(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter' && !keyError) commit(); if (event.key === 'Escape') onCancel(); }} placeholder="property" aria-invalid={Boolean(keyError)} title={keyError ?? 'Property name'} className={`min-w-full bg-transparent outline-none ${keyError ? 'text-destructive' : 'text-muted-foreground'}`} />
      {keyError && <p className="mt-0.5 text-micro leading-4 text-destructive" role="alert">{keyError}</p>}
    </div>
    <input aria-label="Property value" value={value} onChange={(event) => setValue(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') commit(); if (event.key === 'Escape') onCancel(); }} placeholder="value" className="min-w-0 bg-transparent text-foreground outline-none" />
    <button type="button" onClick={onCancel} className="p-1 text-muted-foreground"><X size={11} /></button>
  </div>;
}
