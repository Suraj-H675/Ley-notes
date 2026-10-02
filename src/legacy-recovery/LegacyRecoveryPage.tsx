import { useState } from "react";
import { Archive, Database, Download, ShieldAlert, Trash2 } from "lucide-react";
import {
  buildLegacyBrowserArchive,
  downloadLegacyBrowserArchive,
  eraseLegacyBrowserData,
  inspectLegacyBrowserData,
  type LegacyRecoveryInspection,
} from "./legacy-browser-recovery";

const ERASE_CONFIRMATION = "ERASE LEGACY DATA";

export function LegacyRecoveryPage() {
  const [inspection, setInspection] = useState<LegacyRecoveryInspection | null>(null);
  const [busy, setBusy] = useState<"inspect" | "export" | "erase" | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmation, setConfirmation] = useState("");

  async function inspect() {
    setBusy("inspect");
    setError(null);
    setMessage(null);
    try {
      setInspection(await inspectLegacyBrowserData());
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(null);
    }
  }

  async function exportArchive() {
    setBusy("export");
    setError(null);
    setMessage(null);
    try {
      const archive = await buildLegacyBrowserArchive();
      const filename = downloadLegacyBrowserArchive(archive);
      setMessage(`Downloaded ${filename}. Keep it somewhere you trust before erasing browser data.`);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(null);
    }
  }

  async function erase() {
    if (confirmation !== ERASE_CONFIRMATION) return;
    setBusy("erase");
    setError(null);
    setMessage(null);
    try {
      await eraseLegacyBrowserData(() => {
        setMessage(
          "Erasure is waiting for another Ley tab on this site to close. The already-confirmed deletion will finish when that connection releases the database.",
        );
      });
      setConfirmation("");
      setInspection(await inspectLegacyBrowserData());
      setMessage("Legacy browser data erased from this site origin.");
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="min-h-screen bg-[#101114] px-5 py-10 text-[#eae7df] sm:py-16">
      <main className="mx-auto max-w-3xl">
        <a href="/" className="text-sm text-[#b4b1a9] hover:text-white">
          ← Back to Ley
        </a>
        <div className="mt-8 border border-white/9 bg-[#151619] p-6 sm:p-8">
          <div className="flex items-center gap-2 text-[#c2b28f]">
            <Archive size={16} aria-hidden="true" />
            <span className="font-mono text-xs uppercase tracking-[0.12em]">
              Migration-only recovery
            </span>
          </div>
          <h1 className="mt-4 font-serif text-4xl tracking-[-0.025em]">
            Recover retired browser-local Ley data
          </h1>
          <p className="mt-4 max-w-2xl leading-7 text-[#b4b1a9]">
            Older Ley browser builds could store human-authored notes, attachments,
            and revision history in this site's IndexedDB. The current Ley product
            does not use that database. This page exists only to inspect, export, or
            erase those historical bytes on the same browser origin.
          </p>
          <div className="mt-6 flex gap-3 border-l-2 border-[#c2b28f]/65 bg-white/[0.025] p-4 text-sm leading-6 text-[#aaa79f]">
            <ShieldAlert className="mt-0.5 shrink-0 text-[#c2b28f]" size={16} />
            <p>
              Nothing is read until you choose <strong className="text-[#eae7df]">Inspect</strong>.
              Nothing is uploaded. Recovered notebook data is human-owned archival data,
              not trusted Ley continuity or agent memory.
            </p>
          </div>
          <button
            type="button"
            disabled={busy !== null}
            onClick={() => void inspect()}
            className="mt-6 inline-flex h-10 items-center gap-2 bg-[#c2b28f] px-4 text-sm font-semibold text-[#15161a] disabled:opacity-50"
          >
            <Database size={14} aria-hidden="true" />
            {busy === "inspect" ? "Inspecting…" : "Inspect legacy browser data"}
          </button>
        </div>

        {inspection && (
          <section className="mt-6 border border-white/9 bg-[#151619] p-6 sm:p-8">
            <h2 className="font-serif text-2xl">Local recovery status</h2>
            <p className="mt-3 text-sm leading-6 text-[#b4b1a9]">
              {inspectionMessage(inspection)}
            </p>
            {inspection.status === "recoverable" && (
              <>
                <div className="mt-5 grid gap-3 sm:grid-cols-3">
                  <Metric label="Pages" value={inspection.pages} />
                  <Metric label="Assets" value={inspection.assets} />
                  <Metric label="Revisions" value={inspection.revisions} />
                </div>
                <p className="mt-4 text-xs leading-5 text-[#8f8c84]">
                  Binary asset payloads: {formatBytes(inspection.assetBytes)} before base64/JSON
                  overhead. Large archives may require additional browser memory while the download
                  is prepared.
                </p>
                <p className="mt-4 font-mono text-xs leading-5 text-[#8f8c84]">
                  Source: {inspection.source}
                  {inspection.ambiguousActiveBackup
                    ? " · archive will also retain the ambiguous active-table copy"
                    : ""}
                </p>
                <button
                  type="button"
                  disabled={busy !== null}
                  onClick={() => void exportArchive()}
                  className="mt-5 inline-flex h-10 items-center gap-2 border border-white/12 px-4 text-sm font-semibold text-[#eae7df] hover:bg-white/5 disabled:opacity-50"
                >
                  <Download size={14} aria-hidden="true" />
                  {busy === "export" ? "Preparing archive…" : "Export lossless JSON archive"}
                </button>
              </>
            )}
          </section>
        )}

        {inspection && inspection.status !== "absent" && (
          <section className="mt-6 border border-red-400/20 bg-[#151619] p-6 sm:p-8">
            <div className="flex items-center gap-2 text-red-300">
              <Trash2 size={15} aria-hidden="true" />
              <h2 className="font-semibold">Erase retired browser data</h2>
            </div>
            <p className="mt-3 text-sm leading-6 text-[#b4b1a9]">
              This permanently deletes the old <span className="font-mono">ley-notes</span>{" "}
              IndexedDB database for this website origin. It does not delete project folders,
              current Desktop continuity, backups, downloads, or copies in browser sync/profile backups.
            </p>
            <label className="mt-5 block max-w-md text-xs text-[#a5a29a]">
              Type <span className="font-mono text-[#eae7df]">{ERASE_CONFIRMATION}</span> to confirm
              <input
                value={confirmation}
                onChange={(event) => setConfirmation(event.target.value)}
                spellCheck={false}
                autoComplete="off"
                className="mt-2 h-10 w-full border border-white/12 bg-[#101114] px-3 font-mono text-sm text-[#eae7df] outline-none focus:border-red-300/60"
              />
            </label>
            <button
              type="button"
              disabled={busy !== null || confirmation !== ERASE_CONFIRMATION}
              onClick={() => void erase()}
              className="mt-4 inline-flex h-10 items-center gap-2 border border-red-300/30 px-4 text-sm font-semibold text-red-200 hover:bg-red-400/10 disabled:opacity-40"
            >
              <Trash2 size={14} aria-hidden="true" />
              {busy === "erase" ? "Erasing…" : "Permanently erase legacy data"}
            </button>
          </section>
        )}

        {message && (
          <p role="status" className="mt-5 border border-emerald-300/15 bg-emerald-300/[0.04] p-4 text-sm text-emerald-100">
            {message}
          </p>
        )}
        {error && (
          <p role="alert" className="mt-5 border border-red-300/20 bg-red-400/[0.05] p-4 text-sm text-red-200">
            {error}
          </p>
        )}
      </main>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <div className="border border-white/8 bg-[#111215] p-4">
      <p className="text-xs text-[#8f8c84]">{label}</p>
      <p className="mt-1 text-2xl font-semibold tabular-nums">{value.toLocaleString()}</p>
    </div>
  );
}

function inspectionMessage(inspection: LegacyRecoveryInspection): string {
  switch (inspection.status) {
    case "absent":
      return "No legacy Ley browser database exists on this site origin. Inspection did not create one.";
    case "empty":
      return "The retired Ley browser database exists, but it contains no recoverable browser-local pages, assets, or revisions.";
    case "filesystem-cache-only":
      return "Only a retired filesystem-vault projection is present. Ley will not mislabel that cache as authoritative browser-local notebook data.";
    case "recoverable":
      return "Recoverable browser-local notebook data was found. Export it before erasing if you may need these historical notes or attachments.";
  }
}

function errorMessage(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  return "Legacy browser-data recovery failed.";
}

function formatBytes(bytes: number): string {
  if (bytes < 1_024) return `${bytes} B`;
  if (bytes < 1_048_576) return `${(bytes / 1_024).toFixed(1)} KB`;
  if (bytes < 1_073_741_824) return `${(bytes / 1_048_576).toFixed(1)} MB`;
  return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
}
