/**
 * GraphControls — sidebar with all the knobs from Obsidian's graph view:
 *   Filters: search, tag filter, orphans toggle
 *   Display: color mode, link thickness, arrows toggle, text fade threshold
 *   Physics: center force, repel force, link force, link distance, iterations
 *   Local graph: enable + depth slider
 *
 * Pure controlled component — receives values + setters from the parent.
 */

import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/data/db';
import { Input } from '@/ui/Input';
import { Button } from '@/ui/Button';
import { DEFAULT_PHYSICS, type PhysicsSettings } from '@/core/graph/layout';
import type { ColorMode } from './GraphCanvas';
import { cn } from '@/lib/classnames';

export interface GraphControlsState {
  query: string;
  setQuery: (v: string) => void;
  tagFilter: string | null;
  setTagFilter: (v: string | null) => void;
  orphansOnly: boolean;
  setOrphansOnly: (v: boolean) => void;
  colorMode: ColorMode;
  setColorMode: (v: ColorMode) => void;
  linkThickness: number;
  setLinkThickness: (v: number) => void;
  showArrows: boolean;
  setShowArrows: (v: boolean) => void;
  textFadeThreshold: number;
  setTextFadeThreshold: (v: number) => void;
  physics: PhysicsSettings;
  setPhysics: (v: PhysicsSettings) => void;
  localEnabled: boolean;
  setLocalEnabled: (v: boolean) => void;
  localDepth: number;
  setLocalDepth: (v: number) => void;
}

export interface GraphControlsProps {
  state: GraphControlsState;
  stats: { pageCount: number; edgeCount: number; communityCount: number };
}

export function GraphControls({ state, stats }: GraphControlsProps) {
  return (
    <div className="flex h-full flex-col gap-4 overflow-y-auto px-3 py-4">
      <Section title="Filters">
        <Field label="Search">
          <Input
            type="text"
            value={state.query}
            onChange={(e) => state.setQuery(e.target.value)}
            placeholder="Filter nodes by title…"
          />
        </Field>
        <Field label="Tag">
          <TagFilterSelect
            value={state.tagFilter}
            onChange={state.setTagFilter}
          />
        </Field>
        <Checkbox
          checked={state.orphansOnly}
          onChange={state.setOrphansOnly}
          label="Orphans only (unlinked)"
        />
      </Section>

      <Section title="Display">
        <Field label="Color by">
          <div className="grid grid-cols-2 gap-1">
            {(['community', 'tag', 'folder', 'degree'] as const).map((m) => (
              <button
                key={m}
                type="button"
                onClick={() => state.setColorMode(m)}
                className={cn(
                  'rounded-sm border px-2 py-1 text-meta capitalize',
                  state.colorMode === m
                    ? 'border-primary bg-primary/15 text-foreground'
                    : 'border-border bg-surface-1 text-muted-foreground-strong hover:bg-surface-2',
                )}
              >
                {m}
              </button>
            ))}
          </div>
        </Field>
        <Slider
          label="Node size"
          min={0.5}
          max={3}
          step={0.1}
          value={1}
          display={`${1}×`}
          onChange={() => {
            /* node size is derived from degree; this is reserved for a multiplier in v2 */
          }}
          disabled
        />
        <Slider
          label="Link thickness"
          min={0.5}
          max={4}
          step={0.5}
          value={state.linkThickness}
          display={`${state.linkThickness}px`}
          onChange={state.setLinkThickness}
        />
        <Slider
          label="Text fade threshold"
          min={0}
          max={50}
          step={1}
          value={state.textFadeThreshold}
          display={`${state.textFadeThreshold}°`}
          onChange={state.setTextFadeThreshold}
        />
        <Checkbox
          checked={state.showArrows}
          onChange={state.setShowArrows}
          label="Show arrows"
        />
      </Section>

      <Section title="Physics">
        <Slider
          label="Center force"
          min={0}
          max={1}
          step={0.01}
          value={state.physics.centerForce}
          display={state.physics.centerForce.toFixed(2)}
          onChange={(v) => state.setPhysics({ ...state.physics, centerForce: v })}
        />
        <Slider
          label="Repel force"
          min={0}
          max={1000}
          step={10}
          value={state.physics.repelForce}
          display={String(state.physics.repelForce)}
          onChange={(v) => state.setPhysics({ ...state.physics, repelForce: v })}
        />
        <Slider
          label="Link force"
          min={0}
          max={5}
          step={0.1}
          value={state.physics.linkForce}
          display={state.physics.linkForce.toFixed(1)}
          onChange={(v) => state.setPhysics({ ...state.physics, linkForce: v })}
        />
        <Slider
          label="Iterations"
          min={50}
          max={800}
          step={50}
          value={state.physics.iterations}
          display={String(state.physics.iterations)}
          onChange={(v) => state.setPhysics({ ...state.physics, iterations: v })}
        />
        <Button
          size="sm"
          variant="ghost"
          onClick={() => state.setPhysics(DEFAULT_PHYSICS)}
        >
          Reset physics
        </Button>
      </Section>

      <Section title="Local graph">
        <Checkbox
          checked={state.localEnabled}
          onChange={state.setLocalEnabled}
          label="Center on active page"
          disabled={!stats.pageCount}
        />
        <Slider
          label="Depth"
          min={1}
          max={5}
          step={1}
          value={state.localDepth}
          display={`${state.localDepth} ${state.localDepth === 1 ? 'hop' : 'hops'}`}
          onChange={(v) => state.setLocalDepth(v)}
        />
      </Section>

      <Section title="Stats">
        <div className="flex flex-col gap-1 text-meta text-muted-foreground-strong">
          <div className="flex justify-between">
            <span>Pages</span>
            <span className="font-mono">{stats.pageCount}</span>
          </div>
          <div className="flex justify-between">
            <span>Edges</span>
            <span className="font-mono">{stats.edgeCount}</span>
          </div>
          <div className="flex justify-between">
            <span>Communities</span>
            <span className="font-mono">{stats.communityCount}</span>
          </div>
        </div>
      </Section>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-2">
      <div className="text-meta font-semibold uppercase tracking-wide text-muted-foreground">
        {title}
      </div>
      <div className="flex flex-col gap-2">{children}</div>
    </section>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1 text-meta">
      <span className="text-muted-foreground">{label}</span>
      {children}
    </label>
  );
}

