import { useEffect, useState } from "react";
import { CircleAlert, RefreshCw, ShieldCheck } from "lucide-react";
import { Button } from "@/shared/components/Button";
import {
  readChronicleCaptureTargets,
  requestChronicleCapture,
  revokeChronicleCapture,
} from "./chronicle-api";
import type {
  ChronicleCaptureMode,
  ChronicleCaptureTarget,
} from "./chronicle-api";

export function ChronicleCaptureControl({ projectId }: { projectId: string }) {
  const [targets, setTargets] = useState<ChronicleCaptureTarget[] | null>(null);
  const [selectedModes, setSelectedModes] = useState<
    Record<string, ChronicleCaptureMode>
  >({});
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    setError(null);
    setBusy(true);
    try {
      setTargets(await readChronicleCaptureTargets(projectId));
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    let current = true;
    void readChronicleCaptureTargets(projectId)
      .then((next) => {
        if (current) setTargets(next);
      })
      .catch((cause) => {
        if (current) setError(message(cause));
      })
      .finally(() => {
        if (current) setBusy(false);
      });
    return () => {
      current = false;
    };
  }, [projectId]);

  async function change(
    target: ChronicleCaptureTarget,
    action: "enable" | "revoke",
  ) {
    setError(null);
    setBusy(true);
    try {
      if (action === "enable") {
        await requestChronicleCapture(target, selectedMode(target));
      } else {
        await revokeChronicleCapture(target);
      }
      setTargets(await readChronicleCaptureTargets(projectId));
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusy(false);
    }
  }

  function selectedMode(target: ChronicleCaptureTarget): ChronicleCaptureMode {
    return selectedModes[target.locatorId] ?? defaultMode(target);
  }

  return (
    <section aria-labelledby="codex-capture-title">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h2
            id="codex-capture-title"
            className="text-lg font-semibold tracking-tight"
          >
            Codex activity capture
          </h2>
          <p className="mt-1 max-w-3xl text-micro leading-5 text-muted-foreground">
            Authorize future observable Codex activity for one attached working
            copy. Connecting Codex or importing project files does not grant
            capture permission.
          </p>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => void load()}
          disabled={busy}
          aria-label="Refresh Codex capture permissions"
        >
          <RefreshCw size={13} aria-hidden="true" />
          Refresh
        </Button>
      </div>

      <p className="mt-2 max-w-3xl text-micro leading-5 text-muted-foreground">
        Ley does not read Codex transcript files or hidden reasoning. Tool
        results are observations, not proof that a command, test, or task
        succeeded. Model sharing is a separate permission.
      </p>

      <div
        className="mt-4 divide-y divide-border border-y border-border"
        aria-live="polite"
      >
        {targets === null && !error && (
          <p className="py-4 text-meta text-muted-foreground">
            Loading attached working copies…
          </p>
        )}
        {targets?.length === 0 && (
          <p className="py-4 text-meta leading-6 text-muted-foreground">
            This Project Brain has no authorized working copy. Attach one before
            enabling Codex capture.
          </p>
        )}
        {targets?.map((target) => (
          <div
            key={target.locatorId}
            className="flex flex-col gap-3 py-4 sm:flex-row sm:items-center sm:justify-between"
          >
            <div className="min-w-0">
              <p className="break-all text-meta font-medium">
                {target.localPath}
              </p>
              <p className="mt-1 text-micro leading-5 text-muted-foreground">
                {captureStatus(target)}
              </p>
              {target.inactiveReason &&
                target.authorized &&
                !target.effective && (
                  <p className="mt-1 text-micro text-warning">
                    Permission is inactive:{" "}
                    {target.inactiveReason.replaceAll("-", " ")}.
                  </p>
                )}
            </div>
            <div className="flex shrink-0 flex-col gap-2 sm:items-end">
              <label
                className="text-micro font-medium text-muted-foreground"
                htmlFor={`chronicle-retention-${target.locatorId}`}
              >
                Retention
              </label>
              <select
                id={`chronicle-retention-${target.locatorId}`}
                value={selectedMode(target)}
                disabled={busy}
                onChange={(event) => {
                  const value = parseMode(event.target.value);
                  if (!value) return;
                  setSelectedModes((current) => ({
                    ...current,
                    [target.locatorId]: value,
                  }));
                }}
                className="max-w-xs rounded-md border border-border bg-surface-1 px-3 py-2 text-micro focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
              >
                <option value="minimal">Minimal</option>
                <option value="structured">Structured</option>
                <option value="full-evidence">Full evidence</option>
              </select>
              <div className="flex flex-wrap gap-2">
                <Button
                  disabled={busy}
                  onClick={() => void change(target, "enable")}
                >
                  <ShieldCheck size={13} aria-hidden="true" />
                  {target.effective ? "Review permission" : "Review and enable"}
                </Button>
                {target.grantId && target.authorized && (
                  <Button
                    variant="ghost"
                    disabled={busy}
                    onClick={() => void change(target, "revoke")}
                  >
                    Stop capture
                  </Button>
                )}
              </div>
            </div>
          </div>
        ))}
      </div>

      {error && (
        <p
          role="alert"
          className="mt-3 flex items-start gap-2 text-micro text-destructive"
        >
          <CircleAlert
            size={14}
            className="mt-0.5 shrink-0"
            aria-hidden="true"
          />
          <span>{error}</span>
        </p>
      )}
      <p className="mt-3 max-w-3xl text-micro leading-5 text-muted-foreground">
        Enabling capture opens a native confirmation for the exact Project
        Brain, working copy, host, and retention mode. Stopping capture
        preserves already retained Chronicle history.
      </p>
    </section>
  );
}

function parseMode(value: string): ChronicleCaptureMode | null {
  if (
    value === "minimal" ||
    value === "structured" ||
    value === "full-evidence"
  ) {
    return value;
  }
  return null;
}

function defaultMode(target: ChronicleCaptureTarget): ChronicleCaptureMode {
  return target.mode ?? "structured";
}

function captureStatus(target: ChronicleCaptureTarget): string {
  if (target.effective && target.mode) {
    return `Codex capture is enabled with ${target.mode} retention.`;
  }
  if (target.authorized) {
    return "A retained permission exists, but it is not currently effective.";
  }
  return "Codex capture is disabled for this working copy.";
}

function message(cause: unknown): string {
  return cause instanceof Error
    ? cause.message
    : typeof cause === "string"
      ? cause
      : "Capture permissions could not be updated.";
}
