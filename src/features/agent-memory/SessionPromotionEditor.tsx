import { useState } from "react";
import { FilePlus2, ShieldCheck } from "lucide-react";
import { Button } from "@/shared/components/Button";
import {
  buildSessionNoteDraft,
  SESSION_NOTE_FOLDER,
} from "./session-note-draft";
import type { PromotedSessionNoteDraft, SessionContext } from "./types";

export function SessionPromotionEditor({
  projectName,
  session,
  onCancel,
  onDirtyChange,
  onPromote,
}: {
  projectName: string;
  session: SessionContext;
  onCancel: () => void;
  onDirtyChange: (dirty: boolean) => void;
  onPromote: (draft: PromotedSessionNoteDraft) => Promise<void>;
}) {
  const initialTitle = `${session.name} handoff`;
  const [title, setTitle] = useState(initialTitle);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const canSubmit = title.trim().length > 0 && !busy;
  const preview =
    session.finish?.handoff ??
    session.finish?.summary ??
    session.checkpoints.at(-1)?.summary ??
    session.goal;

  async function submit() {
    if (!canSubmit) return;
    setBusy(true);
    setError(null);
    try {
      await onPromote(
        buildSessionNoteDraft(projectName, session, title.trim()),
      );
    } catch (cause) {
      setError(errorMessage(cause));
      setBusy(false);
    }
  }

  return (
    <div className="flex max-h-[52vh] min-h-0 flex-col p-4 sm:px-5">
      <div className="min-h-0 overflow-y-auto overscroll-contain pr-1">
        <div className="flex items-start gap-3 rounded-lg border border-primary/20 bg-primary/7 p-3">
          <ShieldCheck
            size={15}
            className="mt-0.5 shrink-0 text-primary"
            aria-hidden="true"
          />
          <p className="text-micro leading-5 text-muted-foreground">
            Create a user-owned Markdown handoff from this inspected session.
            Ley verifies that the open notes vault is the project’s bound vault
            before writing. The immutable session remains separate.
          </p>
        </div>

        <label className="mt-3 block text-meta font-medium">
          Note title
          <input
            type="text"
            name="session-promotion-title"
            autoComplete="off"
            value={title}
            maxLength={256}
            disabled={busy}
            onChange={(event) => {
              setTitle(event.target.value);
              onDirtyChange(event.target.value !== initialTitle);
            }}
            className="mt-1.5 w-full rounded-lg border border-border bg-background/45 px-3 py-2 text-meta text-foreground outline-none focus-visible:border-primary focus-visible:ring-1 focus-visible:ring-primary"
          />
        </label>

        <div className="mt-3 rounded-lg border border-border bg-background/35 p-3">
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
            Destination · {SESSION_NOTE_FOLDER}
          </p>
          <p className="mt-2 line-clamp-3 whitespace-pre-wrap break-words text-micro leading-5 text-muted-foreground">
            {preview}
          </p>
          <p className="mt-2 text-micro leading-5 text-muted-foreground">
            Includes the goal, outcome, handoff, unresolved work, visible
            checkpoints, verification, and captured artifact trail. It is
            labeled as a bounded snapshot and never silently synchronizes.
          </p>
          {(session.truncated || session.omittedCheckpoints > 0) && (
            <p className="mt-2 rounded-md bg-amber-500/10 px-2.5 py-2 text-micro leading-5 text-amber-500">
              This inspected projection omits {session.omittedCheckpoints} older
              checkpoints or clipped text. The note will preserve that
              disclosure.
            </p>
          )}
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
        <Button size="sm" variant="ghost" disabled={busy} onClick={onCancel}>
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
