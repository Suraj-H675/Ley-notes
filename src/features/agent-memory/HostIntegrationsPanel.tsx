import { useEffect, useState } from "react";
import {
  Cable,
  CheckCircle2,
  CircleAlert,
  RefreshCw,
  Unplug,
} from "lucide-react";
import { Button } from "@/shared/components/Button";
import { cn } from "@/shared/lib/classnames";
import {
  connectAgentHost,
  disconnectAgentHost,
  inspectAgentProject,
  readAgentHostIntegrations,
} from "./api";
import type { AgentHostIntegrationStatus } from "./types";

export function HostIntegrationsPanel({
  projectPath,
  compact = false,
}: {
  projectPath: string;
  compact?: boolean;
}) {
  const [hosts, setHosts] = useState<AgentHostIntegrationStatus[] | null>(null);
  const [busyAction, setBusyAction] = useState<{
    hostId: AgentHostIntegrationStatus["id"];
    action: "connect" | "check" | "disconnect";
  } | null>(null);
  const [activityCheck, setActivityCheck] = useState<
    Partial<Record<AgentHostIntegrationStatus["id"], "observed" | "missing">>
  >({});
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setError(null);
    try {
      setHosts(await readAgentHostIntegrations(projectPath));
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }

  useEffect(() => {
    let current = true;
    void readAgentHostIntegrations(projectPath)
      .then((next) => {
        if (current) setHosts(next);
      })
      .catch((cause) => {
        if (current) setError(errorMessage(cause));
      });
    return () => {
      current = false;
    };
  }, [projectPath]);

  async function connect(host: AgentHostIntegrationStatus) {
    setBusyAction({ hostId: host.id, action: "connect" });
    setError(null);
    try {
      const next = await connectAgentHost(projectPath, host.id);
      setHosts((current) =>
        (current ?? []).map((item) => (item.id === next.id ? next : item)),
      );
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusyAction(null);
    }
  }

  async function checkActivity(host: AgentHostIntegrationStatus) {
    setBusyAction({ hostId: host.id, action: "check" });
    setError(null);
    try {
      const inspection = await inspectAgentProject(projectPath);
      if (inspection.status !== "ready") {
        throw new Error("Ley project memory is not ready to verify host activity.");
      }
      const expectedHost = host.id;
      const observed = inspection.dashboard.sessions.some(
        (session) =>
          session.sourceKind === "host-hook" &&
          session.sourceHost?.trim().toLowerCase() === expectedHost,
      );
      setActivityCheck((current) => ({
        ...current,
        [host.id]: observed ? "observed" : "missing",
      }));
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusyAction(null);
    }
  }

  async function disconnect(host: AgentHostIntegrationStatus) {
    setBusyAction({ hostId: host.id, action: "disconnect" });
    setError(null);
    try {
      const next = await disconnectAgentHost(projectPath, host.id);
      setHosts((current) =>
        (current ?? []).map((item) => (item.id === next.id ? next : item)),
      );
      setActivityCheck((current) => {
        const nextActivity = { ...current };
        delete nextActivity[host.id];
        return nextActivity;
      });
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <section aria-labelledby="agent-integrations-title">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2
            id="agent-integrations-title"
            className={cn(
              "font-semibold tracking-tight",
              compact ? "text-body" : "text-lg",
            )}
          >
            Coding agents
          </h2>
          <p className="mt-1 max-w-3xl text-micro leading-5 text-muted-foreground">
            Connect Ley to a detected local host. Ley uses its packaged native
            helper; no Cargo install, Node runtime, or PATH edit is required.
          </p>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => void refresh()}
          disabled={busyAction !== null}
          title="Refresh coding-agent detection"
          aria-label="Refresh coding-agent detection"
        >
          <RefreshCw size={13} aria-hidden="true" />
        </Button>
      </div>

      <div className="mt-3 divide-y divide-border border-y border-border">
        {hosts === null ? (
          <div className="flex items-center gap-2 py-4 text-meta text-muted-foreground">
            <RefreshCw
              size={14}
              className="animate-spin motion-reduce:animate-none"
              aria-hidden="true"
            />
            Checking Codex and Claude Code on this device…
          </div>
        ) : (
          hosts.map((host) => (
            <HostIntegrationRow
              key={host.id}
              host={host}
              busyAction={
                busyAction?.hostId === host.id ? busyAction.action : null
              }
              activityCheck={activityCheck[host.id]}
              onConnect={() => void connect(host)}
              onCheckActivity={() => void checkActivity(host)}
              onDisconnect={() => void disconnect(host)}
            />
          ))
        )}
      </div>

      {error && (
        <div
          role="alert"
          className="mt-3 flex items-start gap-2 text-micro leading-5 text-destructive"
        >
          <CircleAlert size={14} className="mt-0.5 shrink-0" aria-hidden="true" />
          <span>{error}</span>
        </div>
      )}
      <p className="mt-3 text-micro leading-5 text-muted-foreground">
        “Configured” describes host/plugin configuration only. Actual Ley
        activity is recorded separately after a hook or MCP session reaches Ley.
      </p>
    </section>
  );
}