function Checkbox({
  checked,
  onChange,
  label,
  disabled,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <label
      className={cn(
        'flex items-center gap-2 text-meta select-none',
        disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer text-foreground',
      )}
    >
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        disabled={disabled}
        className="h-3.5 w-3.5 rounded-sm border border-border bg-surface-1 accent-primary"
      />
      <span>{label}</span>
    </label>
  );
}

function Slider({
  label,
  min,
  max,
  step,
  value,
  display,
  onChange,
  disabled,
}: {
  label: string;
  min: number;
  max: number;
  step: number;
  value: number;
  display: string;
  onChange: (v: number) => void;
  disabled?: boolean;
}) {
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center justify-between text-meta">
        <span className="text-muted-foreground">{label}</span>
        <span className="font-mono text-muted-foreground-strong">{display}</span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(Number(e.target.value))}
        className="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-surface-3 accent-primary disabled:cursor-not-allowed disabled:opacity-50"
      />
    </div>
  );
}

function TagFilterSelect({
  value,
  onChange,
}: {
  value: string | null;
  onChange: (v: string | null) => void;
}) {
  const tags = useLiveQuery(async () => {
    const rows = await db.tags.toArray();
    const set = new Set(rows.map((r) => r.tag));
    return [...set].sort();
  }, []);

  return (
    <select
      value={value ?? ''}
      onChange={(e) => onChange(e.target.value || null)}
      className="h-8 rounded-md border border-border bg-surface-1 px-2 text-meta text-foreground focus:border-primary focus:outline-none"
    >
      <option value="">All tags</option>
      {(tags ?? []).map((t) => (
        <option key={t} value={t}>
          #{t}
        </option>
      ))}
    </select>
  );
}