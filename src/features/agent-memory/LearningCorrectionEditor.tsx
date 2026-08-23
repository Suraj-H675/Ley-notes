import { useState } from "react";
import { PencilLine } from "lucide-react";
import { Button } from "@/shared/components/Button";
import { correctAgentLearning } from "./api";
import type { AgentMemoryDashboard, LearningContext } from "./types";

export function LearningCorrectionEditor({
  projectPath,
  learning,
  onCancel,
  onCorrected,
}: {
  projectPath: string;
  learning: LearningContext;
  onCancel: () => void;
  onCorrected: (dashboard: AgentMemoryDashboard) => void;
}) {
  const [title, setTitle] = useState(learning.title);
  const [guidance, setGuidance] = useState(learning.guidance);
  const [confidence, setConfidence] = useState(learning.confidencePercent);
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const changed =
    title.trim() !== learning.title ||
    guidance.trim() !== learning.guidance ||
    confidence !== learning.confidencePercent;
  const canSubmit =
    title.trim().length > 0 &&
    guidance.trim().length > 0 &&
    reason.trim().length > 0 &&
    changed;
  const titleError = title.trim() === "" ? "A title is required." : null;
  const guidanceError =
    guidance.trim() === "" ? "Corrected guidance is required." : null;
  const reasonError =
    reason.trim() === ""
      ? "Explain what changed or why the previous claim was wrong."
      : null;

  async function submit() {
    if (!canSubmit || busy) return;
    setBusy(true);
    setError(null);
    try {
      const dashboard = await correctAgentLearning(
        projectPath,
        learning.learningId,
        learning.eventCount,
        title.trim(),
        guidance.trim(),
        confidence,
        reason.trim(),
      );
      onCorrected(dashboard);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="max-h-[48vh] overflow-y-auto overscroll-contain pr-1">
      <div className="flex items-start gap-3 rounded-md border border-primary/20 bg-primary/8 p-3">
        <PencilLine
          size={15}
          className="mt-0.5 shrink-0 text-primary"
          aria-hidden="true"
        />
        <p className="text-micro leading-5 text-muted-foreground">
          Correction appends a new version, preserves cited evidence and old
          history, and returns the claim to review. Confirm the corrected
          version separately before reuse.
        </p>
      </div>
      <div className="mt-3 grid gap-3">
        <label className="block text-meta font-medium">
          Title
          <input
            type="text"
            name="learning-correction-title"
            autoComplete="off"
            value={title}
            maxLength={256}
            disabled={busy}
            onChange={(event) => setTitle(event.target.value)}
            aria-invalid={Boolean(titleError)}
            className={`mt-1.5 w-full rounded-md border bg-background/45 px-3 py-2 text-meta outline-none focus-visible:border-primary focus-visible:ring-1 focus-visible:ring-primary ${
              titleError
                ? "border-destructive text-destructive"
                : "border-border text-foreground"
            }`}
          />
          {titleError && (
            <p className="mt-0.5 text-micro text-destructive" role="alert">
              {titleError}
            </p>
          )}
        </label>
        <label className="block text-meta font-medium">
          Corrected guidance
          <textarea
            name="learning-correction-guidance"
            autoComplete="off"
            value={guidance}
            maxLength={16_000}
            rows={4}
            disabled={busy}
            onChange={(event) => setGuidance(event.target.value)}
            aria-invalid={Boolean(guidanceError)}
            className={`mt-1.5 w-full resize-y rounded-md border bg-background/45 px-3 py-2 text-meta leading-5 outline-none focus-visible:border-primary focus-visible:ring-1 focus-visible:ring-primary ${
              guidanceError
                ? "border-destructive text-destructive"
                : "border-border text-foreground"
            }`}
          />
          {guidanceError && (
            <p className="mt-0.5 text-micro text-destructive" role="alert">
              {guidanceError}
            </p>
          )}
        </label>
        <label className="block text-meta font-medium">
          <span className="flex items-center justify-between gap-3">
            Confidence
            <output className="tabular-nums text-muted-foreground">
              {confidence}%
            </output>
          </span>
          <input
            type="range"
            name="learning-correction-confidence"
            min={0}
            max={100}
            step={1}
            value={confidence}
            disabled={busy}
            onChange={(event) => setConfidence(event.target.valueAsNumber)}
            className="mt-2 w-full rounded accent-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
          />
        </label>
        <label className="block text-meta font-medium">
          Why is this correction needed?
          <span className="ml-1 font-normal text-muted-foreground">
            · required
          </span>
          <textarea
            name="learning-correction-reason"
            autoComplete="off"
            value={reason}
            maxLength={4_000}
            rows={2}
            disabled={busy}
            placeholder="Explain what changed or what the previous claim got wrong…"
            onChange={(event) => setReason(event.target.value)}
            aria-invalid={Boolean(reasonError)}
            className={`mt-1.5 w-full resize-none rounded-md border bg-background/45 px-3 py-2 text-meta outline-none placeholder:text-subtle-foreground focus-visible:border-primary focus-visible:ring-1 focus-visible:ring-primary ${
              reasonError
                ? "border-destructive text-destructive"
                : "border-border text-foreground"
            }`}
          />
          {reasonError && (
            <p className="mt-0.5 text-micro text-destructive" role="alert">
              {reasonError}
            </p>
          )}
        </label>
      </div>
      {error && (
        <p className="mt-2 break-words text-micro text-destructive" role="alert">
          {error}
        </p>
      )}
      <div className="mt-3 flex justify-end gap-2">
        <Button
          size="sm"
          variant="ghost"
          disabled={busy}
          onClick={onCancel}
        >
          Cancel
        </Button>
        <Button
          size="sm"
          variant="primary"
          disabled={busy || !canSubmit}
          onClick={() => void submit()}
        >
          {busy ? "Appending correction…" : "Append correction"}
        </Button>
      </div>
    </div>
  );
}

function errorMessage(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  return "Ley could not append this correction. Reload the learning and try again.";
}
