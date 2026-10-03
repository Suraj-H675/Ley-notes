import { lazy, Suspense, useEffect, useState, type ReactNode } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import {
  AlertTriangle,
  BookCheck,
  BrainCircuit,
  CheckCircle2,
  GitBranch,
  History,
  MessageSquareWarning,
  PencilLine,
  ShieldCheck,
  Trash2,
  X,
} from "lucide-react";
import { Button } from "@/shared/components/Button";
import { cn } from "@/shared/lib/classnames";
import {
  absoluteTime,
  CompactEmpty,
  errorMessage,
  ErrorNotice,
  humanize,
  KnowledgeSurfaceFallback,
  relativeTime,
  SessionStatus,
} from "./AgentMemoryPresentation";
import { readAgentSession, readAgentSessionTurns } from "./api";
import type {
  AgentMemoryDashboard,
  ArtifactEvidenceReference,
  ProjectRevisionFreshness,
  RevisionApplicability,
  RevisionCompatibility,
  SessionContext,
  SessionTurnsContext,
} from "./types";

const SessionRenameEditor = lazy(() =>
  import("./SessionRenameEditor").then((module) => ({
    default: module.SessionRenameEditor,
  })),
);
const SessionErasureEditor = lazy(() =>
  import("./SessionErasureEditor").then((module) => ({
    default: module.SessionErasureEditor,
  })),
);

function SectionLabel({
  id,
  icon: Icon,
  label,
}: {
  id: string;
  icon: typeof History;
  label: string;
}) {
  return (
    <h3
      id={id}
      className="flex items-center gap-2 text-meta font-semibold text-muted-foreground"
    >
      <Icon size={13} className="text-secondary" />
      {label}
    </h3>
  );
}

function RecordGroup({
  icon: Icon,
  title,
  count,
  children,
}: {
  icon: typeof History;
  title: string;
  count: number;
  children: ReactNode;
}) {
  return (
    <section className="border-t border-border pt-3">
      <h4 className="flex items-center gap-2 text-micro font-medium text-muted-foreground">
        <Icon size={13} />
        {title}
        <span className="ml-auto tabular-nums">{count}</span>
      </h4>
      <div className="mt-2 divide-y divide-border/70">{children}</div>
    </section>
  );
}

function RecordItem({
  title,
  body,
  meta,
}: {
  title: string;
  body?: string;
  meta?: string;
}) {
  return (
    <div className="py-2.5">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <p className="text-meta font-medium">{title}</p>
        {meta && (
          <span className="text-micro text-muted-foreground">{meta}</span>
        )}
      </div>
      {body && (
        <p className="mt-1 whitespace-pre-wrap text-micro leading-5 text-muted-foreground-strong">
          {body}
        </p>
      )}
    </div>
  );
}

function VerificationItem({
  verification,
  onEvidence,
}: {
  verification: SessionContext["checkpoints"][number]["verification"][number];
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
}) {
  return (
    <div className="py-2.5">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <p className="text-meta font-medium">{humanize(verification.kind)}</p>
        <span className="text-micro text-muted-foreground">
          {humanize(verification.status)}
        </span>
      </div>
      {verification.summary && (
        <p className="mt-1 whitespace-pre-wrap text-micro leading-5 text-muted-foreground-strong">
          {verification.summary}
        </p>
      )}
      {verification.command && (
        <code className="mt-2 block overflow-x-auto rounded-sm border border-border bg-background/45 px-2 py-1.5 font-mono text-micro text-muted-foreground-strong">
          {verification.command}
        </code>
      )}
      {verification.evidenceArtifacts.length > 0 && (
        <div className="mt-2 flex flex-wrap gap-1.5">
          {verification.evidenceArtifacts.map((artifact) => (
            <button
              type="button"
              key={`${artifact.artifactSnapshotId}:${artifact.artifactPath}:${artifact.contentHash}`}
              title={
                artifact.mediaType
                  ? `Original ${artifact.mediaType} verification evidence · snapshot ${artifact.artifactSnapshotId}`
                  : `${artifact.artifactPath}:${artifact.startLine}-${artifact.endLine} · snapshot ${artifact.artifactSnapshotId}`
              }
              onClick={() => onEvidence(artifact)}
              className="max-w-full touch-manipulation truncate rounded-sm border border-border bg-background/55 px-2 py-1 text-left font-mono text-micro text-muted-foreground outline-none transition-[transform,border-color,background-color,color] hover:border-primary/35 hover:bg-primary/7 hover:text-foreground active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
            >
              {artifact.artifactPath}
              {artifact.mediaType
                ? ` · ${artifact.mediaType}`
                : `:${artifact.startLine}`}
            </button>
          ))}
        </div>
      )}
      {verification.evidenceArtifactsOmitted > 0 && (
        <p className="mt-2 text-micro text-muted-foreground">
          {verification.evidenceArtifactsOmitted} more verification evidence{" "}
          {verification.evidenceArtifactsOmitted === 1
            ? "citation is"
            : "citations are"}{" "}
          omitted from this bounded session view.
        </p>
      )}
    </div>
  );
}

