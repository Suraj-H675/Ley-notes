import { lazy, Suspense, useEffect, useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import {
  AlertTriangle,
  ArrowRight,
  BookCheck,
  BrainCircuit,
  Check,
  CheckCircle2,
  ChevronRight,
  CircleDot,
  MessageSquareWarning,
  PencilLine,
  RefreshCw,
  RotateCcw,
  X,
  XCircle,
} from "lucide-react";
import { Button } from "@/shared/components/Button";
import { cn } from "@/shared/lib/classnames";
import { readAgentLearning, reviewAgentLearning } from "./api";
import {
  absoluteTime,
  ErrorNotice,
  errorMessage,
  humanize,
  LargeEmpty,
  PageHeading,
  relativeTime,
  StatusPill,
  TrustDot,
} from "./AgentMemoryPresentation";
import type {
  AgentMemoryDashboard,
  ArtifactEvidenceReference,
  LearningAction,
  LearningContext,
  LearningSummary,
} from "./types";

const LearningCorrectionEditor = lazy(() =>
  import("./LearningCorrectionEditor").then((module) => ({
    default: module.LearningCorrectionEditor,
  })),
);

export function Lessons({
  dashboard,
  onLearning,
}: {
  dashboard: AgentMemoryDashboard;
  onLearning: (id: string) => void;
}) {
  const learnings = dashboard.allLearnings.learnings;
  return (
    <section aria-labelledby="lessons-title">
      <PageHeading
        eyebrow="Procedural memory"
        title="Lessons"
        description={`Evidence-backed guidance remains reviewable, temporal, and separate from ordinary notes. Showing ${learnings.length} of ${dashboard.allLearnings.totalMatching}.`}
      />
      <div className="mt-6 grid gap-3 lg:grid-cols-2">
        {learnings.length === 0 ? (
          <div className="lg:col-span-2">
            <LargeEmpty
              icon={BookCheck}
              title="No lessons proposed"
              body="Agents can propose learnings from cited session records. Nothing becomes trusted until evidence or your explicit confirmation supports it."
            />
          </div>
        ) : (
          learnings.map((learning) => (
            <LearningCard
              key={learning.learningId}
              learning={learning}
              onClick={() => onLearning(learning.learningId)}
            />
          ))
        )}
      </div>
    </section>
  );
}

export function ReviewInbox({
  dashboard,
  onLearning,
}: {
  dashboard: AgentMemoryDashboard;
  onLearning: (id: string) => void;
}) {
  const inbox = dashboard.reviewInbox;
  return (
    <section aria-labelledby="review-inbox-title">
      <PageHeading
        eyebrow="Human authority"
        title="Review inbox"
        description={`Confirm useful guidance, contest uncertain claims, reject false memory, or mark guidance stale. Showing ${inbox.learnings.length} of ${inbox.totalMatching}.`}
      />
      {inbox.learnings.length === 0 ? (
        <div className="mt-6">
          <LargeEmpty
            icon={CheckCircle2}
            title="You’re all caught up"
            body="No proposed, contested, source-changed, or stale lessons need review."
          />
        </div>
      ) : (
        <div className="mt-6 divide-y divide-border border-y border-border">
          {inbox.learnings.map((learning) => (
            <LearningCard
              key={learning.learningId}
              learning={learning}
              onClick={() => onLearning(learning.learningId)}
              wide
            />
          ))}
        </div>
      )}
    </section>
  );
}

export function LearningInspector({
  learningId,
  projectPath,
  candidates,
  candidatesOmitted,
  onClose,
  onLearning,
  onSession,
  onEvidence,
  onReviewed,
}: {
  learningId: string | null;
  projectPath: string;
  candidates: LearningSummary[];
  candidatesOmitted: number;
  onClose: () => void;
  onLearning: (learningId: string) => void;
  onSession: (sessionId: string) => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
  onReviewed: (dashboard: AgentMemoryDashboard) => void;
}) {
  const [learning, setLearning] = useState<LearningContext | null>(null);
  const [action, setAction] = useState<LearningAction | null>(null);
  const [note, setNote] = useState("");
  const [replacementLearningId, setReplacementLearningId] = useState("");
  const [correcting, setCorrecting] = useState(false);
  const [busy, setBusy] = useState(Boolean(learningId));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!learningId) return;
    let current = true;
    void readAgentLearning(projectPath, learningId)
      .then((next) => {
        if (current) setLearning(next);
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
  }, [learningId, projectPath]);

  const noteRequired =
    action === "contest" ||
    action === "reject" ||
    action === "mark-stale" ||
    action === "supersede";
  const selectedReplacement =
    action === "supersede"
      ? candidates.find(
          (candidate) => candidate.learningId === replacementLearningId,
        )
      : undefined;
  const canSubmit =
    action &&
    (!noteRequired || note.trim().length > 0) &&
    (action !== "supersede" || selectedReplacement !== undefined);
  const terminal =
    learning?.state === "rejected" || learning?.state === "superseded";
  const replacementSummary =
    learning?.supersededBy === undefined
      ? undefined
      : candidates.find(
          (candidate) => candidate.learningId === learning.supersededBy,
        );

  async function submitReview() {
    if (!learningId || !learning || !action || !canSubmit) return;
    setBusy(true);
    setError(null);
    try {
      const dashboard = await reviewAgentLearning(
        projectPath,
        learningId,
        learning.eventCount,
        action,
        note.trim(),
        action === "supersede" ? replacementLearningId : null,
        selectedReplacement?.eventCount ?? null,
      );
      onReviewed(dashboard);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  function beginCorrection() {
    if (!learning) return;
    setAction(null);
    setNote("");
    setReplacementLearningId("");
    setCorrecting(true);
    setError(null);
  }

  return (
    <Dialog.Root
      open={Boolean(learningId)}
      onOpenChange={(next) => {
        if (!next) onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="app-modal-overlay fixed inset-0 z-[80]" />
        <Dialog.Content
          className="app-modal-surface fixed inset-x-3 bottom-3 top-3 z-[81] mx-auto flex max-w-3xl flex-col overflow-hidden rounded-sm border outline-none focus-visible:ring-2 focus-visible:ring-primary sm:inset-x-6 sm:bottom-6 sm:top-6"
          aria-describedby={undefined}
        >
          <div className="flex shrink-0 items-center justify-between border-b border-border px-4 py-3 sm:px-5">
            <div>
              <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
                Provenance inspector
              </p>
              <Dialog.Title className="mt-0.5 text-body font-semibold">
                {learning?.title ?? "Loading learning…"}
              </Dialog.Title>
            </div>
            <Dialog.Close
              className="rounded-md p-1.5 text-muted-foreground hover:bg-surface-3 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
              aria-label="Close learning inspector"
            >
              <X size={15} />
            </Dialog.Close>
          </div>
          <LearningInspectorBody
            learning={learning}
            busy={busy}
            error={error}
            replacementSummary={replacementSummary}
            onLearning={onLearning}
            onSession={onSession}
            onEvidence={onEvidence}
          />
          <LearningInspectorFooter
            learning={learning}
            terminal={terminal}
            correcting={correcting}
            action={action}
            note={note}
            replacementLearningId={replacementLearningId}
            candidates={candidates}
            candidatesOmitted={candidatesOmitted}
            noteRequired={noteRequired}
            canSubmit={Boolean(canSubmit)}
            busy={busy}
            error={error}
            projectPath={projectPath}
            onReviewed={onReviewed}
            onBeginCorrection={beginCorrection}
            onSetAction={setAction}
            onSetNote={setNote}
            onSetReplacementLearningId={setReplacementLearningId}
            onSetError={setError}
            onSetCorrecting={setCorrecting}
            onSubmitReview={submitReview}
          />
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function LearningInspectorBody({
  learning,
  busy,
  error,
  replacementSummary,
  onLearning,
  onSession,
  onEvidence,
}: {
  learning: LearningContext | null;
  busy: boolean;
  error: string | null;
  replacementSummary?: LearningSummary;
  onLearning: (learningId: string) => void;
  onSession: (sessionId: string) => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
}) {
  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-4 sm:p-6">
      {busy && !learning ? (
        <div className="py-20 text-center text-meta text-muted-foreground">
          Reading cited memory…
        </div>
      ) : error && !learning ? (
        <ErrorNotice message={error} />
      ) : learning ? (
        <div className="space-y-6">
          <LearningOverview
            learning={learning}
            replacementSummary={replacementSummary}
            onLearning={onLearning}
          />
          <LearningOriginLineage learning={learning} onSession={onSession} />
          <LearningEvidence
            learning={learning}
            onSession={onSession}
            onEvidence={onEvidence}
          />
          <LearningApplications learning={learning} onSession={onSession} />
          <LearningHistory learning={learning} />
          {learning.claimTruncated && (
            <p className="rounded-md border border-warning/25 bg-warning/8 p-3 text-micro text-muted-foreground">
              The title or guidance was truncated to keep this inspector
              bounded. Use the CLI for the complete claim before correcting or
              reviewing it.
            </p>
          )}
          <p className="border-l-2 border-secondary/45 pl-3 text-micro leading-5 text-muted-foreground">
            <MessageSquareWarning
              size={13}
              className="mr-2 inline text-secondary"
            />
            {learning.instructionWarning}
          </p>
        </div>
      ) : null}
    </div>
  );
}

function LearningOverview({
  learning,
  replacementSummary,
  onLearning,
}: {
  learning: LearningContext;
  replacementSummary?: LearningSummary;
  onLearning: (learningId: string) => void;
}) {
  return (
    <>
      <div className="flex flex-wrap gap-2">
        <StatusPill
          tone={learning.trustedForReuse ? "success" : "warning"}
          label={humanize(learning.trustState)}
          icon={learning.trustedForReuse ? CheckCircle2 : AlertTriangle}
        />
        <StatusPill
          tone="neutral"
          label={humanize(learning.provenance)}
          icon={BrainCircuit}
        />
        <StatusPill
          tone="neutral"
          label={`${learning.confidencePercent}% confidence`}
          icon={CircleDot}
        />
        <StatusPill
          tone={learning.freshness === "current" ? "success" : "warning"}
          label={humanize(learning.freshness)}
          icon={RefreshCw}
        />
      </div>
      <section>
        <h3 className="text-meta font-semibold text-muted-foreground">
          Guidance
        </h3>
        <p className="mt-2 whitespace-pre-wrap break-words text-body leading-6 text-foreground">
          {learning.guidance}
        </p>
      </section>
      <section>
        <h3 className="text-meta font-semibold text-muted-foreground">
          Version timeline
        </h3>
        <dl className="mt-2 grid border-y border-border text-meta sm:grid-cols-3 sm:divide-x sm:divide-border">
          <div className="py-3 sm:px-3 sm:first:pl-0">
            <dt className="text-micro text-muted-foreground">Created</dt>
            <dd className="mt-0.5 font-medium">
              <time
                dateTime={isoTime(learning.createdAtUnixMs)}
                title={absoluteTime(learning.createdAtUnixMs)}
              >
                {relativeTime(learning.createdAtUnixMs)}
              </time>
            </dd>
          </div>
          <div className="py-3 sm:px-3">
            <dt className="text-micro text-muted-foreground">
              Current version
            </dt>
            <dd className="mt-0.5 font-medium">
              <time
                dateTime={isoTime(learning.validFromUnixMs)}
                title={absoluteTime(learning.validFromUnixMs)}
              >
                {relativeTime(learning.validFromUnixMs)}
              </time>
            </dd>
          </div>
          <div className="py-3 sm:px-3 sm:last:pr-0">
            <dt className="text-micro text-muted-foreground">Ledger</dt>
            <dd className="mt-0.5 font-medium">
              {learning.eventCount} immutable{" "}
              {learning.eventCount === 1 ? "event" : "events"}
            </dd>
          </div>
        </dl>
        {learning.validUntilUnixMs !== undefined && (
          <p className="mt-2 text-micro text-muted-foreground">
            This version stopped being valid{" "}
            <time
              dateTime={isoTime(learning.validUntilUnixMs)}
              title={absoluteTime(learning.validUntilUnixMs)}
            >
              {relativeTime(learning.validUntilUnixMs)}
            </time>
            .
          </p>
        )}
        {learning.supersededBy && (
          <p className="mt-2 text-micro text-muted-foreground">
            Superseded by{" "}
            {replacementSummary ? (
              <button
                type="button"
                onClick={() => onLearning(replacementSummary.learningId)}
                className="font-medium text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
              >
                {replacementSummary.title} ·{" "}
                {humanize(replacementSummary.state)} ·{" "}
                {compactId(replacementSummary.learningId)}
              </button>
            ) : (
              <span className="font-mono text-foreground">
                {learning.supersededBy}
              </span>
            )}
            .
          </p>
        )}
      </section>
    </>
  );
}

function LearningEvidence({
  learning,
  onSession,
  onEvidence,
}: {
  learning: LearningContext;
  onSession: (sessionId: string) => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
}) {
  return (
    <section>
      <h3 className="text-meta font-semibold text-muted-foreground">
        Evidence · {learning.evidenceCount}
      </h3>
      <div className="mt-2 divide-y divide-border border-y border-border">
        {learning.evidence.length === 0 ? (
          <p className="text-meta text-muted-foreground">
            No evidence is available.
          </p>
        ) : (
          learning.evidence.map((evidence) => (
            <div
              key={`${evidence.sessionId}:${evidence.recordId}`}
              className="py-3"
            >
              <div className="flex flex-wrap items-center justify-between gap-2 text-micro text-muted-foreground">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium text-muted-foreground-strong">
                    {humanize(evidence.recordType)}
                  </span>
                  <span>{relativeTime(evidence.sessionUpdatedAtUnixMs)}</span>
                </div>
                <button
                  type="button"
                  onClick={() => onSession(evidence.sessionId)}
                  className="touch-manipulation rounded font-semibold text-primary outline-none transition-transform hover:underline active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
                >
                  Open session
                </button>
              </div>
              {evidence.note && (
                <p className="mt-2 text-meta leading-5 text-muted-foreground-strong">
                  {evidence.note}
                </p>
              )}
              {evidence.artifacts.length > 0 && (
                <div className="mt-2 flex flex-wrap gap-1.5">
                  {evidence.artifacts.map((artifact) => (
                    <button
                      type="button"
                      key={`${artifact.artifactPath}:${artifact.startLine}`}
                      title={
                        artifact.mediaType
                          ? `Original ${artifact.mediaType} evidence · snapshot ${artifact.artifactSnapshotId}`
                          : `${artifact.artifactPath}:${artifact.startLine}-${artifact.endLine} · snapshot ${artifact.artifactSnapshotId}`
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
              )}
            </div>
          ))
        )}
      </div>
      {learning.omittedEvidence > 0 && (
        <p className="mt-2 text-micro text-muted-foreground">
          {learning.omittedEvidence} older evidence{" "}
          {learning.omittedEvidence === 1 ? "reference is" : "references are"}{" "}
          omitted from this bounded inspector.
        </p>
      )}
    </section>
  );
}

function LearningOriginLineage({
  learning,
  onSession,
}: {
  learning: LearningContext;
  onSession: (sessionId: string) => void;
}) {
  return (
    <section>
      <h3 className="text-meta font-semibold text-muted-foreground">
        Origin lineage · {learning.originSourceCount}
      </h3>
      <dl className="mt-2 grid border-y border-border text-meta sm:grid-cols-3 sm:divide-x sm:divide-border">
        <div className="py-3 sm:px-3 sm:first:pl-0">
          <dt className="text-micro text-muted-foreground">Resolution</dt>
          <dd className="mt-0.5 font-medium">
            {learning.originLineage.mechanicallyResolved
              ? "Mechanically resolved"
              : "Incomplete lineage"}
          </dd>
        </div>
        <div className="py-3 sm:px-3">
          <dt className="text-micro text-muted-foreground">
            Causal completeness
          </dt>
          <dd className="mt-0.5 font-medium">
            {learning.originLineage.causalCompletenessProven
              ? "Proven"
              : "Not proven"}
          </dd>
        </div>
        <div className="py-3 sm:px-3 sm:last:pr-0">
          <dt className="text-micro text-muted-foreground">
            Automatic authority ceiling
          </dt>
          <dd className="mt-0.5 font-medium">
            {humanize(learning.originLineage.automaticAuthorityCeiling)}
          </dd>
        </div>
      </dl>
      {learning.originLineage.sources.length > 0 && (
        <div className="mt-2 divide-y divide-border border-y border-border">
          {learning.originLineage.sources.map((source, index) => {
            const key = learningOriginSourceKey(source, index);
            return (
              <div
                key={key}
                className="flex flex-wrap items-center justify-between gap-2 py-3 text-micro"
              >
                <div className="min-w-0">
                  <span className="font-medium text-muted-foreground-strong">
                    {humanize(source.kind)}
                  </span>
                  <p className="mt-0.5 break-all font-mono text-muted-foreground">
                    {learningOriginSourceHandle(source)}
                  </p>
                </div>
                {source.kind === "captured-artifact" ? (
                  <span
                    className="text-micro font-medium text-muted-foreground"
                    title="This lineage row is an exact provenance identity, not a complete text/media read citation. Open the matching evidence reference below when available."
                  >
                    Provenance handle
                  </span>
                ) : (
                  <button
                    type="button"
                    onClick={() => onSession(source.sessionId)}
                    className="touch-manipulation rounded font-semibold text-primary outline-none transition-transform hover:underline active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
                  >
                    Open session
                  </button>
                )}
              </div>
            );
          })}
        </div>
      )}
      {learning.omittedOriginSources > 0 && (
        <p className="mt-2 text-micro text-muted-foreground">
          {learning.omittedOriginSources} origin sources are outside this
          bounded view.
        </p>
      )}
      <p className="mt-2 text-micro leading-5 text-muted-foreground">
        Lineage records where this learning came from. It does not increase
        authority or prove that the recorded sources are causally complete.
      </p>
    </section>
  );
}

function learningOriginSourceKey(
  source: LearningContext["originLineage"]["sources"][number],
  index: number,
): string {
  switch (source.kind) {
    case "session-record":
      return `${source.kind}:${source.sessionId}:${source.recordId}`;
    case "captured-artifact":
      return `${source.kind}:${source.artifactSnapshotId}:${source.artifactPath}`;
    case "turn-evidence":
      return `${source.kind}:${source.sessionId}:${source.recordId}`;
    case "tool-evidence":
      return `${source.kind}:${source.sessionId}:${source.recordId}`;
    case "recovery-candidate":
      return `${source.kind}:${source.sessionId}:${source.candidateFingerprint}`;
    default:
      return `origin:${index}`;
  }
}

function learningOriginSourceHandle(
  source: LearningContext["originLineage"]["sources"][number],
): string {
  switch (source.kind) {
    case "session-record":
      return `${source.sessionId} · ${source.recordType} · ${source.recordId}`;
    case "captured-artifact":
      return `${source.artifactPath} · ${source.artifactSnapshotId}`;
    case "turn-evidence":
      return `${source.sessionId} · ${source.recordId}`;
    case "tool-evidence":
      return `${source.sessionId} · ${source.recordId}`;
    case "recovery-candidate":
      return `${source.sessionId} · ${source.candidateFingerprint}`;
  }
}

function LearningHistory({ learning }: { learning: LearningContext }) {
  if (learning.history.length === 0) return null;
  return (
    <section>
      <h3 className="text-meta font-semibold text-muted-foreground">
        Review history
      </h3>
      <ol className="mt-2 space-y-2">
        {learning.history.map((entry) => (
          <li key={entry.eventId} className="flex gap-3 text-meta">
            <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-border-strong" />
            <span className="min-w-0 break-words">
              <span className="font-medium">{humanize(entry.action)}</span> by{" "}
              {humanize(entry.actor)} ·{" "}
              <time
                dateTime={isoTime(entry.recordedAtUnixMs)}
                title={absoluteTime(entry.recordedAtUnixMs)}
              >
                {relativeTime(entry.recordedAtUnixMs)}
              </time>
              {entry.note ? ` — ${entry.note}` : ""}
            </span>
          </li>
        ))}
      </ol>
      {learning.omittedHistory > 0 && (
        <p className="mt-2 text-micro text-muted-foreground">
          Showing {learning.history.length} of {learning.historyCount} immutable
          history events.
        </p>
      )}
    </section>
  );
}

function LearningApplications({
  learning,
  onSession,
}: {
  learning: LearningContext;
  onSession: (sessionId: string) => void;
}) {
  if (learning.applicationObservations.length === 0) return null;

  return (
    <section>
      <h3 className="text-meta font-semibold text-muted-foreground">
        Procedure application history · {learning.applicationObservationCount}
      </h3>
      <div className="mt-2 divide-y divide-border border-y border-border">
        {learning.applicationObservations.map((application) => (
          <article
            key={application.observationId}
            className="py-3"
          >
            <div className="flex flex-wrap items-center justify-between gap-2 text-micro text-muted-foreground">
              <div className="flex flex-wrap items-center gap-2">
                <span
                  className={cn(
                    "rounded px-1.5 py-0.5 font-medium",
                    application.learningVersionMatchesCurrent
                      ? "bg-success/10 text-success"
                      : "bg-warning/10 text-warning",
                  )}
                >
                  {application.learningVersionMatchesCurrent
                    ? "Exact current version"
                    : "Older learning version"}
                </span>
                <time>{relativeTime(application.recordedAtUnixMs)}</time>
              </div>
              <button
                type="button"
                onClick={() => onSession(application.sessionId)}
                className="touch-manipulation rounded font-semibold text-primary outline-none transition-transform hover:underline active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
              >
                Open session
              </button>
            </div>
            {application.taskExcerpt && (
              <p className="mt-2 text-meta leading-5 text-muted-foreground-strong">
                {application.taskExcerpt}
              </p>
            )}
            <p className="mt-2 text-micro leading-5 text-muted-foreground">
              Typed verification outcomes · {application.passedVerifications}{" "}
              passed · {application.failedVerifications} failed ·{" "}
              {application.skippedVerifications} skipped ·{" "}
              {application.unknownVerifications} unknown
            </p>
            <p className="mt-2 text-micro leading-5 text-muted-foreground">
              Caller-declared application only. Procedure following, condition
              applicability, context usage, and causation remain unproven; no
              trust or ranking change was applied.
            </p>
          </article>
        ))}
      </div>
      {learning.omittedApplicationObservations > 0 && (
        <p className="mt-2 text-micro text-muted-foreground">
          {learning.omittedApplicationObservations} older application
          observations are outside this bounded view.
        </p>
      )}
      <p className="mt-2 text-micro leading-5 text-muted-foreground">
        {learning.applicationClaimNotice}
      </p>
    </section>
  );
}

function LearningInspectorFooter({
  learning,
  terminal,
  correcting,
  action,
  note,
  replacementLearningId,
  candidates,
  candidatesOmitted,
  noteRequired,
  canSubmit,
  busy,
  error,
  projectPath,
  onReviewed,
  onBeginCorrection,
  onSetAction,
  onSetNote,
  onSetReplacementLearningId,
  onSetError,
  onSetCorrecting,
  onSubmitReview,
}: {
  learning: LearningContext | null;
  terminal: boolean;
  correcting: boolean;
  action: LearningAction | null;
  note: string;
  replacementLearningId: string;
  candidates: LearningSummary[];
  candidatesOmitted: number;
  noteRequired: boolean;
  canSubmit: boolean;
  busy: boolean;
  error: string | null;
  projectPath: string;
  onReviewed: (dashboard: AgentMemoryDashboard) => void;
  onBeginCorrection: () => void;
  onSetAction: (action: LearningAction | null) => void;
  onSetNote: (note: string) => void;
  onSetReplacementLearningId: (learningId: string) => void;
  onSetError: (error: string | null) => void;
  onSetCorrecting: (correcting: boolean) => void;
  onSubmitReview: () => Promise<void>;
}) {
  if (!learning) return null;
  const replacementCandidates = candidates.filter(
    (candidate) =>
      candidate.learningId !== learning.learningId &&
      candidate.state !== "rejected" &&
      candidate.state !== "superseded",
  );
  return (
    <div className="shrink-0 border-t border-border bg-surface-1 p-4 sm:p-5">
      {learning.claimTruncated ? (
        <p className="text-meta text-muted-foreground">
          This bounded view omits part of the claim. Inspect the complete CLI
          projection before correcting or reviewing it.
        </p>
      ) : terminal ? (
        <p className="text-meta text-muted-foreground">
          This {humanize(learning.state)} learning is preserved as terminal
          history. Create a new learning rather than rewriting it.
        </p>
      ) : correcting ? (
        <Suspense
          fallback={
            <p className="py-4 text-center text-meta text-muted-foreground">
              Loading correction editor…
            </p>
          }
        >
          <LearningCorrectionEditor
            projectPath={projectPath}
            learning={learning}
            onCancel={() => {
              onSetCorrecting(false);
              onSetError(null);
            }}
            onCorrected={onReviewed}
          />
        </Suspense>
      ) : !action ? (
        <div className="flex flex-wrap items-center gap-2">
          <span className="mr-auto text-meta font-medium">Your decision</span>
          <Button size="sm" variant="outline" onClick={onBeginCorrection}>
            <PencilLine size={13} />
            Correct
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => onSetAction("mark-stale")}
          >
            <RotateCcw size={13} />
            Mark stale
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => onSetAction("contest")}
          >
            <AlertTriangle size={13} />
            Contest
          </Button>
          <Button
            size="sm"
            variant="destructive"
            onClick={() => onSetAction("reject")}
          >
            <XCircle size={13} />
            Reject
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={replacementCandidates.length === 0}
            title={
              replacementCandidates.length === 0
                ? "No non-terminal replacement is available in this bounded Desktop list. Use the CLI if another learning exists."
                : undefined
            }
            onClick={() => onSetAction("supersede")}
          >
            <ArrowRight size={13} />
            Supersede
          </Button>
          <Button
            size="sm"
            variant="primary"
            onClick={() => onSetAction("confirm")}
          >
            <Check size={13} />
            Confirm
          </Button>
        </div>
      ) : (
        <div>
          {action === "supersede" && (
            <label className="mb-3 block text-meta font-medium">
              <span className="block">Replacement learning · required</span>
              <select
                aria-label="Replacement learning"
                value={replacementLearningId}
                disabled={busy}
                onChange={(event) =>
                  onSetReplacementLearningId(event.target.value)
                }
                className="mt-2 w-full rounded-md border border-border bg-background/45 px-3 py-2 text-meta text-foreground outline-none focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-primary"
              >
                <option value="">Choose a replacement…</option>
                {replacementCandidates.map((candidate) => (
                  <option
                    key={candidate.learningId}
                    value={candidate.learningId}
                  >
                    {candidate.title} · {humanize(candidate.state)} ·{" "}
                    {humanize(candidate.trustState)} ·{" "}
                    {humanize(candidate.freshness)} ·{" "}
                    {compactId(candidate.learningId)}
                  </option>
                ))}
              </select>
              <span className="mt-1 block text-micro font-normal leading-4 text-muted-foreground">
                The old learning remains immutable history and points to the
                selected replacement. Ley still validates existence and
                supersession cycles before writing. This picker uses the bounded
                Desktop learning list; use the CLI if the intended replacement
                is not shown.
                {candidatesOmitted > 0
                  ? ` ${candidatesOmitted} additional ${candidatesOmitted === 1 ? "learning is" : "learnings are"} omitted from this list.`
                  : ""}
              </span>
            </label>
          )}
          <label
            htmlFor="learning-review-note"
            className="block text-meta font-medium"
          >
            {actionLabel(action)}
            <span className="ml-1 font-normal text-muted-foreground">
              {noteRequired ? "· note required" : "· note optional"}
            </span>
          </label>
          <textarea
            id="learning-review-note"
            name="learning-review-note"
            autoComplete="off"
            value={note}
            onChange={(event) => onSetNote(event.target.value)}
            placeholder={reviewPlaceholder(action)}
            rows={2}
            className="mt-2 w-full resize-none rounded-md border border-border bg-background/45 px-3 py-2 text-meta text-foreground outline-none placeholder:text-subtle-foreground focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-primary"
          />
          {error && (
            <p className="mt-2 text-micro text-destructive" role="alert">
              {error}
            </p>
          )}
          <div className="mt-3 flex justify-end gap-2">
            <Button
              size="sm"
              variant="ghost"
              disabled={busy}
              onClick={() => {
                onSetAction(null);
                onSetNote("");
                onSetReplacementLearningId("");
                onSetError(null);
              }}
            >
              Cancel
            </Button>
            <Button
              size="sm"
              variant={
                action === "reject" || action === "supersede"
                  ? "destructive"
                  : "primary"
              }
              disabled={busy || !canSubmit}
              onClick={() => void onSubmitReview()}
            >
              {busy ? "Saving…" : actionLabel(action)}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

function LearningCard({
  learning,
  onClick,
  wide = false,
}: {
  learning: LearningSummary;
  onClick: () => void;
  wide?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "group w-full px-1 py-4 text-left transition-colors hover:bg-surface-1/55 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary",
        wide && "sm:py-5",
      )}
    >
      <div className="flex items-start gap-3">
        <TrustDot learning={learning} />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-micro font-medium text-subtle-foreground">
              {humanize(learning.kind)}
            </span>
            <span className="text-micro text-muted-foreground">
              {humanize(learning.trustState)}
            </span>
            <span className="text-micro text-muted-foreground">
              · {learning.confidencePercent}%
            </span>
          </div>
          <h3 className="mt-2 text-body font-semibold">{learning.title}</h3>
          <p
            className={cn(
              "mt-1 text-meta leading-5 text-muted-foreground-strong",
              wide ? "line-clamp-3" : "line-clamp-2",
            )}
          >
            {learning.guidanceExcerpt}
          </p>
          <p className="mt-3 text-micro text-muted-foreground">
            {humanize(learning.provenance)} · {learning.corroboratingSessions}{" "}
            corroborating{" "}
            {learning.corroboratingSessions === 1 ? "session" : "sessions"} ·{" "}
            {relativeTime(learning.updatedAtUnixMs)}
          </p>
        </div>
        <ChevronRight
          size={15}
          className="mt-1 shrink-0 text-subtle-foreground group-hover:text-foreground"
        />
      </div>
    </button>
  );
}

function isoTime(unixMs: number): string {
  return new Date(unixMs).toISOString();
}

function compactId(value: string): string {
  return value.length <= 16 ? value : `${value.slice(0, 12)}…`;
}

function actionLabel(action: LearningAction): string {
  return action === "mark-stale" ? "Mark stale" : humanize(action);
}

function reviewPlaceholder(action: LearningAction): string {
  if (action === "confirm") return "Explain why this is useful or reliable…";
  if (action === "contest") return "Describe what is uncertain or conflicting…";
  if (action === "reject") return "Explain why agents should not reuse this…";
  if (action === "supersede")
    return "Explain why the replacement should be used instead…";
  return "Describe what changed or became outdated…";
}
