import { useEffect, useState } from "react";
import {
  AlertTriangle,
  Check,
  Database,
  EyeOff,
  FileSearch,
  HardDrive,
  LockKeyhole,
  RefreshCw,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import { Button } from "@/shared/components/Button";
import { cn } from "@/shared/lib/classnames";
import {
  eraseAgentProjectMemory,
  readAgentCaptureSettings,
  updateAgentCaptureMode,
} from "./api";
import type {
  AgentCaptureSettings,
  AgentMemoryDashboard,
  AgentProjectInspection,
  CaptureMode,
} from "./types";

const modes: Array<{
  id: CaptureMode;
  name: string;
  eyebrow: string;
  description: string;
  retention: string;
  tone: string;
}> = [
  {
    id: "minimal",
    name: "Minimal",
    eyebrow: "Maximum privacy",
    description:
      "Keeps deterministic structure, hashes, graph relationships, and structured session memory.",
    retention:
      "Project source and automatic prompt/response bodies are not retained.",
    tone: "bg-success/10 text-success",
  },
  {
    id: "structured",
    name: "Structured",
    eyebrow: "Recommended",
    description:
      "Keeps redacted source evidence with citations alongside bounded structured sessions.",
    retention:
      "Stores bounded, pattern-redacted prompts and responses—never a complete raw transcript automatically.",
    tone: "bg-primary/12 text-primary",
  },
  {
    id: "full-evidence",
    name: "Full Evidence",
    eyebrow: "Explicit permission",
    description:
      "Keeps bounded turn evidence and permits a future explicit adapter to submit versioned raw evidence.",
    retention:
      "Highest sensitivity and storage; Ley itself still does not scrape chats.",
    tone: "bg-warning/10 text-warning",
  },
];

export function CapturePrivacyPanel({
  projectPath,
  dashboard,
  onUpdated,
  onErased,
}: {
  projectPath: string;
  dashboard: AgentMemoryDashboard;
  onUpdated: (dashboard: AgentMemoryDashboard) => void;
  onErased: (inspection: AgentProjectInspection) => void;
}) {
  const [settings, setSettings] = useState<AgentCaptureSettings | null>(null);
  const [selected, setSelected] = useState<CaptureMode>(
    dashboard.overview.captureMode,
  );
  const [fullConsent, setFullConsent] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [eraseArmed, setEraseArmed] = useState(false);
  const [eraseConfirmation, setEraseConfirmation] = useState("");
  const [erasing, setErasing] = useState(false);
  const [eraseError, setEraseError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void readAgentCaptureSettings(projectPath)
      .then((next) => {
        if (!current) return;
        setSettings(next);
        setSelected(next.mode);
        setFullConsent(false);
        setError(null);
        setLoading(false);
      })
      .catch((cause) => {
        if (!current) return;
        setError(errorMessage(cause));
        setLoading(false);
      });
    return () => {
      current = false;
    };
  }, [projectPath]);

  async function applyMode() {
    if (!settings || selected === settings.mode || saving) return;
    setSaving(true);
    setError(null);
    try {
      const nextDashboard = await updateAgentCaptureMode(
        projectPath,
        settings.mode,
        selected,
        selected === "full-evidence" && fullConsent,
      );
      onUpdated(nextDashboard);
      const nextSettings = await readAgentCaptureSettings(projectPath);
      setSettings(nextSettings);
      setSelected(nextSettings.mode);
      setFullConsent(false);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setSaving(false);
    }
  }

  async function eraseMemory() {
    if (!settings || erasing || eraseConfirmation !== settings.projectName) {
      return;
    }
    setErasing(true);
    setEraseError(null);
    try {
      onErased(await eraseAgentProjectMemory(projectPath));
    } catch (cause) {
      setEraseError(errorMessage(cause));
      setErasing(false);
    }
  }

  if (loading && !settings) {
    return (
      <div
        className="flex min-h-80 items-center justify-center rounded-md border border-border bg-surface-1 text-meta text-muted-foreground"
        aria-label="Loading capture and privacy settings"
      >
        <RefreshCw
          size={15}
          className="mr-2 animate-spin motion-reduce:animate-none"
        />
        Reading local capture policy…
      </div>
    );
  }

  if (!settings) {
    return (
      <div className="rounded-md border border-destructive/25 bg-destructive/8 p-5">
        <p className="font-semibold">Capture policy is unavailable</p>
        <p className="mt-1 text-meta text-muted-foreground">{error}</p>
      </div>
    );
  }

  const changed = selected !== settings.mode;
  const fullNeedsConsent =
    selected === "full-evidence" &&
    !settings.storeRawTranscripts &&
    !fullConsent;

  return (
    <div className="space-y-7">
      <section className="relative overflow-hidden rounded-sm border border-border bg-surface-1 p-5 shadow-panel sm:p-7">
        <div className="relative max-w-3xl">
          <p className="text-micro font-semibold uppercase tracking-[0.14em] text-primary">
            Capture & privacy
          </p>
          <h2 className="mt-1 text-2xl font-semibold tracking-[-0.035em] sm:text-3xl">
            Decide what this project remembers
          </h2>
          <p className="mt-3 text-body leading-6 text-muted-foreground-strong">
            The policy lives in this project’s small{" "}
            <span className="font-mono text-meta">.ley/capture.json</span> file.
            Applying a change rebuilds the cited snapshot inside{" "}
            {dashboard.binding.vaultName}; it never uploads project data.
          </p>
        </div>
      </section>

      {error && (
        <div
          role="alert"
          className="flex items-start gap-3 rounded-md border border-destructive/25 bg-destructive/8 p-4 text-meta"
        >
          <AlertTriangle
            size={17}
            className="mt-0.5 shrink-0 text-destructive"
            aria-hidden="true"
          />
          <div>
            <p className="font-semibold">Could not update capture</p>
            <p className="mt-0.5 text-muted-foreground">{error}</p>
          </div>
        </div>
      )}

      <section aria-labelledby="evidence-mode-title">
        <div className="mb-3">
          <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
            Project evidence
          </p>
          <h3
            id="evidence-mode-title"
            className="mt-1 text-lg font-semibold tracking-tight"
          >
            Evidence retention mode
          </h3>
        </div>
        <div
          role="radiogroup"
          aria-labelledby="evidence-mode-title"
          className="grid gap-3 lg:grid-cols-3"
        >
          {modes.map((mode) => {
            const active = selected === mode.id;
            return (
              <label
                key={mode.id}
                className={cn(
                  "relative min-w-0 cursor-pointer rounded-md border bg-surface-1 p-4 text-left outline-none transition has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-primary",
                  active
                    ? "border-primary/55 shadow-[0_0_0_1px_hsl(var(--primary)/0.12)]"
                    : "border-border hover:border-border-strong",
                  saving && "cursor-wait opacity-70",
                )}
              >
                <input
                  type="radio"
                  name="capture-mode"
                  value={mode.id}
                  checked={active}
                  disabled={saving}
                  onChange={() => {
                    setSelected(mode.id);
                    if (mode.id !== "full-evidence") setFullConsent(false);
                  }}
                  className="sr-only"
                />
                <div className="flex items-start justify-between gap-3">
                  <span
                    className={cn(
                      "rounded-sm px-2 py-0.5 text-micro font-semibold",
                      mode.tone,
                    )}
                  >
                    {mode.eyebrow}
                  </span>
                  <span
                    className={cn(
                      "flex size-5 items-center justify-center rounded-full border",
                      active
                        ? "border-primary bg-primary text-primary-foreground"
                        : "border-border",
                    )}
                  >
                    {active && <Check size={11} aria-hidden="true" />}
                  </span>
                </div>
                <h4 className="mt-4 text-body font-semibold">{mode.name}</h4>
                <p className="mt-1 text-meta leading-5 text-muted-foreground">
                  {mode.description}
                </p>
                <p className="mt-3 text-micro leading-4 text-muted-foreground-strong">
                  {mode.retention}
                </p>
              </label>
            );
          })}
        </div>

        {selected === "full-evidence" && !settings.storeRawTranscripts && (
          <label className="mt-3 flex cursor-pointer items-start gap-3 rounded-md border border-warning/30 bg-warning/8 p-4">
            <input
              type="checkbox"
              checked={fullConsent}
              disabled={saving}
              onChange={(event) => setFullConsent(event.target.checked)}
              className="mt-0.5 size-4 shrink-0 accent-primary"
            />
            <span>
              <span className="block text-meta font-semibold">
                Permit Full Evidence for this project
              </span>
              <span className="mt-1 block text-meta leading-5 text-muted-foreground">
                I understand a transcript-capable adapter may store raw host
                evidence locally. Ley does not capture complete chats by itself,
                and cloud agents may receive only context I ask them to
                retrieve.
              </span>
            </span>
          </label>
        )}

        <div className="mt-4 flex flex-col gap-3 rounded-md border border-border bg-surface-1 p-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <p className="text-meta font-semibold">
              {changed
                ? `Change ${humanize(settings.mode)} to ${humanize(selected)}`
                : `${humanize(settings.mode)} is active`}
            </p>
            <p className="mt-0.5 text-micro text-muted-foreground">
              Changes are project-specific and trigger a fresh deterministic
              capture.
            </p>
          </div>
          <Button
            variant="primary"
            disabled={!changed || fullNeedsConsent || saving}
            onClick={() => void applyMode()}
          >
            {saving ? (
              <RefreshCw
                size={14}
                className="animate-spin motion-reduce:animate-none"
              />
            ) : (
              <ShieldCheck size={14} />
            )}
            {saving ? "Applying & recapturing" : "Apply & recapture"}
          </Button>
        </div>
      </section>

      <section aria-labelledby="capture-boundary-title">
        <div className="mb-3">
          <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
            Inspect before capture
          </p>
          <h3
            id="capture-boundary-title"
            className="mt-1 text-lg font-semibold tracking-tight"
          >
            Current approved boundary
          </h3>
        </div>
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <BoundaryMetric
            icon={FileSearch}
            label="Eligible files"
            value={settings.eligibleFiles.toLocaleString()}
            detail={formatBytes(settings.eligibleBytes)}
          />
          <BoundaryMetric
            icon={Database}
            label="Retained source"
            value={dashboard.overview.retainedSourceFiles.toLocaleString()}
            detail={`${dashboard.overview.files.toLocaleString()} captured records`}
          />
          <BoundaryMetric
            icon={HardDrive}
            label="Capture ceiling"
            value={formatBytes(settings.maxTotalBytes)}
            detail={`${formatBytes(settings.maxFileBytes)} per file`}
          />
          <BoundaryMetric
            icon={EyeOff}
            label="Excluded by limits"
            value={(
              settings.skippedOversized + settings.skippedTotalLimit
            ).toLocaleString()}
            detail={`${settings.skippedSymlinks} symlinks skipped`}
          />
        </div>

        <div className="mt-3 grid gap-3 lg:grid-cols-2">
          <div className="rounded-md border border-border bg-surface-1 p-4">
            <p className="text-meta font-semibold">Approved roots</p>
            <div className="mt-2 flex flex-wrap gap-2">
              {settings.approvedRoots.map((root) => (
                <span
                  key={root}
                  className="rounded-md bg-surface-3 px-2 py-1 font-mono text-micro"
                >
                  {root}
                </span>
              ))}
            </div>
            <p className="mt-3 text-micro text-muted-foreground">
              Git ignore: {settings.respectGitignore ? "respected" : "not used"}{" "}
              · .leyignore: {settings.ignoreFilePresent ? "present" : "missing"}
            </p>
          </div>
          <div className="rounded-md border border-border bg-surface-1 p-4">
            <div className="flex items-start gap-3">
              <LockKeyhole
                size={16}
                className="mt-0.5 shrink-0 text-primary"
                aria-hidden="true"
              />
              <div>
                <p className="text-meta font-semibold">
                  Local consent boundary
                </p>
                <p className="mt-1 text-meta leading-5 text-muted-foreground">
                  {settings.privacyNotice}
                </p>
                <p
                  className="mt-2 truncate font-mono text-micro text-subtle-foreground"
                  title={settings.captureFingerprint}
                >
                  {settings.captureFingerprint}
                </p>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section aria-labelledby="erase-memory-title">
        <div className="overflow-hidden rounded-sm border border-destructive/25 bg-surface-1 shadow-panel">
          <div className="flex flex-col gap-4 p-5 sm:flex-row sm:items-center sm:justify-between sm:p-6">
            <div className="max-w-2xl">
              <p className="text-micro font-semibold uppercase tracking-[0.14em] text-destructive">
                Local data control
              </p>
              <h3
                id="erase-memory-title"
                className="mt-1 text-lg font-semibold tracking-tight"
              >
                Erase this project’s Agent Memory
              </h3>
              <p className="mt-1 text-meta leading-5 text-muted-foreground">
                Permanently removes captured artifacts, graph history, sessions,
                and lessons from this vault. Your project files, notes,{" "}
                <span className="font-mono text-micro">.ley</span> settings, and
                private vault binding stay in place.
              </p>
            </div>
            <Button
              variant={eraseArmed ? "destructive" : "outline"}
              disabled={saving || erasing}
              onClick={() => {
                setEraseArmed((current) => !current);
                setEraseConfirmation("");
                setEraseError(null);
              }}
            >
              <Trash2 size={14} aria-hidden="true" />
              {eraseArmed ? "Cancel erasure" : "Erase memory…"}
            </Button>
          </div>

          {eraseArmed && (
            <div className="border-t border-destructive/20 bg-destructive/[0.035] p-5 sm:p-6">
              <div className="max-w-2xl">
                <p className="text-body font-semibold">
                  This cannot be undone by Ley
                </p>
                <p className="mt-1 text-meta leading-5 text-muted-foreground">
                  Stop connected Codex, Claude Code, and Gemini sessions first.
                  Existing backups, filesystem snapshots, or copies outside this
                  vault are not securely wiped.
                </p>
                <label className="mt-4 block">
                  <span className="text-micro font-semibold text-muted-foreground-strong">
                    Type{" "}
                    <span className="font-mono text-foreground">
                      {settings.projectName}
                    </span>{" "}
                    to confirm
                  </span>
                  <input
                    type="text"
                    value={eraseConfirmation}
                    disabled={erasing}
                    autoComplete="off"
                    spellCheck={false}
                    onChange={(event) =>
                      setEraseConfirmation(event.target.value)
                    }
                    className="mt-2 h-10 w-full rounded-md border border-border bg-background px-3 text-meta outline-none transition-[border-color,box-shadow] focus:border-destructive/60 focus:ring-2 focus:ring-destructive/15 disabled:opacity-60 sm:max-w-md"
                  />
                </label>
                {eraseError && (
                  <p role="alert" className="mt-3 text-meta text-destructive">
                    {eraseError}
                  </p>
                )}
                <div className="mt-4 flex flex-wrap items-center gap-3">
                  <Button
                    variant="destructive"
                    disabled={
                      erasing || eraseConfirmation !== settings.projectName
                    }
                    onClick={() => void eraseMemory()}
                  >
                    {erasing ? (
                      <RefreshCw
                        size={14}
                        className="animate-spin motion-reduce:animate-none"
                      />
                    ) : (
                      <Trash2 size={14} aria-hidden="true" />
                    )}
                    {erasing
                      ? "Erasing local memory"
                      : "Permanently erase memory"}
                  </Button>
                  <span className="text-micro text-muted-foreground">
                    Recapture remains available afterward.
                  </span>
                </div>
              </div>
            </div>
          )}
        </div>
      </section>
    </div>
  );
}

function BoundaryMetric({
  icon: Icon,
  label,
  value,
  detail,
}: {
  icon: typeof Database;
  label: string;
  value: string;
  detail: string;
}) {
  return (
    <div className="rounded-md border border-border bg-surface-1 p-4 shadow-panel">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-micro text-muted-foreground">{label}</p>
          <p className="mt-1 text-xl font-semibold tabular-nums">{value}</p>
        </div>
        <span className="flex size-8 items-center justify-center rounded-md bg-primary/10 text-primary">
          <Icon size={15} aria-hidden="true" />
        </span>
      </div>
      <p className="mt-2 text-micro text-muted-foreground">{detail}</p>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1_024) return `${bytes} B`;
  if (bytes < 1_048_576) return `${(bytes / 1_024).toFixed(1)} KB`;
  if (bytes < 1_073_741_824) return `${(bytes / 1_048_576).toFixed(1)} MB`;
  return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
}

function humanize(value: string): string {
  return value
    .replaceAll("-", " ")
    .replace(/\b\w/g, (character) => character.toUpperCase());
}

function errorMessage(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  return "Ley could not update this project’s local capture policy.";
}
