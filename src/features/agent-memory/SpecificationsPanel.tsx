import { useEffect, useMemo, useState } from "react";
import {
  CheckCircle2,
  FileCheck2,
  RefreshCw,
  ShieldCheck,
  XCircle,
} from "lucide-react";
import { updatePageFrontmatter } from "@/core/vault/pages";
import type { Page } from "@/infrastructure/database/schema";
import { Button } from "@/shared/components/Button";
import { cn } from "@/shared/lib/classnames";
import {
  approveAgentProjectSpecification,
  readAgentProjectSpecifications,
  revokeAgentProjectSpecification,
  verifyAgentProjectNoteVault,
} from "./api";
import type {
  SpecificationApprovalState,
  SpecificationAuthorityList,
} from "./types";

export function SpecificationsPanel({
  projectPath,
  vaultPath,
  activeNote,
}: {
  projectPath: string;
  vaultPath: string;
  activeNote?: Page;
}) {
  const [authority, setAuthority] = useState<SpecificationAuthorityList | null>(
    null,
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void readAgentProjectSpecifications(projectPath)
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

  const activeSpecificationId = useMemo(
    () => specificationIdFromNote(activeNote),
    [activeNote],
  );
  const activeApproval = authority?.specifications.find(
    (item) =>
      item.approval.specificationId === activeSpecificationId ||
      item.approval.relativePath === activeNote?.path,
  );

  async function approveCurrentNote() {
    if (!activeNote) return;
    setBusy(true);
    setError(null);
    try {
      await verifyAgentProjectNoteVault(projectPath, vaultPath);
      const existingType = activeNote.frontmatter["ley-type"];
      if (existingType !== undefined && existingType !== "specification") {
        throw new Error(
          `This note already uses ley-type: ${String(existingType)}. Remove or change that property deliberately before approving it as a Specification.`,
        );
      }
      const rawId = activeNote.frontmatter["ley-spec-id"];
      if (rawId !== undefined && specificationIdFromValue(rawId) === null) {
        throw new Error(
          "This note has an invalid ley-spec-id. Fix or remove it before approving the Specification.",
        );
      }
      const specificationId =
        specificationIdFromValue(rawId) ??
        activeApproval?.approval.specificationId ??
        createSpecificationId();
      if (existingType !== "specification" || rawId !== specificationId) {
        await updatePageFrontmatter(activeNote.id, {
          ...activeNote.frontmatter,
          "ley-type": "specification",
          "ley-spec-id": specificationId,
        });
      }
      setAuthority(
        await approveAgentProjectSpecification(
          projectPath,
          vaultPath,
          specificationId,
          activeNote.path,
        ),
      );
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  async function revoke(specificationId: string) {
    setBusy(true);
    setError(null);
    try {
      setAuthority(
        await revokeAgentProjectSpecification(
          projectPath,
          vaultPath,
          specificationId,
        ),
      );
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
          Specifications
        </h2>
        <p className="mt-2 text-meta leading-6 text-muted-foreground">
          Specifications are ordinary Markdown notes whose exact revision you
          explicitly approve as requirements for this project. YAML alone never
          grants authority, and editing an approved note makes that approval
          stale until you review it again.
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
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div className="min-w-0">
            <h3 className="text-body font-semibold">Current note</h3>
            {activeNote ? (
              <>
                <p className="mt-1 truncate text-meta text-foreground">
                  {activeNote.title}
                </p>
                <p className="mt-0.5 truncate font-mono text-micro text-muted-foreground">
                  {activeNote.path}
                </p>
                {activeApproval && (
                  <div className="mt-3">
                    <AuthorityState state={activeApproval.state} />
                  </div>
                )}
              </>
            ) : (
              <p className="mt-1 text-meta text-muted-foreground">
                Open the Markdown note that contains the requirements you want
                to authorize.
              </p>
            )}
          </div>
          <Button
            size="sm"
            variant="outline"
            disabled={
              !activeNote || Boolean(activeNote.frontmatterError) || busy
            }
            onClick={() => void approveCurrentNote()}
          >
            {busy ? (
              <RefreshCw
                size={13}
                className="animate-spin motion-reduce:animate-none"
              />
            ) : (
              <FileCheck2 size={13} />
            )}
            {activeApproval?.state === "current"
              ? "Reapprove revision"
              : "Approve current note"}
          </Button>
        </div>
        {activeNote?.frontmatterError && (
          <p className="mt-3 text-micro text-destructive">
            Fix this note’s invalid YAML frontmatter in Source mode before
            approving it.
          </p>
        )}
        <p className="mt-4 text-micro leading-5 text-muted-foreground">
          Approving makes this exact note revision available to connected agents
          for this project. If the host uses a cloud model, the host may send
          the returned Specification text to that provider. Revoking authority
          does not delete the note.
        </p>
        <p className="mt-2 text-micro leading-5 text-muted-foreground">
          Keep acceptance criteria and verification methods in readable Markdown
          (for example, <code>## Acceptance criteria</code> and{" "}
          <code>## Verification</code>). Ley stores the approval separately so
          agent-written text cannot authorize itself.
        </p>
      </section>

      <section>
        <div className="mb-3 flex items-end justify-between gap-3">
          <div>
            <h3 className="text-body font-semibold">Approved notes</h3>
            <p className="mt-1 text-micro text-muted-foreground">
              {authority
                ? `${authority.current} current · ${authority.changed} changed · ${authority.missing} missing`
                : "Loading local approvals…"}
            </p>
          </div>
        </div>
        <div className="space-y-2">
          {authority?.specifications.map((item) => (
            <div
              key={item.approval.specificationId}
              className="flex flex-col gap-3 rounded-sm border border-border bg-surface-1 px-4 py-3 sm:flex-row sm:items-center sm:justify-between"
            >
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <AuthorityState state={item.state} />
                  <span className="truncate font-mono text-micro text-muted-foreground">
                    {item.approval.specificationId}
                  </span>
                </div>
                <p className="mt-1 truncate text-meta text-foreground">
                  {item.approval.relativePath}
                </p>
                <p className="mt-1 text-micro text-muted-foreground">
                  Approved{" "}
                  {new Date(item.approval.approvedAtUnixMs).toLocaleString()}
                </p>
              </div>
              <Button
                size="sm"
                variant="ghost"
                disabled={busy}
                onClick={() => void revoke(item.approval.specificationId)}
              >
                Revoke authority
              </Button>
            </div>
          ))}
          {authority && authority.specifications.length === 0 && (
            <div className="rounded-sm border border-dashed border-border px-4 py-6 text-center text-meta text-muted-foreground">
              No Specification revisions are approved for this project yet.
            </div>
          )}
        </div>
      </section>

      {authority && (
        <p className="text-micro leading-5 text-muted-foreground">
          {authority.privacyNotice}
        </p>
      )}
    </div>
  );
}

function AuthorityState({ state }: { state: SpecificationApprovalState }) {
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

function specificationIdFromNote(note?: Page): string | null {
  if (!note || note.frontmatter["ley-type"] !== "specification") return null;
  return specificationIdFromValue(note.frontmatter["ley-spec-id"]);
}

function specificationIdFromValue(value: unknown): string | null {
  return typeof value === "string" && /^spec_[0-9a-f]{32}$/.test(value)
    ? value
    : null;
}

function createSpecificationId(): string {
  const uuid = globalThis.crypto.randomUUID();
  return `spec_${uuid.replaceAll("-", "").toLowerCase()}`;
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
