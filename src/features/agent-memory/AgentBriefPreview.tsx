import { useState, type FormEvent } from "react";
import {
  AlertTriangle,
  BrainCircuit,
  Cloud,
  Cpu,
  ExternalLink,
  FileCheck2,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import { Button } from "@/shared/components/Button";
import { previewAgentBrief } from "./api";
import type {
  AgentBriefPreview as AgentBriefPreviewResult,
  AgentEgressTarget,
  ArtifactEvidenceReference,
} from "./types";

export function AgentBriefPreview({
  projectPath,
  projectId,
  onEvidence,
}: {
  projectPath: string;
  projectId: string;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
}) {
  const [task, setTask] = useState("");
  const [target, setTarget] = useState<AgentEgressTarget>("cloud");
  const [preview, setPreview] = useState<AgentBriefPreviewResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function compile(event: FormEvent) {
    event.preventDefault();
    const normalized = task.trim();
    if (!normalized || busy) return;
    setBusy(true);
    setError(null);
    try {
      setPreview(
        await previewAgentBrief(projectPath, projectId, normalized, target),
      );
    } catch (cause) {
      setPreview(null);
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <header>
        <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
          What the next agent gets
        </p>
        <h2 className="mt-1 text-2xl font-semibold tracking-[-0.03em]">
          Agent brief preview
        </h2>
        <p className="mt-2 max-w-3xl text-body leading-6 text-muted-foreground-strong">
          Compile the same bounded, task-conditioned continuity pack exposed as{" "}
          <span className="font-mono text-meta">ley_brief</span>. Previewing
          does not start a session, refresh capture, change project content, or
          change egress policy. On a transitional older project, the canonical
          brief path may complete existing local compatibility-authority
          migration.
        </p>
      </header>

      <form
        onSubmit={(event) => void compile(event)}
        className="space-y-4 rounded-md border border-border bg-surface-1 p-4 shadow-panel sm:p-5"
      >
        <label className="block">
          <span className="text-meta font-semibold">Current task</span>
          <textarea
            value={task}
            maxLength={256}
            disabled={busy}
            onChange={(event) => {
              setTask(event.target.value);
              setPreview(null);
              setError(null);
            }}
            placeholder="What should the next agent work on?"
            className="mt-2 min-h-24 w-full resize-y rounded-sm border border-border bg-surface-2 px-3 py-2 text-body text-foreground outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-primary"
          />
          <span className="mt-1 block text-micro text-muted-foreground">
            {task.length}/256 characters · canonical brief defaults: 8 items,
            1,500-token material budget.
          </span>
        </label>

        <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
          <label className="text-meta font-semibold">
            <span className="block">Agent target</span>
            <select
              aria-label="Agent brief egress target"
              value={target}
              disabled={busy}
              onChange={(event) => {
                setTarget(event.target.value as AgentEgressTarget);
                setPreview(null);
                setError(null);
              }}
              className="mt-2 rounded-sm border border-border bg-surface-2 px-3 py-2 text-meta font-normal text-foreground outline-none focus-visible:ring-2 focus-visible:ring-primary"
            >
              <option value="cloud">Cloud (packaged default)</option>
              <option value="local">Local (explicit host assertion)</option>
            </select>
          </label>
          <Button
            variant="primary"
            type="submit"
            disabled={!task.trim() || busy}
          >
            {busy ? (
              <RefreshCw
                size={14}
                className="animate-spin motion-reduce:animate-none"
              />
            ) : (
              <BrainCircuit size={14} />
            )}
            {busy ? "Compiling brief" : "Preview agent brief"}
          </Button>
        </div>

        <div className="flex items-start gap-2 text-micro leading-4 text-muted-foreground">
          {target === "cloud" ? (
            <Cloud size={13} className="mt-0.5 shrink-0" aria-hidden="true" />
          ) : (
            <Cpu size={13} className="mt-0.5 shrink-0" aria-hidden="true" />
          )}
          <span>
            {target === "cloud"
              ? "Cloud matches the packaged MCP default. Ley applies the cloud egress policy before returning any brief content."
              : "Local is an explicit host configuration assertion. Ley does not attest that the downstream provider or runtime is actually local."}
          </span>
        </div>
      </form>

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
            <p className="font-semibold">Brief preview unavailable</p>
            <p className="mt-0.5 text-muted-foreground">{error}</p>
          </div>
        </div>
      )}

      {preview && (
        <BriefResult preview={preview} onEvidence={onEvidence} />
      )}
    </div>
  );
}

