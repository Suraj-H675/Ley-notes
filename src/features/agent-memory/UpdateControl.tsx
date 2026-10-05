import { useEffect, useState } from "react";
import { Download, RefreshCw } from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { Button } from "@/shared/components/Button";

type UpdateState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "current" }
  | { kind: "available"; update: Update }
  | { kind: "installing"; version: string }
  | { kind: "installed"; version: string }
  | { kind: "error"; message: string };

export function UpdateControl({ showVersion = false }: { showVersion?: boolean }) {
  const [state, setState] = useState<UpdateState>({ kind: "idle" });
  const [currentVersion, setCurrentVersion] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void getVersion()
      .then((version) => {
        if (current) setCurrentVersion(version);
      })
      .catch(() => {
        // Version display is helpful context, but an IPC read failure should
        // not disable the explicit update action or invent a version.
      });
    return () => {
      current = false;
    };
  }, []);

  async function checkForUpdates() {
    setState({ kind: "checking" });
    try {
      const update = await check();
      setState(update ? { kind: "available", update } : { kind: "current" });
    } catch (cause) {
      setState({ kind: "error", message: errorMessage(cause) });
    }
  }

  async function install(update: Update) {
    setState({ kind: "installing", version: update.version });
    try {
      await update.downloadAndInstall();
      setState({ kind: "installed", version: update.version });
    } catch (cause) {
      setState({ kind: "error", message: errorMessage(cause) });
    }
  }

  if (state.kind === "available") {
    return (
      <Button
        variant="outline"
        size="sm"
        onClick={() => void install(state.update)}
        title={`Install Ley ${state.update.version}`}
      >
        <Download size={13} aria-hidden="true" />
        Install {state.update.version}
      </Button>
    );
  }

  return (
    <div className="flex flex-col items-end gap-1">
      {showVersion && currentVersion && (
        <span className="text-micro text-muted-foreground">
          Ley Desktop {currentVersion}
        </span>
      )}
      <Button
        variant="ghost"
        size="sm"
        onClick={() => void checkForUpdates()}
        disabled={state.kind === "checking" || state.kind === "installing"}
      >
        <RefreshCw
          size={13}
          aria-hidden="true"
          className={
            state.kind === "checking" || state.kind === "installing"
              ? "animate-spin motion-reduce:animate-none"
              : undefined
          }
        />
        {labelForState(state)}
      </Button>
      {state.kind === "installed" && (
        <span className="max-w-56 text-right text-micro text-success">
          Ley {state.version} is installed. Restart Ley to use it.
        </span>
      )}
      {state.kind === "error" && (
        <span
          className="max-w-64 text-right text-micro leading-4 text-destructive"
          role="alert"
        >
          {state.message}
        </span>
      )}
    </div>
  );
}

function labelForState(state: UpdateState): string {
  switch (state.kind) {
    case "checking":
      return "Checking…";
    case "current":
      return "Up to date";
    case "installing":
      return `Installing ${state.version}…`;
    case "installed":
      return "Update installed";
    default:
      return "Check for updates";
  }
}

function errorMessage(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  return "Ley could not check for updates.";
}
