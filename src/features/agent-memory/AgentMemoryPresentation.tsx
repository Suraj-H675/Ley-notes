import { AlertTriangle, History } from "lucide-react";
import { cn } from "@/shared/lib/classnames";
import type { LearningSummary, ResumeSession } from "./types";

export function PageHeading({
  id,
  eyebrow,
  title,
  description,
}: {
  id?: string;
  eyebrow: string;
  title: string;
  description: string;
}) {
  return (
    <div>
      <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
        {eyebrow}
      </p>
      <h2 id={id} className="mt-1 text-2xl font-semibold tracking-[-0.035em]">
        {title}
      </h2>
      <p className="mt-2 max-w-2xl text-body leading-6 text-muted-foreground-strong">
        {description}
      </p>
    </div>
  );
}

export function StatusPill({
  tone,
  label,
  icon: Icon,
}: {
  tone: "success" | "warning" | "neutral";
  label: string;
  icon: typeof History;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-sm border px-2 py-0.5 text-micro font-medium",
        tone === "success" && "border-success/20 bg-success/10 text-success",
        tone === "warning" && "border-warning/20 bg-warning/10 text-warning",
        tone === "neutral" &&
          "border-border bg-surface-2 text-muted-foreground-strong",
      )}
    >
      <Icon size={11} />
      {label}
    </span>
  );
}

export function TrustDot({
  learning,
}: {
  learning: Pick<LearningSummary, "trustState" | "freshness">;
}) {
  const trusted =
    learning.trustState === "trusted" && learning.freshness === "current";
  const rejected = learning.trustState === "rejected";
  return (
    <span
      className={cn(
        "mt-1.5 size-2.5 shrink-0 rounded-full ring-4",
        trusted
          ? "bg-success ring-success/10"
          : rejected
            ? "bg-destructive ring-destructive/10"
            : "bg-warning ring-warning/10",
      )}
    />
  );
}

export function LargeEmpty({
  icon: Icon,
  title,
  body,
}: {
  icon: typeof History;
  title: string;
  body: string;
}) {
  return (
    <div className="rounded-sm border border-dashed border-border bg-surface-1/45 px-6 py-14 text-center">
      <Icon size={22} className="mx-auto text-subtle-foreground" />
      <h3 className="mt-3 text-meta font-semibold text-foreground">{title}</h3>
      <p className="mx-auto mt-1 max-w-md text-meta leading-relaxed text-muted-foreground">
        {body}
      </p>
    </div>
  );
}

export function CompactEmpty({
  icon: Icon,
  title,
  body,
}: {
  icon: typeof History;
  title: string;
  body: string;
}) {
  return (
    <div className="px-5 py-6 text-center">
      <Icon size={18} className="mx-auto text-subtle-foreground" />
      <p className="mt-2 text-meta font-medium">{title}</p>
      <p className="mx-auto mt-1 max-w-sm text-micro leading-5 text-muted-foreground">
        {body}
      </p>
    </div>
  );
}

export function ErrorNotice({ message }: { message: string }) {
  return (
    <div
      role="alert"
      className="flex gap-2 rounded-md border border-destructive/25 bg-destructive/10 px-3 py-2 text-meta text-destructive"
    >
      <AlertTriangle size={15} className="mt-0.5 shrink-0" />
      <span>{message}</span>
    </div>
  );
}

export function KnowledgeSurfaceFallback() {
  return (
    <div
      className="flex min-h-80 items-center justify-center rounded-md border border-border bg-surface-1 text-meta text-muted-foreground"
      aria-label="Loading project knowledge"
    >
      Loading local project knowledge…
    </div>
  );
}

export function relativeTime(unixMs: number): string {
  const deltaSeconds = Math.round((unixMs - Date.now()) / 1000);
  const formatter = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });
  if (Math.abs(deltaSeconds) < 60)
    return formatter.format(deltaSeconds, "second");
  const deltaMinutes = Math.round(deltaSeconds / 60);
  if (Math.abs(deltaMinutes) < 60)
    return formatter.format(deltaMinutes, "minute");
  const deltaHours = Math.round(deltaMinutes / 60);
  if (Math.abs(deltaHours) < 24) return formatter.format(deltaHours, "hour");
  const deltaDays = Math.round(deltaHours / 24);
  if (Math.abs(deltaDays) < 30) return formatter.format(deltaDays, "day");
  const deltaMonths = Math.round(deltaDays / 30);
  if (Math.abs(deltaMonths) < 12) return formatter.format(deltaMonths, "month");
  return formatter.format(Math.round(deltaMonths / 12), "year");
}

export function absoluteTime(unixMs: number): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(unixMs);
}

export function humanize(value: string): string {
  return value
    .replace(/-/g, " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

export function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

export function SessionStatus({
  status,
  compact = false,
}: {
  status: ResumeSession["status"];
  compact?: boolean;
}) {
  const color =
    status === "active"
      ? "bg-success"
      : status === "paused"
        ? "bg-warning"
        : status === "completed"
          ? "bg-primary"
          : "bg-subtle-foreground";
  if (compact)
    return (
      <span
        className={cn("block size-2 rounded-full", color)}
        title={humanize(status)}
      />
    );
  return (
    <span className="inline-flex items-center gap-1.5 text-micro font-medium text-muted-foreground-strong">
      <span className={cn("size-2 rounded-full", color)} />
      {humanize(status)}
    </span>
  );
}
