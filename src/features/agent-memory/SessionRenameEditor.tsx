import { useState } from "react";
import { History, PencilLine } from "lucide-react";
import { Button } from "@/shared/components/Button";
import { renameAgentSession } from "./api";
import type { AgentMemoryDashboard, SessionContext } from "./types";

export function SessionRenameEditor({
  projectPath,
  session,
  onCancel,
  onDirtyChange,
  onRenamed,
}: {
  projectPath: string;
  session: SessionContext;
  onCancel: () => void;
  onDirtyChange: (dirty: boolean) => void;
  onRenamed: (dashboard: AgentMemoryDashboard) => void;
}) {
  const [name, setName] = useState(session.name);
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const changed = name.trim() !== session.name;
  const canSubmit =
    name.trim().length > 0 && reason.trim().length > 0 && changed;

  async function submit() {
    if (!canSubmit || busy) return;
    setBusy(true);
    setError(null);
    try {
      const dashboard = await renameAgentSession(
        projectPath,
        session.sessionId,
        session.eventCount,
        name.trim(),
        reason.trim(),
      );
      onRenamed(dashboard);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex max-h-[46vh] flex-col overflow-hidden">
      <div className="min-h-0 overflow-y-auto overscroll-contain p-4 sm:p-5">
        <div className="flex items-start gap-3 rounded-lg border border-primary/20 bg-primary/8 p-3">
          <History
            size={15}
            className="mt-0.5 shrink-0 text-primary"
            aria-hidden="true"
          />
          <p className="text-micro leading-5 text-muted-foreground">
            Renaming appends an audited event. The original name, reason,
            session ID, citations, and full history remain intact.
          </p>
        </div>
        <div className="mt-3 grid gap-3">
          <label className="block text-meta font-medium">
            Session name
            <input
              type="text"
              name="session-name"
              autoComplete="off"
              value={name}
              maxLength={128}
              disabled={busy}
              onChange={(event) => {
                const next = event.target.value;
                setName(next);
                onDirtyChange(
                  next.trim() !== session.name || reason.trim().length > 0,
                );
              }}
              className="mt-1.5 w-full rounded-lg border border-border bg-background/45 px-3 py-2 text-meta text-foreground outline-none focus-visible:border-primary focus-visible:ring-1 focus-visible:ring-primary"
            />
          </label>
          <label className="block text-meta font-medium">
            Why are you renaming this session?
            <span className="ml-1 font-normal text-muted-foreground">
              · required
            </span>
            <textarea
              name="session-rename-reason"
              autoComplete="off"
              value={reason}
              maxLength={4_000}
              rows={2}
              disabled={busy}
              placeholder="Describe what the clearer name captures…"
              onChange={(event) => {
                const next = event.target.value;
                setReason(next);
                onDirtyChange(
                  name.trim() !== session.name || next.trim().length > 0,
                );
              }}
              className="mt-1.5 w-full resize-none rounded-lg border border-border bg-background/45 px-3 py-2 text-meta text-foreground outline-none placeholder:text-subtle-foreground focus-visible:border-primary focus-visible:ring-1 focus-visible:ring-primary"
            />
          </label>
        </div>
        {error && (
          <p
            className="mt-2 break-words text-micro text-destructive"
            role="alert"
          >
            {error}
          </p>
        )}
      </div>
      <div className="flex shrink-0 justify-end gap-2 border-t border-border px-4 py-3 sm:px-5">
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
          variant="primary"
          className="h-8"
          disabled={busy || !canSubmit}
          onClick={() => void submit()}
        >
          <PencilLine size={13} aria-hidden="true" />
          {busy ? "Appending rename…" : "Append rename"}
        </Button>
      </div>
    </div>
  );
}

function errorMessage(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  return "Ley could not rename this session. Reload it and try again.";
}
