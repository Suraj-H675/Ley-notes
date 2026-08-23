import { useEffect, useState } from "react";
import { FileText, LayoutDashboard, Plus, ShieldCheck } from "lucide-react";
import { listCanvases, type CanvasSummary } from "@/core/vault/canvas";
import { Button } from "@/shared/components/Button";
import {
  buildSessionNoteDraft,
  SESSION_NOTE_FOLDER,
} from "./session-note-draft";
import type {
  SessionCanvasDestination,
  SessionCanvasLinkRequest,
} from "./link-session-canvas";
import type { SessionContext } from "./types";

export function SessionCanvasEditor({
  projectName,
  session,
  onCancel,
  onDirtyChange,
  onLink,
}: {
  projectName: string;
  session: SessionContext;
  onCancel: () => void;
  onDirtyChange: (dirty: boolean) => void;
  onLink: (request: SessionCanvasLinkRequest) => Promise<void>;
}) {
  const initialTitle = `${session.name} handoff`;
  const initialCanvasName = `${projectName} continuity`;
  const [title, setTitle] = useState(initialTitle);
  const [canvases, setCanvases] = useState<CanvasSummary[]>([]);
  const [mode, setMode] = useState<SessionCanvasDestination["kind"]>("new");
  const [canvasPath, setCanvasPath] = useState("");
  const [canvasName, setCanvasName] = useState(initialCanvasName);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void listCanvases()
      .then((items) => {
        if (!current) return;
        setCanvases(items);
        const firstWritable = items.find((canvas) => !canvas.readError);
        if (firstWritable) {
          setMode("existing");
          setCanvasPath(firstWritable.path);
        }
      })
      .catch((cause) => {
        if (current) setError(errorMessage(cause));
      })
      .finally(() => {
        if (current) setLoading(false);
      });
    return () => {
      current = false;
    };
  }, []);

  const destination: SessionCanvasDestination =
    mode === "existing"
      ? { kind: "existing", path: canvasPath }
      : { kind: "new", name: canvasName.trim() };
  const writableCanvases = canvases.filter((canvas) => !canvas.readError);
  const firstWritablePath = writableCanvases[0]?.path ?? "";
  const canSubmit =
    !loading &&
    !busy &&
    title.trim().length > 0 &&
    (destination.kind === "existing"
      ? destination.path.length > 0
      : destination.name.length > 0);

  function markDirty(next: {
    title?: string;
    mode?: SessionCanvasDestination["kind"];
    canvasPath?: string;
    canvasName?: string;
  }) {
    onDirtyChange(
      (next.title ?? title) !== initialTitle ||
        (next.mode ?? mode) !==
          (writableCanvases.length > 0 ? "existing" : "new") ||
        (next.canvasPath ?? canvasPath) !== firstWritablePath ||
        (next.canvasName ?? canvasName) !== initialCanvasName,
    );
  }

  async function submit() {
    if (!canSubmit) return;
    setBusy(true);
    setError(null);
    try {
      await onLink({
        draft: buildSessionNoteDraft(projectName, session, title.trim()),
        destination,
      });
    } catch (cause) {
      setError(errorMessage(cause));
      setBusy(false);
    }
  }

  return (
    <div className="flex h-[min(38rem,62vh)] min-h-0 w-full flex-col overflow-hidden p-4 sm:px-5">
      <div className="min-h-0 overflow-y-auto overscroll-contain pr-1">
        <div className="flex items-start gap-3 rounded-md border border-primary/20 bg-primary/7 p-3">
          <ShieldCheck
            size={15}
            className="mt-0.5 shrink-0 text-primary"
            aria-hidden="true"
          />
          <p className="text-micro leading-5 text-muted-foreground">
            Ley verifies this project’s bound vault, creates or reuses one
            ordinary Markdown handoff, then adds a standard JSON Canvas file
            card. The immutable session remains separate.
          </p>
        </div>

        <label className="mt-4 block text-meta font-medium">
          Note title
          <input
            type="text"
            autoComplete="off"
            value={title}
            maxLength={256}
            disabled={busy}
            onChange={(event) => {
              const next = event.target.value;
              setTitle(next);
              markDirty({ title: next });
            }}
            className="mt-1.5 w-full rounded-lg border border-border bg-background/45 px-3 py-2 text-meta text-foreground outline-none transition-[border-color,box-shadow] focus-visible:border-primary focus-visible:ring-1 focus-visible:ring-primary"
          />
        </label>

        <fieldset className="mt-4">
          <legend className="text-meta font-medium">Canvas destination</legend>
          <div className="mt-2 grid gap-2 sm:grid-cols-2">
            <label
              className={`flex cursor-pointer items-start gap-3 rounded-md border p-3 transition-[transform,background-color,border-color] active:scale-[0.99] motion-reduce:transform-none ${
                mode === "existing"
                  ? "border-primary/40 bg-primary/8"
                  : "border-border bg-background/30 hover:bg-surface-2"
              } ${writableCanvases.length === 0 ? "cursor-not-allowed opacity-50" : ""}`}
            >
              <input
                type="radio"
                name="session-canvas-destination"
                value="existing"
                checked={mode === "existing"}
                disabled={busy || writableCanvases.length === 0}
                onChange={() => {
                  setMode("existing");
                  markDirty({ mode: "existing" });
                }}
                className="mt-1 accent-primary"
              />
              <span className="min-w-0">
                <span className="flex items-center gap-1.5 text-meta font-medium">
                  <LayoutDashboard size={14} aria-hidden="true" />
                  Existing Canvas
                </span>
                <span className="mt-1 block text-micro leading-5 text-muted-foreground">
                  {writableCanvases.length > 0
                    ? "Add the handoff to a Canvas already in this vault."
                    : "This vault does not have a writable Canvas yet."}
                </span>
              </span>
            </label>
            <label
              className={`flex cursor-pointer items-start gap-3 rounded-md border p-3 transition-[transform,background-color,border-color] active:scale-[0.99] motion-reduce:transform-none ${
                mode === "new"
                  ? "border-primary/40 bg-primary/8"
                  : "border-border bg-background/30 hover:bg-surface-2"
              }`}
            >
              <input
                type="radio"
                name="session-canvas-destination"
                value="new"
                checked={mode === "new"}
                disabled={busy}
                onChange={() => {
                  setMode("new");
                  markDirty({ mode: "new" });
                }}
                className="mt-1 accent-primary"
              />
              <span className="min-w-0">
                <span className="flex items-center gap-1.5 text-meta font-medium">
                  <Plus size={14} aria-hidden="true" />
                  New Canvas
                </span>
                <span className="mt-1 block text-micro leading-5 text-muted-foreground">
                  Create a Canvas in the vault’s canvases folder.
                </span>
              </span>
            </label>
          </div>
        </fieldset>

        {loading ? (
          <p className="mt-3 text-micro text-muted-foreground" role="status">
            Reading this vault’s Canvases…
          </p>
        ) : mode === "existing" ? (
          <label className="mt-3 block text-meta font-medium">
            Choose Canvas
            <select
              value={canvasPath}
              disabled={busy}
              onChange={(event) => {
                const next = event.target.value;
                setCanvasPath(next);
                markDirty({ canvasPath: next });
              }}
              className="mt-1.5 w-full rounded-lg border border-border bg-background/45 px-3 py-2 text-meta text-foreground outline-none focus-visible:border-primary focus-visible:ring-1 focus-visible:ring-primary"
            >
              {canvases.map((canvas) => (
                <option
                  key={canvas.path}
                  value={canvas.path}
                  disabled={Boolean(canvas.readError)}
                >
                  {canvas.name}
                  {canvas.readError ? " — repair required" : ""}
                </option>
              ))}
            </select>
          </label>
        ) : (
          <label className="mt-3 block text-meta font-medium">
            Canvas name
            <input
              type="text"
              autoComplete="off"
              value={canvasName}
              maxLength={256}
              disabled={busy}
              onChange={(event) => {
                const next = event.target.value;
                setCanvasName(next);
                markDirty({ canvasName: next });
              }}
              className="mt-1.5 w-full rounded-lg border border-border bg-background/45 px-3 py-2 text-meta text-foreground outline-none focus-visible:border-primary focus-visible:ring-1 focus-visible:ring-primary"
            />
            <span className="mt-1 block text-micro leading-5 text-muted-foreground">
              If a Canvas with this generated filename already exists, Ley
              safely reuses it.
            </span>
          </label>
        )}

        <div className="mt-4 grid gap-2 rounded-md border border-border bg-background/35 p-3 sm:grid-cols-2">
          <div className="flex min-w-0 items-center gap-2">
            <FileText size={14} className="shrink-0 text-primary" />
            <div className="min-w-0">
              <p className="truncate text-meta font-medium">
                {title.trim() || "Untitled handoff"}
              </p>
              <p className="truncate text-micro text-muted-foreground">
                {SESSION_NOTE_FOLDER}
              </p>
            </div>
          </div>
          <div className="flex min-w-0 items-center gap-2">
            <LayoutDashboard size={14} className="shrink-0 text-primary" />
            <div className="min-w-0">
              <p className="truncate text-meta font-medium">
                {mode === "existing"
                  ? (canvases.find((canvas) => canvas.path === canvasPath)
                      ?.name ?? "Choose a Canvas")
                  : canvasName.trim() || "Untitled Canvas"}
              </p>
              <p className="text-micro text-muted-foreground">
                Standard file card · retries do not duplicate
              </p>
            </div>
          </div>
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
          <LayoutDashboard size={13} aria-hidden="true" />
          {busy ? "Linking…" : "Link & open Canvas"}
        </Button>
      </div>
    </div>
  );
}

function errorMessage(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  return "Ley could not link this session. Check the vault and try again.";
}
