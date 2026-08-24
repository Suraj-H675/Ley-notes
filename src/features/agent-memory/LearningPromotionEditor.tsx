import { useState } from "react";
import { FilePlus2, ShieldCheck } from "lucide-react";
import { Button } from "@/shared/components/Button";
import {
  buildPromotionDraft,
  PROMOTION_FOLDER,
} from "./learning-promotion-draft";
import type { LearningContext, PromotedLearningNoteDraft } from "./types";

export function LearningPromotionEditor({
  projectName,
  learning,
  onCancel,
  onPromote,
}: {
  projectName: string;
  learning: LearningContext;
  onCancel: () => void;
  onPromote: (draft: PromotedLearningNoteDraft) => Promise<void>;
}) {
  const [title, setTitle] = useState(learning.title);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const canSubmit = title.trim().length > 0 && !busy;

  async function submit() {
    if (!canSubmit) return;
    setBusy(true);
    setError(null);
    try {
      await onPromote(buildPromotionDraft(projectName, learning, title.trim()));
    } catch (cause) {
      setError(errorMessage(cause));
      setBusy(false);
    }
  }

  return (
    <div className="flex max-h-[48vh] min-h-0 flex-col">
      <div className="min-h-0 overflow-y-auto overscroll-contain pr-1">
        <div className="flex items-start gap-3 rounded-md border border-success/25 bg-success/8 p-3">
          <ShieldCheck
            size={15}
            className="mt-0.5 shrink-0 text-success"
            aria-hidden="true"
          />
          <p className="text-micro leading-5 text-muted-foreground">
            Promotion creates an ordinary Markdown note from this exact trusted
            version. The Agent Memory ledger remains separate and unchanged.
          </p>
        </div>

        <label className="mt-3 block text-meta font-medium">
          Note title
          <input
            type="text"
            name="learning-promotion-title"
            autoComplete="off"
            value={title}
            maxLength={256}
            disabled={busy}
            onChange={(event) => setTitle(event.target.value)}
            className="mt-1.5 w-full rounded-md border border-border bg-background/45 px-3 py-2 text-meta text-foreground outline-none focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-primary"
          />
        </label>

        <div className="mt-3 rounded-md border border-border bg-background/35 p-3">
          <div className="flex min-w-0 items-center gap-2">
            <FilePlus2
              size={14}
              className="shrink-0 text-primary"
              aria-hidden="true"
            />
            <p className="min-w-0 truncate text-meta font-medium">
              {title.trim() || "Untitled"}
            </p>
          </div>
          <p className="mt-1 truncate text-micro text-muted-foreground">
            Destination · {PROMOTION_FOLDER}
          </p>
          <p className="mt-2 line-clamp-4 whitespace-pre-wrap break-words text-micro leading-5 text-muted-foreground">
            {learning.guidance}
          </p>
          <p className="mt-2 text-micro text-muted-foreground">
            Ley adds local provenance, confidence, validity, and cited source
            identifiers beneath the guidance.
          </p>
        </div>
      </div>

      {error && (
        <p
          className="mt-2 break-words text-micro text-destructive"
          role="alert"
        >
          {error}
        </p>
      )}

      <div className="mt-3 flex shrink-0 justify-end gap-2">
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
          disabled={!canSubmit}
          onClick={() => void submit()}
        >
          {busy ? "Creating note…" : "Create & open note"}
        </Button>
      </div>
    </div>
  );
}

function errorMessage(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  return "Ley could not create this note. Check the vault and try again.";
}