function HostIntegrationRow({
  host,
  busyAction,
  activityCheck,
  onConnect,
  onCheckActivity,
  onDisconnect,
}: {
  host: AgentHostIntegrationStatus;
  busyAction: "connect" | "check" | "disconnect" | null;
  activityCheck?: "observed" | "missing";
  onConnect: () => void;
  onCheckActivity: () => void;
  onDisconnect: () => void;
}) {
  const configured = host.managedByLeyDesktop && host.configured === true;
  const busy = busyAction !== null;
  const { Icon, statusLabel, iconClassName } = hostIntegrationPresentation(
    host,
    configured,
  );

  return (
    <div className="flex flex-col gap-3 py-3.5 sm:flex-row sm:items-center sm:justify-between">
      <div className="flex min-w-0 items-start gap-3">
        <span
          className={cn(
            "mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-md",
            iconClassName,
          )}
        >
          <Icon size={15} aria-hidden="true" />
        </span>
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <p className="text-meta font-semibold">{host.displayName}</p>
            <span className="text-micro font-medium text-muted-foreground">
              {statusLabel}
            </span>
            {host.restartRequired && (
              <span className="text-micro text-warning">Restart required</span>
            )}
            {host.reviewRequired && (
              <span className="text-micro text-warning">Review hooks</span>
            )}
          </div>
          <p className="mt-0.5 max-w-2xl text-micro leading-5 text-muted-foreground">
            {host.statusDetail}
          </p>
          {host.version && (
            <p className="mt-0.5 font-mono text-micro text-subtle-foreground">
              {host.version}
            </p>
          )}
          {activityCheck === "observed" && (
            <p className="mt-1 text-micro font-medium text-success">
              Smoke check passed: Ley retained a {host.displayName} hook event
              for this project.
            </p>
          )}
          {activityCheck === "missing" && (
            <p className="mt-1 text-micro text-warning">
              No {host.displayName} hook activity is retained yet. Start a new
              session in this project after restart/trust review, then check
              again.
            </p>
          )}
        </div>
      </div>
      {host.detected && !configured ? (
        <Button
          variant="outline"
          size="sm"
          disabled={busy}
          onClick={onConnect}
          className="self-start sm:self-auto"
        >
          {busy ? (
            <RefreshCw
              size={13}
              className="animate-spin motion-reduce:animate-none"
              aria-hidden="true"
            />
          ) : (
            <Cable size={13} aria-hidden="true" />
          )}
          {busy ? `Connecting ${host.displayName}…` : `Connect ${host.displayName}`}
        </Button>
      ) : configured ? (
        <div className="flex items-center gap-1 self-start sm:self-auto">
          <Button
            variant="ghost"
            size="sm"
            disabled={busy}
            onClick={onCheckActivity}
            aria-label={`Run ${host.displayName} smoke check`}
          >
            {busyAction === "check" ? (
              <RefreshCw
                size={13}
                className="animate-spin motion-reduce:animate-none"
                aria-hidden="true"
              />
            ) : (
              <CheckCircle2 size={13} aria-hidden="true" />
            )}
            {busyAction === "check" ? "Checking activity…" : "Run smoke check"}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            disabled={busy}
            onClick={onDisconnect}
            aria-label={`Disconnect ${host.displayName}`}
          >
            {busyAction === "disconnect" ? (
              <RefreshCw
                size={13}
                className="animate-spin motion-reduce:animate-none"
                aria-hidden="true"
              />
            ) : (
              <Unplug size={13} aria-hidden="true" />
            )}
            {busyAction === "disconnect" ? "Disconnecting…" : "Disconnect"}
          </Button>
        </div>
      ) : null}
    </div>
  );
}

function hostIntegrationPresentation(
  host: AgentHostIntegrationStatus,
  configured: boolean,
) {
  if (configured) {
    return {
      Icon: CheckCircle2,
      statusLabel: host.enabled ? "Configured" : "Installed · disabled",
      iconClassName: "bg-success/10 text-success",
    };
  }
  if (host.detected) {
    return {
      Icon: Cable,
      statusLabel: "Detected",
      iconClassName: "bg-primary/10 text-primary",
    };
  }
  return {
    Icon: Unplug,
    statusLabel: "Not found",
    iconClassName: "bg-surface-2 text-muted-foreground",
  };
}

function errorMessage(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  return "Could not update the coding-agent integration.";
}