function ProblemItem({
  problem,
}: {
  problem: SessionContext["checkpoints"][number]["problems"][number];
}) {
  return (
    <div className="py-2.5">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <p className="text-meta font-medium">{problem.title}</p>
        <span className="text-micro text-muted-foreground">
          {problem.resolutionDetail
            ? "Resolved"
            : problem.latestAttemptOutcome
              ? `Latest · ${humanize(problem.latestAttemptOutcome)}`
              : "Unresolved"}
        </span>
      </div>
      <p className="mt-1 whitespace-pre-wrap text-micro leading-5 text-muted-foreground-strong">
        {problem.symptom}
      </p>
      {problem.attempts.length > 0 && (
        <ol className="mt-3 space-y-2 border-l border-border pl-3">
          {problem.attempts.map((attempt, index) => (
            <li key={attempt.id}>
              <p className="text-micro font-medium">
                Attempt {index + 1} · {humanize(attempt.outcome)}
              </p>
              <p className="mt-0.5 text-micro leading-5 text-muted-foreground-strong">
                {attempt.action}
              </p>
              {attempt.evidence && (
                <p className="mt-0.5 text-micro italic leading-5 text-muted-foreground">
                  Evidence: {attempt.evidence}
                </p>
              )}
            </li>
          ))}
        </ol>
      )}
      {problem.resolutionDetail && (
        <div className="mt-3 rounded-md border border-success/15 bg-success/7 px-3 py-2">
          <p className="text-micro font-medium text-success">Resolution</p>
          <p className="mt-1 text-micro leading-5 text-muted-foreground-strong">
            <span className="font-medium text-foreground">Root cause:</span>{" "}
            {problem.resolutionDetail.rootCause}
          </p>
          <p className="mt-1 text-micro leading-5 text-muted-foreground-strong">
            <span className="font-medium text-foreground">Changed:</span>{" "}
            {problem.resolutionDetail.change}
          </p>
          {problem.resolutionDetail.verification && (
            <p className="mt-1 text-micro leading-5 text-muted-foreground-strong">
              <span className="font-medium text-foreground">Verified:</span>{" "}
              {problem.resolutionDetail.verification}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

function MemoryList({
  title,
  items,
  tone,
}: {
  title: string;
  items: string[];
  tone: "warning" | "neutral";
}) {
  return (
    <div className="mt-3">
      <p
        className={cn(
          "text-micro font-medium",
          tone === "warning" ? "text-warning" : "text-muted-foreground",
        )}
      >
        {title}
      </p>
      <ul className="mt-1 space-y-1 text-micro leading-5 text-muted-foreground-strong">
        {items.map((item, index) => (
          <li key={`${index}:${item}`} className="flex gap-2">
            <span className="mt-2 size-1 shrink-0 rounded-full bg-border-strong" />
            <span>{item}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

function sourceLabel(session: SessionContext): string {
  const parts = [
    humanize(session.source.kind),
    session.source.host,
    session.source.agent,
  ].filter((part): part is string => Boolean(part));
  return parts.join(" · ");
}

export function SessionInspector({
  sessionId,
  projectPath,
  onClose,
  onEvidence,
  onRenamed,
  onErased,
}: {
  sessionId: string | null;
  projectPath: string;
  onClose: () => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
  onRenamed: (dashboard: AgentMemoryDashboard) => void;
  onErased: (dashboard: AgentMemoryDashboard) => void;
}) {
  const [session, setSession] = useState<SessionContext | null>(null);
  const [turns, setTurns] = useState<SessionTurnsContext | null>(null);
  const [turnsBusy, setTurnsBusy] = useState(false);
  const [turnsError, setTurnsError] = useState<string | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [renameDirty, setRenameDirty] = useState(false);
  const [erasing, setErasing] = useState(false);
  const [erasureDirty, setErasureDirty] = useState(false);
  const [busy, setBusy] = useState(Boolean(sessionId));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!sessionId) return;
    let current = true;
    void readAgentSession(projectPath, sessionId)
      .then((next) => {
        if (current) {
          setSession(next);
          setError(null);
        }
      })
      .catch((cause) => {
        if (current) setError(errorMessage(cause));
      })
      .finally(() => {
        if (current) setBusy(false);
      });
    return () => {
      current = false;
    };
  }, [projectPath, sessionId]);

  function loadTurns() {
    if (!sessionId || turns || turnsBusy) return;
    setTurnsBusy(true);
    setTurnsError(null);
    void readAgentSessionTurns(projectPath, sessionId)
      .then(setTurns)
      .catch((cause) => setTurnsError(errorMessage(cause)))
      .finally(() => setTurnsBusy(false));
  }

  function toggleRenaming() {
    if (
      ((renaming && renameDirty) || (erasing && erasureDirty)) &&
      !window.confirm("Discard your unsaved changes?")
    ) {
      return;
    }
    setRenameDirty(false);
    setErasureDirty(false);
    setErasing(false);
    setRenaming((current) => !current);
  }

  function toggleErasing() {
    if (
      ((erasing && erasureDirty) || (renaming && renameDirty)) &&
      !window.confirm("Discard your unsaved changes?")
    ) {
      return;
    }
    setErasureDirty(false);
    setRenameDirty(false);
    setRenaming(false);
    setErasing((current) => !current);
  }

  return (
    <Dialog.Root
      open={Boolean(sessionId)}
      onOpenChange={(next) => {
        if (
          !next &&
          ((!(renaming && renameDirty) && !(erasing && erasureDirty)) ||
            window.confirm("Discard your unsaved session changes?"))
        ) {
          onClose();
        }
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="app-modal-overlay fixed inset-0 z-[80]" />
        <Dialog.Content
          className="app-modal-surface fixed inset-x-3 bottom-3 top-3 z-[81] mx-auto flex max-w-4xl flex-col overflow-hidden rounded-sm border outline-none sm:inset-x-6 sm:bottom-6 sm:top-6"
          aria-describedby={undefined}
        >
          <SessionInspectorHeader
            session={session}
            renaming={renaming}
            onToggleRenaming={toggleRenaming}
          />

          <SessionInspectorBody
            session={session}
            busy={busy}
            error={error}
            turns={turns}
            turnsBusy={turnsBusy}
            turnsError={turnsError}
            loadTurns={loadTurns}
            erasing={erasing}
            onToggleErasing={toggleErasing}
            onEvidence={onEvidence}
          />
          {renaming && session && (
            <div className="shrink-0 border-t border-border bg-surface-1">
              <Suspense fallback={<KnowledgeSurfaceFallback />}>
                <SessionRenameEditor
                  projectPath={projectPath}
                  session={session}
                  onCancel={() => {
                    if (
                      renameDirty &&
                      !window.confirm("Discard your unsaved session rename?")
                    ) {
                      return;
                    }
                    setRenameDirty(false);
                    setRenaming(false);
                  }}
                  onDirtyChange={setRenameDirty}
                  onRenamed={(dashboard) => {
                    onRenamed(dashboard);
                    setRenameDirty(false);
                    setRenaming(false);
                    setBusy(true);
                    setError(null);
                    void readAgentSession(projectPath, session.sessionId)
                      .then(setSession)
                      .catch((cause) => setError(errorMessage(cause)))
                      .finally(() => setBusy(false));
                  }}
                />
              </Suspense>
            </div>
          )}
          {erasing && session && (
            <div
              id="session-erasure-panel"
              className="shrink-0 border-t border-destructive/20 bg-surface-1"
            >
              <Suspense fallback={<KnowledgeSurfaceFallback />}>
                <SessionErasureEditor
                  projectPath={projectPath}
                  session={session}
                  onCancel={() => {
                    if (
                      erasureDirty &&
                      !window.confirm("Discard this erasure confirmation?")
                    ) {
                      return;
                    }
                    setErasureDirty(false);
                    setErasing(false);
                  }}
                  onDirtyChange={setErasureDirty}
                  onErased={(result) => onErased(result.dashboard)}
                />
              </Suspense>
            </div>
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function SessionInspectorBody({
  session,
  busy,
  error,
  turns,
  turnsBusy,
  turnsError,
  loadTurns,
  erasing,
  onToggleErasing,
  onEvidence,
}: {
  session: SessionContext | null;
  busy: boolean;
  error: string | null;
  turns: SessionTurnsContext | null;
  turnsBusy: boolean;
  turnsError: string | null;
  loadTurns: () => void;
  erasing: boolean;
  onToggleErasing: () => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
}) {
  return (
    <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain p-4 sm:p-6">
      {busy && !session ? (
        <div className="py-20 text-center text-meta text-muted-foreground">
          Replaying structured session events…
        </div>
      ) : error && !session ? (
        <ErrorNotice message={error} />
      ) : session ? (
        <div className="space-y-7">
          {error && (
            <p
              className="rounded-md border border-destructive/25 bg-destructive/8 p-3 text-micro text-destructive"
              role="alert"
            >
              {error}
            </p>
          )}
          <SessionOverview session={session} />
          <SessionUtilityMeasurement session={session} />
          <SessionCapturedTurns
            session={session}
            turns={turns}
            turnsBusy={turnsBusy}
            turnsError={turnsError}
            loadTurns={loadTurns}
          />
          <SessionNamingHistory session={session} />
          <SessionOutcome session={session} />
          <SessionCheckpointTimeline
            session={session}
            onEvidence={onEvidence}
          />
          {session.omittedCheckpoints > 0 && (
            <p className="rounded-md border border-warning/20 bg-warning/10 px-3 py-2 text-micro text-warning">
              {session.omittedCheckpoints} older checkpoints were omitted from
              this bounded view.
            </p>
          )}
          <p className="border-l-2 border-secondary/45 pl-3 text-micro leading-5 text-muted-foreground">
            <MessageSquareWarning
              size={13}
              className="mr-2 inline text-secondary"
            />
            {session.instructionWarning}
          </p>
          <SessionLocalData
            erasing={erasing}
            onToggleErasing={onToggleErasing}
          />
        </div>
      ) : null}
    </div>
  );
}

function SessionOverview({ session }: { session: SessionContext }) {
  return (
    <section className="border-y border-border py-4 sm:py-5">
      <div className="flex flex-wrap items-center gap-3">
        <SessionStatus status={session.status} />
        <span className="text-micro text-muted-foreground">
          {sourceLabel(session)}
        </span>
        <span className="text-micro text-muted-foreground">
          {new Intl.DateTimeFormat(undefined, {
            dateStyle: "medium",
            timeStyle: "short",
          }).format(session.startedAtUnixMs)}
        </span>
      </div>
      <h3 className="mt-4 text-meta font-semibold text-muted-foreground">
        Goal
      </h3>
      <p className="mt-1 whitespace-pre-wrap text-body leading-6">
        {session.goal}
      </p>
      <div className="mt-4 flex flex-wrap gap-x-5 gap-y-1 text-micro text-muted-foreground">
        <span>{session.checkpointCount} checkpoints</span>
        <span>
          {session.promptCount ?? 0} prompts · {session.responseCount ?? 0}{" "}
          responses
        </span>
        <span>{session.eventCount} immutable events</span>
        <span>~{session.estimatedTextTokens} context tokens</span>
      </div>
    </section>
  );
}

function SessionUtilityMeasurement({ session }: { session: SessionContext }) {
  if (session.contextUtilityBindingCount <= 0) return null;

  return (
    <section aria-labelledby="session-utility-measurement-title">
      <SectionLabel
        id="session-utility-measurement-title"
        icon={BrainCircuit}
        label="Context utility measurement"
      />
      <div className="mt-2 border-y border-border py-4 sm:py-5">
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-meta">
          <span>{session.contextUtilityBindingCount} bound</span>
          <span>{session.observedContextUtilityBindingCount} observed</span>
          <span>{session.unobservedContextUtilityBindingCount} unobserved</span>
        </div>
        <p className="mt-2 text-micro leading-5 text-muted-foreground">
          Measurement provenance only. These counts do not prove that supplied
          context was used, helpful, harmful, or causally responsible for the
          session outcome.
        </p>

        {session.unobservedContextUtilityBindings.length > 0 && (
          <div className="mt-3 divide-y divide-border/70 border-t border-border pt-1">
            {session.unobservedContextUtilityBindings.map((binding) => (
              <article
                key={binding.bindingId}
                className="py-3"
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="font-mono text-micro text-muted-foreground-strong">
                    {binding.bindingId}
                  </span>
                  <time className="text-micro text-muted-foreground">
                    {relativeTime(binding.recordedAtUnixMs)}
                  </time>
                </div>
                <p className="mt-1 text-micro leading-5 text-muted-foreground">
                  {binding.includedRecordCount} included record handles
                  {binding.omittedIncludedRecords > 0
                    ? ` · ${binding.omittedIncludedRecords} omitted`
                    : ""}
                  {` · ~${binding.estimatedTokens} tokens`}
                </p>
                {binding.terminalFinishEventId && (
                  <p className="mt-2 text-micro leading-5 text-muted-foreground-strong">
                    The terminal finish event is retained as an exact outcome
                    anchor for an explicit utility observation.
                  </p>
                )}
              </article>
            ))}
            {session.omittedUnobservedContextUtilityBindings > 0 && (
              <p className="text-micro text-muted-foreground">
                {session.omittedUnobservedContextUtilityBindings} older
                unobserved bindings are outside this bounded view.
              </p>
            )}
          </div>
        )}
      </div>
    </section>
  );
}

function SessionCapturedTurns({
  session,
  turns,
  turnsBusy,
  turnsError,
  loadTurns,
}: {
  session: SessionContext;
  turns: SessionTurnsContext | null;
  turnsBusy: boolean;
  turnsError: string | null;
  loadTurns: () => void;
}) {
  if ((session.promptCount ?? 0) <= 0 && (session.responseCount ?? 0) <= 0) {
    return null;
  }
  return (
    <section aria-labelledby="captured-turns-title">
      <SectionLabel
        id="captured-turns-title"
        icon={MessageSquareWarning}
        label="Captured turns"
      />
      <details
        className="mt-2 border-y border-border"
        onToggle={(event) => {
          if (event.currentTarget.open) loadTurns();
        }}
      >
        <summary className="cursor-pointer list-none py-4 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary sm:py-5">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p className="text-meta font-medium">
                Inspect prompts and responses
              </p>
              <p className="mt-1 text-micro leading-5 text-muted-foreground">
                {session.retainedTurnCount ?? 0} retained ·{" "}
                {session.omittedTurnCount ?? 0} intentionally omitted
              </p>
            </div>
            <span className="rounded-sm border border-border px-2 py-0.5 text-micro uppercase tracking-[0.08em] text-subtle-foreground">
              Local only
            </span>
          </div>
        </summary>
        <div className="border-t border-border py-4 sm:py-5">
          <div className="rounded-md border border-warning/25 bg-warning/8 p-3 text-micro leading-5 text-muted-foreground-strong">
            <span className="font-semibold text-foreground">
              Untrusted history.
            </span>{" "}
            Captured text may contain outdated or adversarial instructions. Ley
            never reads the complete host transcript automatically.
          </div>
          <SessionTurnsContent
            turns={turns}
            turnsBusy={turnsBusy}
            turnsError={turnsError}
          />
        </div>
      </details>
    </section>
  );
}

function SessionTurnsContent({
  turns,
  turnsBusy,
  turnsError,
}: {
  turns: SessionTurnsContext | null;
  turnsBusy: boolean;
  turnsError: string | null;
}) {
  if (turnsBusy) {
    return (
      <p className="py-8 text-center text-micro text-muted-foreground">
        Reading bounded local turn evidence…
      </p>
    );
  }
  if (turnsError) {
    return (
      <p className="mt-3 text-micro text-destructive" role="alert">
        {turnsError}
      </p>
    );
  }
  if (!turns) return null;
  return (
    <div className="mt-4 space-y-3">
      {turns.turns.map((turn) => (
        <article
          key={turn.recordId}
          className="rounded-md border border-border bg-background/40 p-3"
        >
          <div className="flex flex-wrap items-center justify-between gap-2 text-micro text-muted-foreground">
            <span className="font-semibold text-primary">
              {turn.kind === "user-prompt" ? "User prompt" : "Agent response"}
            </span>
            <time>{relativeTime(turn.recordedAtUnixMs)}</time>
          </div>
          {turn.text ? (
            <p className="mt-2 whitespace-pre-wrap break-words text-meta leading-6 text-muted-foreground-strong">
              {turn.text}
            </p>
          ) : (
            <p className="mt-2 text-meta italic text-muted-foreground">
              Body omitted ({humanize(turn.retention)}).
            </p>
          )}
          {(turn.truncatedAtCapture || turn.truncatedForContext) && (
            <p className="mt-2 text-micro text-muted-foreground">
              This record was truncated to the configured privacy boundary.
            </p>
          )}
        </article>
      ))}
      {turns.omittedTurns > 0 && (
        <p className="text-micro text-muted-foreground">
          {turns.omittedTurns} older turn records are outside this bounded view.
        </p>
      )}
    </div>
  );
}

function SessionNamingHistory({ session }: { session: SessionContext }) {
  if (session.renameCount <= 0) return null;
  return (
    <section aria-labelledby="session-naming-history-title">
      <SectionLabel
        id="session-naming-history-title"
        icon={PencilLine}
        label="Naming history"
      />
      <div className="mt-2 border-y border-border py-4">
        <div className="border-b border-border pb-3">
          <p className="text-micro font-medium text-muted-foreground">
            Original name
          </p>
          <p className="mt-1 text-meta font-medium">{session.originalName}</p>
        </div>
        <ol className="mt-3 space-y-3">
          {session.renames.map((rename, index) => (
            <li
              key={`${rename.recordedAtUnixMs}:${index}`}
              className="grid gap-1 sm:grid-cols-[minmax(0,1fr)_auto]"
            >
              <div className="min-w-0">
                <p className="break-words text-meta font-medium">
                  {rename.name}
                </p>
                <p className="mt-0.5 whitespace-pre-wrap text-micro leading-5 text-muted-foreground">
                  {rename.note}
                </p>
              </div>
              <time className="text-micro text-muted-foreground">
                {relativeTime(rename.recordedAtUnixMs)}
              </time>
            </li>
          ))}
        </ol>
        {session.omittedRenames > 0 && (
          <p className="mt-3 border-t border-border pt-3 text-micro text-muted-foreground">
            {session.omittedRenames} older renames are preserved in the session
            log but omitted from this bounded view.
          </p>
        )}
      </div>
    </section>
  );
}

function SessionOutcome({ session }: { session: SessionContext }) {
  if (!session.finish) return null;
  return (
    <section aria-labelledby="session-outcome-title">
      <SectionLabel
        id="session-outcome-title"
        icon={CheckCircle2}
        label="Outcome & handoff"
      />
      <div className="mt-2 border-y border-border py-4">
        <p className="text-meta leading-5 text-muted-foreground-strong">
          {session.finish.summary}
        </p>
        {session.finish.handoff && (
          <p className="mt-3 rounded-md bg-primary/7 px-3 py-2 text-meta leading-5">
            <span className="font-medium text-primary">Handoff:</span>{" "}
            {session.finish.handoff}
          </p>
        )}
        {session.finish.finalResponse && (
          <details className="mt-3 border-t border-border pt-3">
            <summary className="cursor-pointer rounded text-micro font-medium text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary">
              Final response
            </summary>
            <p className="mt-2 whitespace-pre-wrap text-micro leading-5 text-muted-foreground-strong">
              {session.finish.finalResponse}
            </p>
          </details>
        )}
        {session.finish.unresolved.length > 0 && (
          <MemoryList
            title="Still unresolved"
            items={session.finish.unresolved}
            tone="warning"
          />
        )}
      </div>
    </section>
  );
}

function SessionLocalData({
  erasing,
  onToggleErasing,
}: {
  erasing: boolean;
  onToggleErasing: () => void;
}) {
  return (
    <section
      aria-labelledby="session-local-data-title"
      className="flex flex-col gap-3 border-t border-border pt-5 sm:flex-row sm:items-center sm:justify-between"
    >
      <div className="max-w-2xl">
        <h3 id="session-local-data-title" className="text-meta font-semibold">
          Local session data
        </h3>
        <p className="mt-1 text-micro leading-5 text-muted-foreground">
          Forget this private session and every lesson derived from it without
          removing unrelated project memory.
        </p>
      </div>
      <Button
        size="sm"
        variant="outline"
        className="shrink-0 text-destructive hover:border-destructive/35 hover:bg-destructive/8"
        aria-expanded={erasing}
        aria-controls="session-erasure-panel"
        onClick={onToggleErasing}
      >
        <Trash2 size={13} aria-hidden="true" />
        Erase session memory…
      </Button>
    </section>
  );
}

function SessionCheckpointTimeline({
  session,
  onEvidence,
}: {
  session: SessionContext;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
}) {
  return (
    <section aria-labelledby="checkpoint-timeline-title">
      <SectionLabel
        id="checkpoint-timeline-title"
        icon={History}
        label="Checkpoint timeline"
      />
      <div className="mt-3 divide-y divide-border border-y border-border">
        {session.checkpoints.length === 0 ? (
          <CompactEmpty
            icon={History}
            title="No checkpoints yet"
            body="This session has started but has not recorded structured progress."
          />
        ) : (
          session.checkpoints.map((checkpoint, index) => (
            <SessionCheckpointCard
              key={checkpoint.checkpointId}
              checkpoint={checkpoint}
              index={index}
              revisionFreshness={session.revisionFreshness}
              onEvidence={onEvidence}
            />
          ))
        )}
      </div>
    </section>
  );
}

function SessionCheckpointCard({
  checkpoint,
  index,
  revisionFreshness,
  onEvidence,
}: {
  checkpoint: SessionContext["checkpoints"][number];
  index: number;
  revisionFreshness: ProjectRevisionFreshness;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
}) {
  return (
    <article className="relative py-4 sm:py-5">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="text-meta font-semibold text-primary">
          Checkpoint {index + 1}
        </span>
        <time className="text-micro text-muted-foreground">
          {relativeTime(checkpoint.recordedAtUnixMs)}
        </time>
      </div>
      <p className="mt-2 whitespace-pre-wrap text-body leading-6">
        {checkpoint.summary}
      </p>

      <div className="mt-4 grid gap-3 lg:grid-cols-2">
        {checkpoint.decisions.length > 0 && (
          <RecordGroup
            icon={GitBranch}
            title="Decisions"
            count={checkpoint.decisions.length}
          >
            {checkpoint.decisions.map((decision) => (
              <RecordItem
                key={decision.id}
                title={decision.title}
                body={decision.decision}
              />
            ))}
          </RecordGroup>
        )}
        {checkpoint.tasks.length > 0 && (
          <RecordGroup
            icon={BookCheck}
            title="Tasks"
            count={checkpoint.tasks.length}
          >
            {checkpoint.tasks.map((task) => (
              <RecordItem
                key={task.id}
                title={task.title}
                meta={humanize(task.status)}
              />
            ))}
          </RecordGroup>
        )}
        {checkpoint.problems.length > 0 && (
          <RecordGroup
            icon={AlertTriangle}
            title="Problems & outcomes"
            count={checkpoint.problems.length}
          >
            {checkpoint.problems.map((problem) => (
              <ProblemItem key={problem.id} problem={problem} />
            ))}
          </RecordGroup>
        )}
        {checkpoint.verification.length > 0 && (
          <RecordGroup
            icon={ShieldCheck}
            title="Verification"
            count={checkpoint.verification.length}
          >
            {checkpoint.verification.map((verification) => (
              <VerificationItem
                key={verification.id}
                verification={verification}
                onEvidence={onEvidence}
              />
            ))}
          </RecordGroup>
        )}
      </div>

      {checkpoint.projectRevision && (
        <ProjectRevisionButton
          revision={checkpoint.projectRevision}
          applicability={checkpoint.revisionApplicability}
          freshness={revisionFreshness}
        />
      )}

      {checkpoint.touchedArtifacts.length > 0 && (
        <div className="mt-4 border-t border-border pt-4">
          <p className="text-micro font-medium text-muted-foreground">
            Cited artifacts
          </p>
          <div className="mt-2 flex flex-wrap gap-1.5">
            {checkpoint.touchedArtifacts.map((artifact) => (
              <button
                type="button"
                key={`${artifact.artifactPath}:${artifact.startLine}`}
                title={
                  artifact.mediaType
                    ? `Original ${artifact.mediaType} evidence · snapshot ${artifact.artifactSnapshotId}`
                    : `${artifact.artifactPath}:${artifact.startLine}-${artifact.endLine}`
                }
                onClick={() => onEvidence(artifact)}
                className="max-w-full touch-manipulation truncate rounded-sm border border-border bg-surface-2 px-2 py-1 text-left font-mono text-micro text-muted-foreground outline-none transition-[transform,border-color,background-color,color] hover:border-primary/35 hover:bg-primary/7 hover:text-foreground active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
              >
                {artifact.artifactPath}
                {artifact.mediaType
                  ? ` · ${artifact.mediaType}`
                  : `:${artifact.startLine}`}
              </button>
            ))}
          </div>
        </div>
      )}

      {checkpoint.commands.length > 0 && (
        <details className="mt-4 border-t border-border pt-4">
          <summary className="cursor-pointer rounded text-micro font-medium text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary">
            Commands · {checkpoint.commands.length}
          </summary>
          <div className="mt-2 space-y-2">
            {checkpoint.commands.map((command) => (
              <div
                key={command.id}
                className="overflow-hidden rounded-md border border-border bg-background/45"
              >
                <code className="block overflow-x-auto px-3 py-2 font-mono text-micro text-foreground">
                  {command.command}
                </code>
                {(command.summary || command.exitCode !== undefined) && (
                  <p className="border-t border-border px-3 py-2 text-micro text-muted-foreground">
                    {command.exitCode !== undefined
                      ? `Exit ${command.exitCode}`
                      : "Exit not recorded"}
                    {command.summary ? ` · ${command.summary}` : ""}
                  </p>
                )}
              </div>
            ))}
          </div>
        </details>
      )}

      {checkpoint.unresolved.length > 0 && (
        <MemoryList
          title="Unresolved at this checkpoint"
          items={checkpoint.unresolved}
          tone="warning"
        />
      )}
    </article>
  );
}

function SessionInspectorHeader({
  session,
  renaming,
  onToggleRenaming,
}: {
  session: SessionContext | null;
  renaming: boolean;
  onToggleRenaming: () => void;
}) {
  return (
    <div className="flex shrink-0 items-center justify-between gap-3 border-b border-border px-4 py-3 sm:px-5">
      <div className="min-w-0">
        <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
          Session provenance
        </p>
        <Dialog.Title className="mt-0.5 truncate text-body font-semibold">
          {session?.name ?? "Loading session…"}
        </Dialog.Title>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        {session && (
          <Button
            size="sm"
            variant={renaming ? "outline" : "ghost"}
            className="h-8"
            aria-expanded={renaming}
            onClick={onToggleRenaming}
          >
            <PencilLine size={13} aria-hidden="true" />
            Rename
          </Button>
        )}
        <Dialog.Close
          className="rounded-md p-1.5 text-muted-foreground hover:bg-surface-3 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
          aria-label="Close session inspector"
        >
          <X size={15} />
        </Dialog.Close>
      </div>
    </div>
  );
}

function ProjectRevisionButton({
  revision,
  applicability,
  freshness,
}: {
  revision: NonNullable<
    SessionContext["checkpoints"][number]["projectRevision"]
  >;
  applicability?: RevisionApplicability;
  freshness: ProjectRevisionFreshness;
}) {
  const shortHead = revision.head?.slice(0, 10);
  const currentHead = freshness.currentHead?.slice(0, 10);
  const changeLabel =
    revision.trackedChanges === 0
      ? "clean tracked tree"
      : `${revision.trackedChanges.toLocaleString()} tracked change${revision.trackedChanges === 1 ? "" : "s"}`;
  const currentChangeLabel =
    freshness.trackedWorktreeChanges === undefined
      ? "tracked status unavailable"
      : freshness.trackedWorktreeChanges === 0
        ? "clean tracked tree"
        : `${freshness.trackedWorktreeChanges.toLocaleString()} tracked change${freshness.trackedWorktreeChanges === 1 ? "" : "s"}`;
  const compatibility = applicability?.compatibility;
  return (
    <div className="mt-4 border-t border-border pt-4">
      <p className="text-micro font-medium text-muted-foreground">
        Captured Project Revision
      </p>
      <div className="mt-2 flex w-full items-center gap-3 rounded-md border border-border bg-surface-2 px-3 py-2.5 text-left">
        <span className="grid size-8 shrink-0 place-items-center rounded-md bg-primary/10 text-primary">
          <GitBranch size={16} aria-hidden="true" />
        </span>
        <span className="min-w-0 flex-1">
          <span
            className="block truncate font-mono text-meta font-medium text-foreground"
            translate="no"
          >
            {shortHead ?? "Snapshot only"}
            {revision.branch ? ` · ${revision.branch}` : ""}
          </span>
          <span className="mt-0.5 block text-micro text-muted-foreground">
            Captured {absoluteTime(revision.capturedAtUnixMs)} · {changeLabel}
          </span>
        </span>
      </div>
      <div className="mt-2 rounded-md border border-border bg-background/45 px-3 py-2.5">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-micro font-medium text-muted-foreground">
            Current applicability
          </span>
          <span
            className={cn(
              "rounded-sm px-1.5 py-0.5 text-micro font-medium",
              compatibility === "divergent" || compatibility === "unknown"
                ? "bg-warning/10 text-warning"
                : "bg-success/10 text-success",
            )}
          >
            {revisionCompatibilityLabel(compatibility ?? "unknown")}
          </span>
        </div>
        <p className="mt-1.5 text-micro text-muted-foreground">
          {freshness.liveGitChecked
            ? `Current Git: ${freshness.currentBranch ?? "detached HEAD"}${currentHead ? ` · ${currentHead}` : ""} · ${currentChangeLabel}.`
            : "Current Git metadata is unavailable."}
        </p>
        <p className="mt-1 text-micro text-muted-foreground">
          Git metadata only. Live file contents were not checked.
        </p>
      </div>
    </div>
  );
}

function revisionCompatibilityLabel(value: RevisionCompatibility): string {
  switch (value) {
    case "current-lineage":
      return "Current lineage";
    case "ancestor":
      return "Ancestor";
    case "merged":
      return "Merged";
    case "divergent":
      return "Divergent";
    case "unknown":
      return "Unknown";
  }
}
