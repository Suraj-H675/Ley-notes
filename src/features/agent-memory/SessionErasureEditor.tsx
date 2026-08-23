import { useState } from "react";
import { LockKeyhole, RefreshCw, Trash2 } from "lucide-react";
import { Button } from "@/shared/components/Button";
import { eraseAgentSession } from "./api";
import type { AgentSessionErasure, SessionContext } from "./types";

interface SessionErasureEditorProps {
  projectPath: string;
  session: SessionContext;
  onCancel: () => void;
  onDirtyChange: (dirty: boolean) => void;
  onErased: (result: AgentSessionErasure) => void;
}

export function SessionErasureEditor({
  projectPath,
  session,
  onCancel,
  onDirtyChange,
  onErased,
}: SessionErasureEditorProps) {
  const [confirmation, setConfirmation] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const confirmed = confirmation === session.name;

  async function erase() {
    if (!confirmed || busy) return;
    setBusy(true);
    setError(null);
    try {
      onErased(
        await eraseAgentSession(
          projectPath,
          session.sessionId,
          session.eventCount,
          session.name,
        ),
      );
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex max-h-[68vh] flex-col overflow-hidden">
      <div className="min-h-0 overflow-y-auto overscroll-contain p-4 sm:p-5">
        <div className="flex items-start gap-3 rounded-md border border-destructive/25 bg-destructive/8 p-4">
          <LockKeyhole
            size={16}
            className="mt-0.5 shrink-0 text-destructive"
            aria-hidden="true"
          />
          <div>
            <p className="text-meta font-semibold">
              Permanently erase this private session record
            </p>
            <p className="mt-1 text-micro leading-5 text-muted-foreground">
              Ley will physically remove all {session.eventCount} session{" "}
              {session.eventCount === 1 ? "event" : "events"} and every lesson
              that cites them. A dependent supersession chain is removed too.
              This cannot be undone in Ley.
            </p>
          </div>
        </div>

        <div className="mt-3 rounded-md border border-border bg-background/35 p-4">
          <p className="text-micro font-semibold uppercase tracking-[0.12em] text-muted-foreground">
            Kept in place
          </p>
          <ul className="mt-2 grid gap-1.5 text-micro leading-5 text-muted-foreground sm:grid-cols-2">
            <li>Unrelated sessions and lessons</li>
            <li>Project captures and graph history</li>
            <li>Project files, policy, and vault binding</li>
            <li>Ordinary Markdown and Canvas copies</li>
          </ul>
          <p className="mt-3 border-t border-border pt-3 text-micro leading-5 text-muted-foreground">
            Delete linked notes or Canvas files separately if those user-owned
            copies should also be forgotten. Backups, filesystem snapshots,
            provider-retained context, and device remnants are outside Ley’s
            control.
          </p>
        </div>

        <label className="mt-4 block">
          <span className="text-micro font-semibold text-muted-foreground-strong">
            Type{" "}
            <span className="break-all font-mono text-foreground">
              {session.name}
            </span>{" "}
            to confirm
          </span>
          <input
            type="text"
            name="session-erasure-confirmation"
            value={confirmation}
            disabled={busy}
            autoComplete="off"
            spellCheck={false}
            onChange={(event) => {
              setConfirmation(event.target.value);
              onDirtyChange(event.target.value.length > 0);
            }}
            className="mt-2 h-10 w-full rounded-lg border border-border bg-background px-3 text-meta text-foreground outline-none transition-[border-color,box-shadow] focus:border-destructive/60 focus:ring-2 focus:ring-destructive/15 disabled:opacity-60"
          />
        </label>

        {error && (
          <p
            className="mt-3 break-words text-meta text-destructive"
            role="alert"
          >
            {error}
          </p>
        )}
      </div>

      <div className="flex shrink-0 flex-wrap items-center justify-end gap-2 border-t border-border px-4 py-3 sm:px-5">
        <Button
          size="sm"
          variant="ghost"
          className="h-8"
          disabled={busy}
          onClick={onCancel}
        >
          Cancel
        </Button>
        <Button
          size="sm"
          variant="destructive"
          className="h-8"
          disabled={busy || !confirmed}
          onClick={() => void erase()}
        >
          {busy ? (
            <RefreshCw
              size={13}
              className="animate-spin motion-reduce:animate-none"
              aria-hidden="true"
            />
          ) : (
            <Trash2 size={13} aria-hidden="true" />
          )}
          {busy ? "Erasing local memory…" : "Permanently erase session"}
        </Button>
      </div>
    </div>
  );
}

function errorMessage(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  return "Ley could not erase this session. Reload it and try again.";
}
