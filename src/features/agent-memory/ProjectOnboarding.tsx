import {
  ArrowRight,
  BrainCircuit,
  CheckCircle2,
  FolderOpen,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import { Button } from "@/shared/components/Button";
import { ErrorNotice } from "./AgentMemoryPresentation";
import { HostIntegrationsPanel } from "./HostIntegrationsPanel";
import type {
  AgentInitialCapturePreview,
  AgentProjectInspection,
} from "./types";

function onboardingCopy(
  inspection: AgentProjectInspection | null,
  projectPath: string | null,
): { title: string; body: string } {
  if (!inspection) {
    return projectPath
      ? {
          title: "Opening local project",
          body: "Ley is validating this project’s local identity, continuity authority, and captured memory.",
        }
      : {
          title: "Choose a project",
          body: "Ley only reads a project after you choose its folder. It never scans neighboring folders or discovers projects silently.",
        };
  }
  switch (inspection.status) {
    case "uninitialized":
      return {
        title: "Review what Ley will capture",
        body: `Nothing has been written yet. Review the proposed capture boundary for “${inspection.suggestedName}”. Durable continuity will stay in Ley’s private local app storage.`,
      };
    case "unbound":
      return {
        title: "Reconnect historical Ley data",
        body: `“${inspection.projectName}” is a pre-cutover initialized project without a private vault binding. Choose the existing legacy Ley vault that already contains captured memory for this exact project. Ley will not create a new legacy vault.`,
      };
    case "vault-unavailable":
      return {
        title: "Reconnect legacy migration source",
        body: `“${inspection.projectName}” was connected to “${inspection.previousVaultName}”, which moved or is unavailable. Choose that legacy Ley vault explicitly to finish migration into native continuity.`,
      };
    case "needs-capture":
      return {
        title: "Create the first snapshot",
        body:
          inspection.storage.kind === "legacy-vault"
            ? `“${inspection.projectName}” is connected to “${inspection.storage.vaultName}” but has not been captured yet.`
            : `“${inspection.projectName}” has native local continuity authority but has not been captured yet.`,
      };
    case "ready":
      return { title: "Project ready", body: "This project is ready." };
  }
}

function InitialCapturePreviewCard({
  preview,
  storageLabel,
}: {
  preview: AgentInitialCapturePreview;
  storageLabel: string;
}) {
  const hasHardBoundSkips =
    preview.skippedOversized > 0 ||
    preview.skippedTotalLimit > 0 ||
    preview.skippedSymlinks > 0;
  return (
    <div className="mt-4 rounded-md border border-border bg-background/35 p-4">
      <div className="flex items-start gap-3">
        <ShieldCheck size={16} className="mt-0.5 shrink-0 text-primary" />
        <div className="min-w-0 flex-1">
          <p className="text-meta font-semibold">Proposed capture boundary</p>
          <p className="mt-1 text-micro leading-5 text-muted-foreground">
            Structured capture · {preview.eligibleFiles.toLocaleString()}{" "}
            eligible {preview.eligibleFiles === 1 ? "file" : "files"} ·{" "}
            {formatOnboardingBytes(preview.eligibleBytes)} · durable memory in{" "}
            {storageLabel}
          </p>
        </div>
      </div>

      <div className="mt-3 grid gap-2 text-micro sm:grid-cols-2">
        <div className="rounded-sm bg-surface-1 px-3 py-2">
          <span className="text-muted-foreground">Effective roots</span>
          <p className="mt-0.5 font-mono text-foreground">
            {preview.approvedRoots.join(", ")}
          </p>
        </div>
        <div className="rounded-sm bg-surface-1 px-3 py-2">
          <span className="text-muted-foreground">Capture limits</span>
          <p className="mt-0.5 text-foreground">
            {formatOnboardingBytes(preview.maxFileBytes)} per file ·{" "}
            {formatOnboardingBytes(preview.maxTotalBytes)} total
          </p>
        </div>
      </div>

      {preview.includedPaths.length > 0 && (
        <div className="mt-3">
          <p className="text-micro font-medium text-muted-foreground">
            Eligible path sample
          </p>
          <div className="mt-1.5 flex flex-wrap gap-1.5">
            {preview.includedPaths.map((path) => (
              <span
                key={path}
                className="max-w-full truncate rounded bg-surface-2 px-2 py-1 font-mono text-micro text-muted-foreground-strong"
                title={path}
              >
                {path}
              </span>
            ))}
            {preview.omittedIncludedPaths > 0 && (
              <span className="rounded bg-surface-2 px-2 py-1 text-micro text-muted-foreground">
                +{preview.omittedIncludedPaths.toLocaleString()} more
              </span>
            )}
          </div>
        </div>
      )}

      {hasHardBoundSkips && (
        <p className="mt-3 text-micro leading-5 text-muted-foreground">
          Also skipped by hard bounds:{" "}
          {preview.skippedOversized.toLocaleString()} oversized,{" "}
          {preview.skippedTotalLimit.toLocaleString()} beyond total limit,{" "}
          {preview.skippedSymlinks.toLocaleString()} symlink
          {preview.skippedSymlinks === 1 ? "" : "s"}.
        </p>
      )}
      <p className="mt-3 text-micro leading-5 text-muted-foreground">
        {preview.exclusionNotice}
      </p>
      <p className="mt-2 text-micro leading-5 text-muted-foreground-strong">
        {preview.privacyNotice}
      </p>
    </div>
  );
}

export function ProjectOnboarding({
  inspection,
  projectPath,
  busy,
  error,
  onChoose,
  onForget,
  onInitialize,
  onConnect,
  onCapture,
}: {
  inspection: AgentProjectInspection | null;
  projectPath: string | null;
  busy: boolean;
  error: string | null;
  onChoose: () => void;
  onForget: () => void;
  onInitialize: () => void;
  onConnect: () => void;
  onCapture: () => void;
}) {
  const copy = onboardingCopy(inspection, projectPath);
  const primaryAction = inspection
    ? onboardingPrimaryAction(inspection, onInitialize, onConnect, onCapture)
    : null;
  return (
    <main className="min-h-0 flex-1 overflow-y-auto px-4 py-10 sm:px-6">
      <div className="mx-auto flex min-h-full max-w-xl items-center justify-center">
        <div className="w-full rounded-sm border border-border bg-surface-1 p-6 shadow-panel sm:p-8">
          <div className="flex size-11 items-center justify-center rounded-md border border-primary/20 bg-primary/10 text-primary">
            {inspection ? <BrainCircuit size={21} /> : <FolderOpen size={21} />}
          </div>
          <h2 className="mt-5 text-2xl font-semibold tracking-[-0.035em]">
            {copy.title}
          </h2>
          <p className="mt-2 text-body leading-6 text-muted-foreground-strong">
            {copy.body}
          </p>
          {projectPath && (
            <p
              className="mt-4 truncate rounded-sm bg-background/40 px-3 py-2 font-mono text-micro text-muted-foreground"
              title={projectPath}
            >
              {projectPath}
            </p>
          )}
          {inspection?.status === "uninitialized" && (
            <InitialCapturePreviewCard
              preview={inspection.preview}
              storageLabel="Ley’s private local app storage"
            />
          )}
          {error && (
            <div className="mt-4">
              <ErrorNotice message={error} />
            </div>
          )}
          <div className="mt-6 flex flex-wrap gap-2">
            {!inspection ? (
              <>
                <Button variant="primary" disabled={busy} onClick={onChoose}>
                  {busy ? (
                    <RefreshCw
                      size={14}
                      className="animate-spin motion-reduce:animate-none"
                    />
                  ) : (
                    <FolderOpen size={14} />
                  )}
                  {busy ? "Opening…" : "Choose project folder"}
                </Button>
                {projectPath && (
                  <Button variant="outline" onClick={onForget}>
                    Back to projects
                  </Button>
                )}
              </>
            ) : (
              <>
                <Button
                  variant="primary"
                  disabled={busy}
                  onClick={primaryAction?.onClick}
                >
                  {busy ? (
                    <RefreshCw
                      size={14}
                      className="animate-spin motion-reduce:animate-none"
                    />
                  ) : (
                    <ArrowRight size={14} />
                  )}
                  {busy ? "Preparing Ley…" : primaryAction?.label}
                </Button>
                <Button variant="outline" disabled={busy} onClick={onChoose}>
                  Choose another
                </Button>
                <Button variant="ghost" disabled={busy} onClick={onForget}>
                  Back to projects
                </Button>
              </>
            )}
          </div>
          <div className="mt-6 border-t border-border pt-4">
            <p className="flex gap-2 text-micro leading-5 text-muted-foreground">
              <ShieldCheck size={14} className="mt-0.5 shrink-0 text-primary" />
              Known credentials, private keys, environment files, build output,
              and ignored paths are excluded before durable memory is written.
            </p>
          </div>
        </div>
      </div>
    </main>
  );
}

function onboardingPrimaryAction(
  inspection: AgentProjectInspection,
  onInitialize: () => void,
  onConnect: () => void,
  onCapture: () => void,
) {
  switch (inspection.status) {
    case "uninitialized":
      return { onClick: onInitialize, label: "Enable Ley for this project" };
    case "unbound":
      return { onClick: onConnect, label: "Reconnect & migrate" };
    case "vault-unavailable":
      return { onClick: onConnect, label: "Reconnect & migrate" };
    default:
      return { onClick: onCapture, label: "Capture project" };
  }
}

export function ProjectSetupReady({
  projectName,
  projectPath,
  onContinue,
}: {
  projectName: string;
  projectPath: string;
  onContinue: () => void;
}) {
  return (
    <main className="min-h-0 flex-1 overflow-y-auto px-4 py-10 sm:px-6">
      <div className="mx-auto flex min-h-full max-w-xl items-center justify-center">
        <div className="w-full rounded-sm border border-border bg-surface-1 p-6 shadow-panel sm:p-8">
          <div className="flex size-11 items-center justify-center rounded-md border border-success/25 bg-success/10 text-success">
            <CheckCircle2 size={21} aria-hidden="true" />
          </div>
          <h2 className="mt-5 text-2xl font-semibold tracking-[-0.035em]">
            Ley is ready for {projectName}
          </h2>
          <p className="mt-2 text-body leading-6 text-muted-foreground-strong">
            The project is enabled and its first local continuity snapshot is
            ready. You can open Ley now, or connect a detected coding agent
            without leaving this setup flow.
          </p>
          <div className="mt-6 border-t border-border pt-5">
            <HostIntegrationsPanel projectPath={projectPath} compact />
          </div>
          <Button variant="primary" className="mt-6" onClick={onContinue}>
            Open project
            <ArrowRight size={14} aria-hidden="true" />
          </Button>
          <p className="mt-6 border-t border-border pt-4 text-micro leading-5 text-muted-foreground">
            No account · no knowledge cloud · continuity stays in Ley’s private
            local app storage.
          </p>
        </div>
      </div>
    </main>
  );
}

function formatOnboardingBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)} ${unit}`;
}