function BriefResult({
  preview,
  onEvidence,
}: {
  preview: AgentBriefPreviewResult;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
}) {
  const withheld = preview.egressCoverage?.withheldDerivedResults ?? 0;
  return (
    <div className="space-y-4">
      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <Metric label="Evidence" value={humanize(preview.evidenceState)} />
        <Metric
          label="Budget"
          value={preview.estimatedTokens + " / " + preview.maxTokens + " tokens"}
        />
        <Metric label="Returned items" value={String(preview.items.length)} />
        <Metric
          label="Egress"
          value={
            withheld > 0
              ? withheld + " derived withheld"
              : humanize(preview.egressTarget ?? "none")
          }
        />
      </section>

      {preview.premiseAdjudication.warnings.length > 0 && (
        <section className="rounded-md border border-warning/30 bg-warning/8 p-4">
          <p className="text-meta font-semibold">
            Premise warnings · {humanize(preview.premiseAdjudication.state)}
          </p>
          <ul className="mt-2 space-y-2 text-meta leading-5 text-muted-foreground">
            {preview.premiseAdjudication.warnings.map((warning, index) => (
              <li key={warning.kind + ":" + index}>{warning.message}</li>
            ))}
          </ul>
        </section>
      )}

      {preview.specifications.length > 0 && (
        <section>
          <div className="mb-2 flex items-center gap-2">
            <FileCheck2 size={15} aria-hidden="true" />
            <h3 className="text-body font-semibold">
              Approved Specifications · {preview.specifications.length}
            </h3>
          </div>
          <div className="space-y-2">
            {preview.specifications.map((specification) => (
              <div
                key={specification.specificationId}
                className="rounded-md border border-border bg-surface-1 p-3"
              >
                <p className="font-mono text-meta">
                  {specification.relativePath}
                </p>
                <p className="mt-1 text-micro text-muted-foreground">
                  relevance {specification.relevanceScore} ·{" "}
                  {specification.exactMatch ? "exact match" : "task match"} ·{" "}
                  {specification.estimatedTokens} tokens
                </p>
              </div>
            ))}
          </div>
        </section>
      )}

      <section>
        <div className="mb-2 flex items-center justify-between gap-3">
          <h3 className="text-body font-semibold">
            Continuity items · {preview.items.length}
          </h3>
          <span className="text-micro text-muted-foreground">
            {preview.liveSourceChecked
              ? "Live source checked"
              : "Historical evidence; live source unchecked"}
          </span>
        </div>
        {preview.items.length === 0 ? (
          <div className="rounded-md border border-border bg-surface-1 p-4 text-meta text-muted-foreground">
            No continuity item fit this task and authority boundary.
          </div>
        ) : (
          <div className="space-y-2">
            {preview.items.map((item) => (
              <article
                key={item.kind + ":" + item.entityId}
                className="rounded-md border border-border bg-surface-1 p-4"
              >
                <div className="flex flex-wrap items-start justify-between gap-2">
                  <div>
                    <p className="text-micro font-semibold uppercase tracking-[0.12em] text-muted-foreground">
                      {humanize(item.kind)} · {humanize(item.authority)}
                    </p>
                    <h4 className="mt-1 text-body font-semibold">{item.title}</h4>
                  </div>
                  <span className="text-micro text-muted-foreground">
                    {item.estimatedTokens} tokens
                  </span>
                </div>
                <p className="mt-2 whitespace-pre-wrap text-meta leading-5 text-muted-foreground-strong">
                  {item.excerpt}
                </p>
                {item.citation && (
                  <button
                    type="button"
                    onClick={() => onEvidence(item.citation!)}
                    className="mt-3 inline-flex items-center gap-1.5 text-micro font-semibold text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
                  >
                    <ExternalLink size={12} aria-hidden="true" />
                    Open cited evidence
                  </button>
                )}
              </article>
            ))}
          </div>
        )}
      </section>

      {(preview.gaps.length > 0 ||
        (preview.egressExclusions?.length ?? 0) > 0) && (
        <section className="rounded-md border border-border bg-surface-1 p-4">
          <div className="flex items-center gap-2">
            <ShieldCheck size={15} aria-hidden="true" />
            <h3 className="text-body font-semibold">Why content was limited</h3>
          </div>
          <ul className="mt-2 space-y-1.5 text-meta leading-5 text-muted-foreground">
            {preview.gaps.map((gap, index) => (
              <li key={gap.kind + ":" + index}>{gap.message}</li>
            ))}
            {(preview.egressExclusions ?? []).map((exclusion, index) => (
              <li key={exclusion.scopeId + ":" + index}>
                {humanize(exclusion.policy)} blocked {humanize(exclusion.scopeKind)}{" "}
                {exclusion.scopeId}.
              </li>
            ))}
          </ul>
        </section>
      )}

      <details className="rounded-md border border-border bg-surface-1">
        <summary className="cursor-pointer px-4 py-3 text-meta font-semibold">
          Raw canonical compiler payload
        </summary>
        <pre className="max-h-[32rem] overflow-auto border-t border-border p-4 text-micro leading-5 text-muted-foreground">
          {JSON.stringify(preview, null, 2)}
        </pre>
      </details>

      <p className="text-micro leading-4 text-subtle-foreground">
        {preview.instructionWarning}
      </p>
      <p className="text-micro leading-4 text-subtle-foreground">
        {preview.privacyNotice}
      </p>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-border bg-surface-1 p-4">
      <p className="text-micro font-semibold uppercase tracking-[0.12em] text-muted-foreground">
        {label}
      </p>
      <p className="mt-1 text-body font-semibold">{value}</p>
    </div>
  );
}

function humanize(value: string): string {
  return value
    .replaceAll("-", " ")
    .replace(/\b\w/g, (character) => character.toUpperCase());
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
