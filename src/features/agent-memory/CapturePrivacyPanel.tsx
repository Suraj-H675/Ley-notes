import { useEffect, useState } from "react";
import {
  AlertTriangle,
  Check,
  Database,
  Download,
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
  chooseAgentContinuityExportParent,
  exportAgentProjectContinuity,
  readAgentCaptureSettings,
  readAgentEgressPolicy,
  updateAgentEgressPolicy,
  updateAgentCaptureMode,
} from "./api";
import type {
  AgentCaptureSettings,
  AgentContinuityExport,
  AgentEgressPolicy,
  AgentMemoryDashboard,
  AgentProjectInspection,
  CaptureMode,
  ProjectAgentEgressPolicy,
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
      "Keeps bounded artifact metadata, hashes, and structured session memory.",
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
      "Keeps bounded turn evidence and retains supported captured image originals for multimodal evidence.",
    retention:
      "Highest sensitivity and storage; images are stored as original evidence without OCR or visual redaction. Ley does not scrape complete chat transcripts. Installed host integrations may still add small Ley session/checkpoint guidance to the agent context; that is separate from this retention setting.",
    tone: "bg-warning/10 text-warning",
  },
];

const egressPolicies: Array<{
  id: AgentEgressPolicy;
  name: string;
  description: string;
}> = [
  {
    id: "agent-ok",
    name: "Agent OK",
    description:
      "Ley context may be supplied to the configured agent target when all other authority and privacy checks allow it.",
  },
  {
    id: "local-model-only",
    name: "Local model only",
    description:
      "Ley context is allowed only when the integration explicitly starts Ley with a local egress target.",
  },
  {
    id: "confirm-per-use",
    name: "Confirm per use",
    description:
      "Currently blocks every Ley agent-context request. Ley does not yet show a trustworthy per-use confirmation prompt.",
  },
  {
    id: "never-send",
    name: "Never send",
    description:
      "Ley never supplies this project’s retained context to an agent target.",
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
  const [egressPolicy, setEgressPolicy] =
    useState<ProjectAgentEgressPolicy | null>(null);
  const [selectedEgress, setSelectedEgress] =
    useState<AgentEgressPolicy>("agent-ok");
  const [selected, setSelected] = useState<CaptureMode>(
    dashboard.overview.captureMode,
  );
  const [fullConsent, setFullConsent] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [egressProjectPath, setEgressProjectPath] = useState<string | null>(
    null,
  );
  const [egressSaving, setEgressSaving] = useState(false);
  const [egressError, setEgressError] = useState<string | null>(null);
  const [eraseArmed, setEraseArmed] = useState(false);
  const [eraseConfirmation, setEraseConfirmation] = useState("");
  const [erasing, setErasing] = useState(false);
  const [eraseError, setEraseError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  const [exported, setExported] = useState<AgentContinuityExport | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);
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

  useEffect(() => {
    let current = true;
    void readAgentEgressPolicy(projectPath)
      .then((next) => {
        if (!current) return;
        setEgressProjectPath(projectPath);
        setEgressPolicy(next);
        setSelectedEgress(next.projectPolicy);
        setEgressError(null);
      })
      .catch((cause) => {
        if (!current) return;
        setEgressProjectPath(projectPath);
        setEgressPolicy(null);
        setEgressError(errorMessage(cause));
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

  async function applyEgressPolicy() {
    if (
      !egressPolicy ||
      selectedEgress === egressPolicy.projectPolicy ||
      egressSaving
    ) {
      return;
    }
    setEgressSaving(true);
    setEgressError(null);
    try {
      const next = await updateAgentEgressPolicy(
        projectPath,
        egressPolicy.projectId,
        egressPolicy.projectPolicy,
        selectedEgress,
      );
      setEgressPolicy(next);
      setSelectedEgress(next.projectPolicy);
    } catch (cause) {
      setEgressError(errorMessage(cause));
      try {
        const latest = await readAgentEgressPolicy(projectPath);
        setEgressPolicy(latest);
        setSelectedEgress(latest.projectPolicy);
      } catch {
        setEgressPolicy(null);
      }
    } finally {
      setEgressSaving(false);
    }
  }

  async function exportContinuity() {
    if (exporting) return;
    setExportError(null);
    const parent = await chooseAgentContinuityExportParent();
    if (!parent) return;
    setExporting(true);
    try {
      setExported(await exportAgentProjectContinuity(projectPath, parent));
    } catch (cause) {
      setExported(null);
      setExportError(errorMessage(cause));
    } finally {
      setExporting(false);
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
    settings.mode !== "full-evidence" &&
    !fullConsent;
  const egressLoading = egressProjectPath !== projectPath;
  const visibleEgressPolicy = egressLoading ? null : egressPolicy;
  const visibleEgressError = egressLoading ? null : egressError;

  return (
    <CapturePrivacyContent
      settings={settings}
      dashboard={dashboard}
      selected={selected}
      setSelected={setSelected}
      fullConsent={fullConsent}
      setFullConsent={setFullConsent}
      saving={saving}
      error={error}
      changed={changed}
      fullNeedsConsent={fullNeedsConsent}
      applyMode={applyMode}
      egressPolicy={visibleEgressPolicy}
      selectedEgress={selectedEgress}
      setSelectedEgress={setSelectedEgress}
      egressLoading={egressLoading}
      egressSaving={egressSaving}
      egressError={visibleEgressError}
      applyEgressPolicy={applyEgressPolicy}
      eraseArmed={eraseArmed}
      onPrepareErase={() => {
        setEraseArmed((current) => !current);
        setEraseConfirmation("");
        setEraseError(null);
      }}
      eraseConfirmation={eraseConfirmation}
      setEraseConfirmation={setEraseConfirmation}
      erasing={erasing}
      eraseError={eraseError}
      eraseMemory={eraseMemory}
      exporting={exporting}
      exported={exported}
      exportError={exportError}
      exportContinuity={exportContinuity}
    />
  );
}

function CapturePrivacyContent({
  settings,
  dashboard,
  selected,
  setSelected,
  fullConsent,
  setFullConsent,
  saving,
  error,
  changed,
  fullNeedsConsent,
  applyMode,
  egressPolicy,
  selectedEgress,
  setSelectedEgress,
  egressLoading,
  egressSaving,
  egressError,
  applyEgressPolicy,
  eraseArmed,
  onPrepareErase,
  eraseConfirmation,
  setEraseConfirmation,
  erasing,
  eraseError,
  eraseMemory,
  exporting,
  exported,
  exportError,
  exportContinuity,
}: {
  settings: AgentCaptureSettings;
  dashboard: AgentMemoryDashboard;
  selected: CaptureMode;
  setSelected: (mode: CaptureMode) => void;
  fullConsent: boolean;
  setFullConsent: (consent: boolean) => void;
  saving: boolean;
  error: string | null;
  changed: boolean;
  fullNeedsConsent: boolean;
  applyMode: () => Promise<void>;
  egressPolicy: ProjectAgentEgressPolicy | null;
  selectedEgress: AgentEgressPolicy;
  setSelectedEgress: (policy: AgentEgressPolicy) => void;
  egressLoading: boolean;
  egressSaving: boolean;
  egressError: string | null;
  applyEgressPolicy: () => Promise<void>;
  eraseArmed: boolean;
  onPrepareErase: () => void;
  eraseConfirmation: string;
  setEraseConfirmation: (value: string) => void;
  erasing: boolean;
  eraseError: string | null;
  eraseMemory: () => Promise<void>;
  exporting: boolean;
  exported: AgentContinuityExport | null;
  exportError: string | null;
  exportContinuity: () => Promise<void>;
}) {
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
            The capture mode lives in this project’s small{" "}
            <span className="font-mono text-meta">.ley/capture.json</span> file;
            exclusions live in <span className="font-mono text-meta">.ley/.leyignore</span>.
            Applying a mode change rebuilds the cited snapshot in{" "}
            {dashboard.storage.kind === "native"
              ? "Ley’s private local app storage"
              : dashboard.storage.vaultName}
            ; it never uploads project data.
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

        {selected === "full-evidence" && settings.mode !== "full-evidence" && (
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
                I understand Full Evidence may retain supported original images
                without visual redaction. It does not authorize raw host
                transcript collection; any future transcript-capable adapter
                must request separate consent.
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
            Effective capture boundary
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
            <p className="text-meta font-semibold">Effective roots</p>
            <div className="mt-2 flex flex-wrap gap-2">
              {settings.approvedRoots.map((root) => (
                <span
                  key={root}
                  className="rounded-sm bg-surface-3 px-2 py-1 font-mono text-micro"
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

      <AgentEgressSection
        policy={egressPolicy}
        selected={selectedEgress}
        setSelected={setSelectedEgress}
        loading={egressLoading}
        saving={egressSaving}
        error={egressError}
        applyPolicy={applyEgressPolicy}
      />

      <ContinuityExportSection
        saving={saving}
        erasing={erasing}
        exporting={exporting}
        exported={exported}
        exportError={exportError}
        exportContinuity={exportContinuity}
      />

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
                Permanently removes captured artifacts, sessions, and lessons
                from Ley’s local continuity store. Your project files, ordinary
                Markdown notes, Canvas documents, and{" "}
                <span className="font-mono text-micro">.ley</span> settings stay
                in place. Any user-owned note or Canvas copy of erased Agent
                Memory remains until you delete that file separately with the
                external tool that owns it.
              </p>
            </div>
            <Button
              variant={eraseArmed ? "destructive" : "outline"}
              disabled={saving || erasing}
              onClick={onPrepareErase}
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
                  Stop connected Codex and Claude Code sessions first. Existing
                  backups, filesystem snapshots, or copies outside this vault
                  are not securely wiped.
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

function AgentEgressSection({
  policy,
  selected,
  setSelected,
  loading,
  saving,
  error,
  applyPolicy,
}: {
  policy: ProjectAgentEgressPolicy | null;
  selected: AgentEgressPolicy;
  setSelected: (policy: AgentEgressPolicy) => void;
  loading: boolean;
  saving: boolean;
  error: string | null;
  applyPolicy: () => Promise<void>;
}) {
  const retainedOverrides = policy
    ? [
        ...policy.specificationOverrides,
        ...policy.mountOverrides,
        ...policy.connectorOverrides,
      ]
    : [];
  const changed = Boolean(policy && selected !== policy.projectPolicy);
  const selectedOption =
    egressPolicies.find((option) => option.id === selected) ?? egressPolicies[0];

  return (
    <section aria-labelledby="agent-egress-title">
      <div className="mb-3">
        <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
          Agent sharing
        </p>
        <h3
          id="agent-egress-title"
          className="mt-1 text-lg font-semibold tracking-tight"
        >
          Control where Ley context may go
        </h3>
        <p className="mt-2 max-w-3xl text-meta leading-5 text-muted-foreground">
          This policy governs context supplied by Ley. It does not change your
          coding agent’s own provider, network, or account configuration.
        </p>
      </div>

      {error && (
        <div
          role="alert"
          className="mb-3 flex items-start gap-3 rounded-md border border-destructive/25 bg-destructive/8 p-4 text-meta"
        >
          <AlertTriangle
            size={17}
            className="mt-0.5 shrink-0 text-destructive"
            aria-hidden="true"
          />
          <div>
            <p className="font-semibold">Agent sharing error</p>
            <p className="mt-0.5 text-muted-foreground">{error}</p>
          </div>
        </div>
      )}

      {loading && !policy ? (
        <div className="rounded-md border border-border bg-surface-1 p-4 text-meta text-muted-foreground">
          <RefreshCw
            size={14}
            className="mr-2 inline animate-spin motion-reduce:animate-none"
            aria-hidden="true"
          />
          Reading agent sharing policy…
        </div>
      ) : policy ? (
        <div className="space-y-3 rounded-md border border-border bg-surface-1 p-4">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
            <label className="min-w-0 flex-1 text-meta font-semibold">
              <span className="block">Agent context sharing policy</span>
              <select
                aria-label="Agent context sharing policy"
                value={selected}
                disabled={saving}
                onChange={(event) =>
                  setSelected(event.target.value as AgentEgressPolicy)
                }
                className="mt-2 w-full rounded-sm border border-border bg-surface-2 px-3 py-2 text-meta font-normal text-foreground outline-none focus-visible:ring-2 focus-visible:ring-primary"
              >
                {egressPolicies.map((option) => (
                  <option key={option.id} value={option.id}>
                    {option.name}
                  </option>
                ))}
              </select>
            </label>
            <Button
              variant="primary"
              disabled={!changed || saving}
              onClick={() => void applyPolicy()}
            >
              {saving ? (
                <RefreshCw
                  size={14}
                  className="animate-spin motion-reduce:animate-none"
                />
              ) : (
                <ShieldCheck size={14} />
              )}
              {saving ? "Applying policy" : "Apply sharing policy"}
            </Button>
          </div>

          <p className="text-meta leading-5 text-muted-foreground">
            {selectedOption.description}
          </p>

          {selected === "local-model-only" && (
            <p className="text-micro leading-4 text-muted-foreground-strong">
              “Local” is an explicit integration assertion; Ley does not verify
              that the downstream provider or runtime is actually local.
            </p>
          )}
          {selected === "confirm-per-use" && (
            <p className="text-micro leading-4 text-warning">
              No confirmation prompt exists yet. This remains fail-closed for
              both cloud and local agent targets.
            </p>
          )}

          <p className="text-micro leading-4 text-subtle-foreground">
            {changed
              ? "Current: " + humanize(policy.projectPolicy) + "."
              : "This is the current project policy."}{" "}
            Updates reject a stale Desktop view if the project identity or
            policy changed instead of overwriting newer authority.
          </p>

          <div className="border-t border-border pt-3">
            <p className="text-meta font-semibold">Retained source restrictions</p>
            <p className="mt-1 text-micro leading-4 text-muted-foreground">
              {retainedOverrides.length === 0
                ? "None."
                : "Older source-specific restrictions remain enforced and read-only here; new restrictions are project-level."}
            </p>
            {retainedOverrides.length > 0 && (
              <ul className="mt-2 space-y-1 text-micro">
                {retainedOverrides.map((override) => (
                  <li
                    key={override.scopeKind + ":" + override.scopeId}
                    className="flex min-w-0 justify-between gap-3"
                  >
                    <span className="truncate font-mono">{override.scopeId}</span>
                    <span className="shrink-0 text-muted-foreground">
                      {humanize(override.policy)}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <p className="text-micro leading-4 text-subtle-foreground">
            {policy.privacyNotice}
          </p>
        </div>
      ) : (
        <div className="rounded-md border border-destructive/25 bg-destructive/8 p-4 text-meta">
          Agent sharing policy is unavailable.
        </div>
      )}
    </section>
  );
}

function ContinuityExportSection({
  saving,
  erasing,
  exporting,
  exported,
  exportError,
  exportContinuity,
}: {
  saving: boolean;
  erasing: boolean;
  exporting: boolean;
  exported: AgentContinuityExport | null;
  exportError: string | null;
  exportContinuity: () => Promise<void>;
}) {
  return (
    <section className="overflow-hidden rounded-sm border border-border bg-surface-1 shadow-panel">
      <div className="flex flex-col gap-4 p-5 sm:flex-row sm:items-center sm:justify-between sm:p-6">
        <div className="max-w-2xl">
          <p className="text-micro font-semibold uppercase tracking-[0.14em] text-primary">
            Portability
          </p>
          <h3 className="mt-1 text-lg font-semibold tracking-tight">
            Export continuity bundle
          </h3>
          <p className="mt-1 text-meta leading-5 text-muted-foreground">
            Creates a local portable bundle containing this project’s
            continuity database, any immutable imported approved-source snapshots
            stored there, and only the artifact evidence required by durable
            citations. Choose a folder outside the project; Ley creates a new
            export directory inside it. The destination may be synced or shared
            by other software, so choose it accordingly.
          </p>
        </div>
        <Button
          variant="outline"
          disabled={saving || exporting || erasing}
          onClick={() => void exportContinuity()}
        >
          {exporting ? (
            <RefreshCw
              size={14}
              className="animate-spin motion-reduce:animate-none"
            />
          ) : (
            <Download size={14} aria-hidden="true" />
          )}
          {exporting ? "Exporting" : "Export bundle…"}
        </Button>
      </div>
      {(exported || exportError) && (
        <div className="border-t border-border px-5 py-4 sm:px-6">
          {exported ? (
            <div className="text-meta">
              <p className="font-semibold">Export complete</p>
              <p
                className="mt-1 break-all font-mono text-micro text-muted-foreground"
                title={exported.destination}
              >
                {exported.destination}
              </p>
              <p className="mt-2 text-micro text-muted-foreground">
                {exported.eventCount.toLocaleString()} events ·{" "}
                {exported.artifactSnapshots.toLocaleString()} cited snapshots ·{" "}
                {exported.evidenceBlobs.toLocaleString()} evidence blobs
              </p>
            </div>
          ) : (
            <p role="alert" className="text-meta text-destructive">
              {exportError}
            </p>
          )}
        </div>
      )}
    </section>
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
