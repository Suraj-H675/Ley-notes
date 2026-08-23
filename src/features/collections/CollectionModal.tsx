import { useMemo, useState } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import * as Dialog from '@radix-ui/react-dialog';
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  Check,
  ExternalLink,
  FileText,
  Hash,
  SlidersHorizontal,
  TableProperties,
  X,
} from 'lucide-react';
import { db } from '@/infrastructure/database/db';
import {
  buildCollectionRows,
  defaultCollectionColumns,
  discoverCollectionProperties,
  propertyKey,
  sortCollectionRows,
  type CollectionColumn,
  type CollectionSort,
} from '@/core/index/collection';
import {
  formatPropertyValue,
  parsePropertyValue,
  propertyValueError,
} from '@/core/parser/property-values';
import { updatePageProperty } from '@/core/vault/pages';
import {
  updateSavedSearchTable,
  type SavedSearchTable,
} from '@/core/vault/saved-searches';
import { useNavStore } from '@/shared/state/nav';

export interface CollectionRequest {
  query: string;
  title: string;
  savedSearchId?: string;
  table?: SavedSearchTable;
}

export function CollectionModal({ request, onClose }: { request: CollectionRequest; onClose: () => void }) {
  const liveRows = useLiveQuery(async () => {
    const [pages, tags] = await Promise.all([db.pages.toArray(), db.tags.toArray()]);
    return buildCollectionRows(pages, tags, request.query);
  }, [request.query]);
  const rows = useMemo(() => liveRows ?? [], [liveRows]);
  const [configuredColumns, setConfiguredColumns] = useState<CollectionColumn[] | null>(
    request.table ? request.table.columns : null,
  );
  const [sort, setSort] = useState<CollectionSort>(
    request.table?.sort ?? { column: 'modified', direction: 'desc' },
  );
  const [columnsOpen, setColumnsOpen] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const discovered = useMemo(() => discoverCollectionProperties(rows), [rows]);
  const columns = configuredColumns ?? defaultCollectionColumns(rows);
  const sortedRows = useMemo(() => sortCollectionRows(rows, sort), [rows, sort]);
  const openPage = useNavStore((state) => state.openPage);
  const openInSplit = useNavStore((state) => state.openInSplit);
  const pushRecent = useNavStore((state) => state.pushRecent);

  function persist(nextColumns: CollectionColumn[], nextSort: CollectionSort) {
    if (!request.savedSearchId) return;
    void updateSavedSearchTable(request.savedSearchId, { columns: nextColumns, sort: nextSort })
      .then(() => setStatus('View saved'))
      .catch((error) => setStatus(error instanceof Error ? error.message : String(error)));
  }

  function toggleColumn(column: CollectionColumn) {
    const next = columns.includes(column)
      ? columns.filter((candidate) => candidate !== column)
      : [...columns, column];
    const nextSort = sort.column !== 'title' && sort.column === column && !next.includes(column)
      ? { column: 'title' as const, direction: 'asc' as const }
      : sort;
    setConfiguredColumns(next);
    setSort(nextSort);
    persist(next, nextSort);
  }

  function changeSort(column: CollectionSort['column']) {
    const next: CollectionSort = sort.column === column
      ? { column, direction: sort.direction === 'asc' ? 'desc' : 'asc' }
      : { column, direction: column === 'modified' ? 'desc' : 'asc' };
    setSort(next);
    persist(columns, next);
  }

  function openNote(pageId: string, split = false) {
    void db.pages.get(pageId).then((page) => {
      if (!page || page.deletedAt !== null || page.missingFromDisk) return;
      if (split) openInSplit(pageId);
      else openPage(pageId);
      pushRecent(pageId);
      onClose();
    });
  }

  const propertyOptions = useMemo(() => {
    const byId = new Map(discovered.map((column) => [column.id, column]));
    for (const column of columns) {
      if (column.startsWith('property:')) {
        const property = column as `property:${string}`;
        if (!byId.has(property)) byId.set(property, { id: property, key: propertyKey(property), count: 0 });
      }
    }
    return [...byId.values()];
  }, [columns, discovered]);

  return (
    <Dialog.Root open onOpenChange={(open) => { if (!open) onClose(); }}>
      <Dialog.Portal>
        <Dialog.Overlay className="app-modal-overlay fixed inset-0 z-[80]" />
        <Dialog.Content aria-describedby={undefined} onEscapeKeyDown={(event) => { if (columnsOpen) { event.preventDefault(); setColumnsOpen(false); return; } if ((document.activeElement as HTMLElement | null)?.dataset.collectionCell === 'true') event.preventDefault(); }} className="app-modal-surface fixed left-1/2 top-1/2 z-[81] flex h-[calc(100vh-16px)] w-[calc(100vw-16px)] max-w-[1380px] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-md border outline-none sm:h-[min(88vh,900px)] sm:w-[min(94vw,1380px)]">
          <header className="flex shrink-0 flex-wrap items-center gap-3 border-b border-border px-3 py-3 sm:px-5">
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-secondary/20 bg-secondary/10 text-secondary">
              <TableProperties size={17} />
            </div>
            <div className="min-w-0 flex-1">
              <Dialog.Title className="truncate text-body font-semibold text-foreground">{request.title}</Dialog.Title>
              <p className="truncate font-mono text-micro text-muted-foreground">{request.query || 'All notes in this vault'}</p>
            </div>
            <div className="relative">
              <button type="button" onClick={() => setColumnsOpen((open) => !open)} aria-expanded={columnsOpen} className="flex h-8 items-center gap-1.5 rounded-md border border-border bg-background px-2.5 text-meta text-muted-foreground hover:bg-surface-2 hover:text-foreground"><SlidersHorizontal size={12} />Columns <span className="rounded bg-surface-3 px-1 text-micro">{columns.length}</span></button>
              {columnsOpen && <div className="absolute right-0 top-10 z-20 w-64 max-w-[calc(100vw-32px)] rounded-lg border border-border bg-surface-1 p-1.5 shadow-menu">
                <p className="px-2 pb-1 pt-0.5 text-micro font-medium uppercase tracking-wide text-subtle-foreground">Visible columns</p>
                <ColumnOption label="Tags" detail="Vault index" active={columns.includes('tags')} onToggle={() => toggleColumn('tags')} />
                <ColumnOption label="Path" detail="Markdown file" active={columns.includes('path')} onToggle={() => toggleColumn('path')} />
                <ColumnOption label="Modified" detail="Last save" active={columns.includes('modified')} onToggle={() => toggleColumn('modified')} />
                {propertyOptions.length > 0 && <div className="my-1 border-t border-border" />}
                <div className="max-h-64 overflow-y-auto">
                  {propertyOptions.map((column) => <ColumnOption key={column.id} label={column.key} detail={`${column.count} ${column.count === 1 ? 'note' : 'notes'}`} active={columns.includes(column.id)} onToggle={() => toggleColumn(column.id)} />)}
                </div>
              </div>}
            </div>
            <Dialog.Close className="rounded-md p-2 text-muted-foreground hover:bg-surface-3 hover:text-foreground" aria-label="Close collection"><X size={15} /></Dialog.Close>
          </header>

          <div className="flex min-h-0 flex-1 overflow-auto bg-background/45">
            {sortedRows.length === 0 ? <div className="m-auto max-w-sm px-6 text-center"><TableProperties size={28} className="mx-auto mb-3 text-subtle-foreground" /><h2 className="text-body font-medium text-foreground">No matching notes</h2><p className="mt-1 text-meta leading-relaxed text-muted-foreground">This collection is live. Notes will appear as soon as their Markdown, tags, or properties match the query.</p></div> : <table className="h-max min-w-full border-separate border-spacing-0 text-meta" aria-label={`${request.title} collection`}>
              <thead className="sticky top-0 z-10 bg-surface-1/95 backdrop-blur">
                <tr>
                  <SortableHeader label="Name" column="title" sort={sort} onSort={changeSort} sticky />
                  {columns.map((column) => <SortableHeader key={column} label={columnLabel(column)} column={column} sort={sort} onSort={changeSort} />)}
                  <th className="w-10 border-b border-border px-2 py-2"><span className="sr-only">Open actions</span></th>
                </tr>
              </thead>
              <tbody>
                {sortedRows.map(({ page, tags }) => <tr key={page.id} className="group hover:bg-surface-2/65">
                  <td className="sticky left-0 z-[1] min-w-56 max-w-72 border-b border-border bg-background px-3 py-2 group-hover:bg-surface-2">
                    <button type="button" onClick={() => openNote(page.id)} className="flex w-full items-center gap-2 text-left"><FileText size={13} className="shrink-0 text-secondary" /><span className="min-w-0"><span className="block truncate font-medium text-foreground">{page.title}</span><span className="block truncate font-mono text-micro text-subtle-foreground">{page.path}</span></span></button>
                  </td>
                  {columns.map((column) => <CollectionCell key={column} column={column} page={page} tags={tags} onStatus={setStatus} />)}
                  <td className="border-b border-border px-2 py-2"><button type="button" onClick={() => openNote(page.id, true)} className="rounded p-1.5 text-muted-foreground opacity-70 hover:bg-surface-3 hover:text-foreground sm:opacity-0 sm:group-hover:opacity-100 sm:focus:opacity-100" aria-label={`Open ${page.title} in split`} title="Open in split"><ExternalLink size={12} /></button></td>
                </tr>)}
              </tbody>
            </table>}
          </div>

          <footer className="flex shrink-0 items-center justify-between gap-3 border-t border-border px-3 py-2 text-micro text-muted-foreground sm:px-5">
            <span>{sortedRows.length} {sortedRows.length === 1 ? 'note' : 'notes'} · edits write directly to YAML properties</span>
            <span className="truncate text-right">{status ?? (request.savedSearchId ? 'Columns and sorting are saved with this query' : 'Save the query to retain this layout')}</span>
          </footer>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function SortableHeader({ label, column, sort, onSort, sticky = false }: { label: string; column: CollectionSort['column']; sort: CollectionSort; onSort: (column: CollectionSort['column']) => void; sticky?: boolean }) {
  const active = sort.column === column;
  return <th aria-sort={active ? (sort.direction === 'asc' ? 'ascending' : 'descending') : 'none'} className={`${sticky ? 'sticky left-0 z-20 bg-surface-1' : ''} min-w-40 border-b border-r border-border px-3 py-2 text-left font-medium text-muted-foreground`}><button type="button" onClick={() => onSort(column)} className="flex w-full items-center gap-1.5 hover:text-foreground"><span className="truncate">{label}</span>{active ? (sort.direction === 'asc' ? <ArrowUp size={11} /> : <ArrowDown size={11} />) : <ArrowUpDown size={11} className="opacity-40" />}</button></th>;
}

function CollectionCell({ column, page, tags, onStatus }: { column: CollectionColumn; page: import('@/infrastructure/database/schema').Page; tags: string[]; onStatus: (status: string | null) => void }) {
  if (column === 'tags') return <td className="min-w-52 border-b border-r border-border px-3 py-2"><div className="flex max-w-72 flex-wrap gap-1">{tags.length > 0 ? tags.map((tag) => <span key={tag} className="inline-flex items-center gap-0.5 rounded-full bg-secondary/10 px-1.5 py-0.5 text-micro text-secondary"><Hash size={9} />{tag}</span>) : <span className="text-subtle-foreground">—</span>}</div></td>;
  if (column === 'path') return <td className="min-w-56 max-w-80 border-b border-r border-border px-3 py-2 font-mono text-micro text-muted-foreground"><span className="block truncate">{page.path}</span></td>;
  if (column === 'modified') return <td className="min-w-44 border-b border-r border-border px-3 py-2 text-muted-foreground"><time dateTime={new Date(page.updatedAt).toISOString()}>{formatModified(page.updatedAt)}</time></td>;
  const key = propertyKey(column);
  const value = page.frontmatter[key];
  return <td className="min-w-44 border-b border-r border-border px-2 py-1"><PropertyCellEditor key={`${page.id}:${column}:${formatPropertyValue(value)}`} value={value} propertyKey={key} onStatus={onStatus} onSave={async (next) => { await updatePageProperty(page.id, key, next); }} /></td>;
}

function PropertyCellEditor({
  value,
  propertyKey,
  onStatus,
  onSave,
}: {
  value: unknown;
  propertyKey: string;
  onStatus: (status: string | null) => void;
  onSave: (value: unknown) => Promise<void>;
}) {
  const initial = formatPropertyValue(value);
  const [draft, setDraft] = useState(initial);
  const [busy, setBusy] = useState(false);
  const error = draft === initial ? null : propertyValueError(draft, value);
  async function commit() {
    if (draft === initial || busy || error) return;
    setBusy(true);
    onStatus(`Saving ${propertyKey}…`);
    try {
      await onSave(parsePropertyValue(draft, value));
      onStatus(`${propertyKey} saved`);
    } catch (cause) {
      onStatus(cause instanceof Error ? cause.message : String(cause));
      setDraft(initial);
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="relative">
      <input
        data-collection-cell="true"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={() => void commit()}
        onKeyDown={(event) => {
          if (event.key === 'Enter' && !error) event.currentTarget.blur();
          if (event.key === 'Escape') {
            event.preventDefault();
            event.stopPropagation();
            setDraft(initial);
          }
        }}
        placeholder="—"
        aria-invalid={Boolean(error)}
        aria-label={`Edit ${propertyKey} property`}
        title={error ?? `Edit ${propertyKey}`}
        className={`h-8 w-full rounded-md border border-transparent bg-transparent px-2 outline-none placeholder:text-subtle-foreground hover:border-border hover:bg-background focus:bg-background disabled:opacity-50 ${
          error
            ? 'text-destructive focus:border-destructive'
            : 'text-foreground focus:border-primary'
        }`}
        disabled={busy}
      />
      {busy && <span className="absolute right-2 top-1/2 h-1.5 w-1.5 -translate-y-1/2 animate-pulse rounded-full bg-secondary" />}
      {error && <p className="mt-0.5 text-micro leading-4 text-destructive" role="alert">{error}</p>}
    </div>
  );
}

function ColumnOption({ label, detail, active, onToggle }: { label: string; detail: string; active: boolean; onToggle: () => void }) {
  return <button type="button" onClick={onToggle} className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-meta text-muted-foreground hover:bg-surface-2 hover:text-foreground"><span className={`flex h-4 w-4 items-center justify-center rounded border ${active ? 'border-secondary bg-secondary text-secondary-foreground' : 'border-border'}`}>{active && <Check size={10} />}</span><span className="min-w-0 flex-1 truncate">{label}</span><span className="text-micro text-subtle-foreground">{detail}</span></button>;
}

function columnLabel(column: CollectionColumn): string {
  if (column === 'tags') return 'Tags';
  if (column === 'path') return 'Path';
  if (column === 'modified') return 'Modified';
  return propertyKey(column);
}

function formatModified(value: number): string {
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(value);
}
