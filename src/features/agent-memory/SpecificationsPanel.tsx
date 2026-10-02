import { useEffect, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  ExternalLink,
  FileCheck2,
  RefreshCw,
  ShieldCheck,
  XCircle,
} from "lucide-react";
import { Button } from "@/shared/components/Button";
import { cn } from "@/shared/lib/classnames";
import {
  approveAgentProjectFileSource,
  openAgentProjectMarkdownSource,
  readAgentProjectApprovedSources,
  reapproveAgentProjectFileSource,
  revokeAgentProjectApprovedSource,
} from "./api";
import type {
  ApprovedSourceAuthorityList,
  ApprovedSourceState,
} from "./types";

export function SpecificationsPanel({ projectPath }: { projectPath: string }) {
  const [authority, setAuthority] = useState<ApprovedSourceAuthorityList | null>(
    null,
  );
  const [relativePath, setRelativePath] = useState("AGENTS.md");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void readAgentProjectApprovedSources(projectPath)
      .then((next) => {
        if (current) {
          setAuthority(next);
          setError(null);
        }
      })
      .catch((cause) => {
        if (current) setError(errorMessage(cause));
      });
    return () => {
      current = false;
    };
  }, [projectPath]);

  async function approvePath() {
    const path = relativePath.trim();
    if (!path) return;
    setBusy(true);
    setError(null);
    try {
      setAuthority(await approveAgentProjectFileSource(projectPath, path));
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  async function reapprove(sourceId: string) {
    setBusy(true);
    setError(null);
    try {
      setAuthority(await reapproveAgentProjectFileSource(projectPath, sourceId));
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  async function revoke(sourceId: string) {
    setBusy(true);
    setError(null);
    try {
      setAuthority(await revokeAgentProjectApprovedSource(projectPath, sourceId));
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  async function openSource(sourceId: string) {
    setBusy(true);
    setError(null);
    try {
      await openAgentProjectMarkdownSource(projectPath, sourceId);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <header className="max-w-3xl">
        <div className="flex items-center gap-2 text-primary">
          <ShieldCheck size={16} aria-hidden="true" />
          <span className="font-mono text-micro uppercase tracking-[0.14em]">
            Human intent
          </span>
        </div>
        <h2 className="mt-2 text-title font-semibold tracking-tight">
          Approved sources
        </h2>
        <p className="mt-2 text-meta leading-6 text-muted-foreground">
          Approve an exact project file revision as human intent for coding
          agents. Ley stores its project-relative path and content hash; editing
          the file makes that approval stale until you review it again.
        </p>
      </header>

      {error && (
        <div
          role="alert"
          className="rounded-sm border border-destructive/35 bg-destructive/8 px-4 py-3 text-meta text-destructive"
        >
          {error}
        </div>
      )}

      <section className="rounded-sm border border-border bg-surface-1 p-5 shadow-panel">
        <h3 className="text-body font-semibold">Approve project file</h3>
        <p className="mt-1 max-w-2xl text-micro leading-5 text-muted-foreground">
          Enter a path relative to the project root, such as <code>AGENTS.md</code>,{" "}
          <code>.github/copilot-instructions.md</code>, or{" "}
          <code>docs/requirements.md</code>. Ley never follows symlinks and will
          not authorize files inside <code>.git</code> or <code>.ley</code>.
        </p>
        <div className="mt-4 flex flex-col gap-2 sm:flex-row">
          <input
            aria-label="Project-relative source path"
            value={relativePath}
            onChange={(event) => setRelativePath(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") void approvePath();
            }}
            className="min-w-0 flex-1 rounded-sm border border-border bg-surface-0 px-3 py-2 font-mono text-meta text-foreground outline-none focus:border-primary"
            placeholder="AGENTS.md"
            spellCheck={false}
          />
          <Button
            size="sm"
            variant="outline"
            disabled={busy || !relativePath.trim()}
            onClick={() => void approvePath()}
          >
            {busy ? (
              <RefreshCw
                size={13}
                className="animate-spin motion-reduce:animate-none"
              />
            ) : (
              <FileCheck2 size={13} />
            )}
            Approve exact revision
          </Button>
        </div>
        <p className="mt-3 text-micro leading-5 text-muted-foreground">
          Approval does not grant filesystem, network, tool, or write
          permission. It only marks these exact bytes as user-authorized human
          intent for this project.
        </p>
      </section>

      <section>
        <div className="mb-3 flex items-end justify-between gap-3">
          <div>
            <h3 className="text-body font-semibold">Active approved sources</h3>
            <p className="mt-1 text-micro text-muted-foreground">
              {authority
                ? `${authority.current} current · ${authority.changed} changed · ${authority.missing} missing`
                : "Loading local authority…"}
            </p>
          </div>
        </div>
        <div className="space-y-2">
          {authority?.sources.map((item) => {
            const path =
              item.approval.projectRelativePath ?? item.approval.displayName;
            const projectFile = item.approval.sourceKind === "project-file";
            return (
              <div
                key={item.approval.sourceId}
                className="flex flex-col gap-3 rounded-sm border border-border bg-surface-1 px-4 py-3 sm:flex-row sm:items-center sm:justify-between"
              >
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <AuthorityState state={item.state} />
                    <span className="rounded-sm bg-surface-2 px-1.5 py-0.5 font-mono text-micro text-muted-foreground">
                      {projectFile ? "project file" : "migrated snapshot"}
                    </span>
                    <span className="truncate font-mono text-micro text-muted-foreground">
                      {item.approval.sourceId}
                    </span>
                  </div>
                  <p className="mt-1 truncate text-meta text-foreground">{path}</p>
                  <p className="mt-1 text-micro text-muted-foreground">
                    Approved{" "}
                    {new Date(
                      item.approval.approvedAtUnixMs,
                    ).toLocaleString()}
                  </p>
                </div>
                <div className="flex flex-wrap gap-2">
                  {projectFile && item.state === "current" &&
                    /\.(?:md|mdx)$/i.test(
                      item.approval.projectRelativePath ?? "",
                    ) && (
                      <Button
                        size="sm"
                        variant="outline"
                        disabled={busy}
                        onClick={() => void openSource(item.approval.sourceId)}
                      >
                        <ExternalLink size={13} aria-hidden="true" />
                        Open externally
                      </Button>
                    )}
                  {projectFile && item.state !== "missing" && (
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={busy}
                      onClick={() => void reapprove(item.approval.sourceId)}
                    >
                      Reapprove revision
                    </Button>
                  )}
                  <Button
                    size="sm"
                    variant="ghost"
                    disabled={busy}
                    onClick={() => void revoke(item.approval.sourceId)}
                  >
                    Revoke authority
                  </Button>
                </div>
              </div>
            );
          })}
          {authority && authority.sources.length === 0 && (
            <div className="rounded-sm border border-dashed border-border px-4 py-6 text-center text-meta text-muted-foreground">
              No active approved sources for this project yet.
            </div>
          )}
        </div>
      </section>

      {authority && authority.legacyIssues.length > 0 && (
        <section>
          <div className="mb-3 flex items-center gap-2">
            <AlertTriangle size={14} className="text-warning" />
            <div>
              <h3 className="text-body font-semibold">
                Legacy approvals needing review
              </h3>
              <p className="mt-1 text-micro text-muted-foreground">
                These old note-vault approvals were not migrated as authority
                because Ley could not prove the exact approved bytes.
              </p>
            </div>
          </div>
          <div className="space-y-2">
            {authority.legacyIssues.map((issue) => (
              <div
                key={issue.sourceId}
                className="flex flex-col gap-3 rounded-sm border border-warning/30 bg-warning/5 px-4 py-3 sm:flex-row sm:items-center sm:justify-between"
              >
                <div className="min-w-0">
                  <p className="truncate text-meta text-foreground">
                    {issue.displayName}
                  </p>
                  <p className="mt-1 font-mono text-micro text-muted-foreground">
                    {issue.reason} · {issue.sourceId}
                  </p>
                </div>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={busy}
                  onClick={() => void revoke(issue.sourceId)}
                >
                  Dismiss legacy approval
                </Button>
              </div>
            ))}
          </div>
        </section>
      )}

      {authority && (
        <p className="text-micro leading-5 text-muted-foreground">
          {authority.privacyNotice}
        </p>
      )}
    </div>
  );
}

function AuthorityState({ state }: { state: ApprovedSourceState }) {
  const current = state === "current";
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-sm px-1.5 py-0.5 font-mono text-micro",
        current ? "bg-success/10 text-success" : "bg-warning/10 text-warning",
      )}
    >
      {current ? <CheckCircle2 size={11} /> : <XCircle size={11} />}
      {state}
    </span>
  );
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
