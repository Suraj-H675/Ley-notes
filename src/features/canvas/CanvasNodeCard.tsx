import { memo, useEffect, useState } from "react";
import { ExternalLink, FileText } from "lucide-react";
import {
  Handle,
  NodeResizer,
  Position,
  useUpdateNodeInternals,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import {
  canvasColorValue,
  type CanvasNode,
  type CanvasSide,
} from "@/core/vault/canvas";
import {
  attachmentObjectUrl,
  isSafeAttachmentPath,
} from "@/core/vault/attachments";

export interface CanvasCardData extends Record<string, unknown> {
  node: CanvasNode;
  fileTitle?: string;
  onOpenFile: (path: string) => void;
  onUpdateText: (id: string, value: string) => void;
  onUpdateGroupLabel: (id: string, value: string) => void;
  onConnectClick: (id: string, side: CanvasSide) => void;
  pendingConnection?: { nodeId: string; side: CanvasSide };
}

export type CanvasFlowNode = Node<CanvasCardData, "canvasCard">;

export const CanvasNodeCard = memo(function CanvasNodeCard({
  id,
  data,
  selected,
}: NodeProps<CanvasFlowNode>) {
  const { node } = data;
  const updateNodeInternals = useUpdateNodeInternals();
  const color = canvasColorValue(node.color);
  const isGroup = node.type === "group";
  const safeUrl = node.type === "link" ? externalUrl(node.url) : null;
  const backgroundPath = isGroup ? node.background : undefined;
  const [backgroundUrl, setBackgroundUrl] = useState<string | null>(null);

  useEffect(() => {
    updateNodeInternals(id);
  }, [id, updateNodeInternals]);

  useEffect(() => {
    let active = true;
    let objectUrl: string | null = null;
    const loaded =
      backgroundPath && isSafeAttachmentPath(backgroundPath)
        ? attachmentObjectUrl(backgroundPath)
        : Promise.resolve(null);
    loaded
      .then((url) => {
        objectUrl = url;
        if (active) setBackgroundUrl(url);
        else if (url) URL.revokeObjectURL(url);
      })
      .catch(() => {
        if (active) setBackgroundUrl(null);
      });

    return () => {
      active = false;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [backgroundPath]);

  return (
    <>
      <div
        className={`pointer-events-none relative h-full w-full overflow-hidden rounded-md border bg-surface-1 text-foreground shadow-sm transition-shadow ${selected ? "shadow-lg ring-2 ring-primary/40" : ""}`}
        style={{
          borderColor: color ?? "hsl(var(--border))",
          backgroundColor: color
            ? `color-mix(in srgb, ${color} 12%, hsl(var(--surface-1)))`
            : undefined,
          ...(backgroundUrl
            ? {
                backgroundImage: `url("${backgroundUrl}")`,
                backgroundPosition: "center",
                backgroundRepeat:
                  node.type === "group" && node.backgroundStyle === "repeat"
                    ? "repeat"
                    : "no-repeat",
                backgroundSize:
                  node.type === "group" && node.backgroundStyle === "ratio"
                    ? "contain"
                    : node.type === "group" && node.backgroundStyle === "repeat"
                      ? "auto"
                      : "cover",
              }
            : {}),
        }}
      >
        <CanvasNodeBody node={node} data={data} safeUrl={safeUrl} />
      </div>
      <NodeResizer
        isVisible={selected}
        minWidth={isGroup ? 320 : 180}
        minHeight={isGroup ? 220 : 96}
        color={color ?? "hsl(var(--primary))"}
      />
      <CanvasNodeHandles
        nodeId={node.id}
        pendingConnection={data.pendingConnection}
        onConnectClick={data.onConnectClick}
      />
    </>
  );
});

function CanvasNodeBody({
  node,
  data,
  safeUrl,
}: {
  node: CanvasNode;
  data: CanvasCardData;
  safeUrl: string | null;
}) {
  return (
    <div className="h-full w-full">
      {node.type === "group" ? (
        <input
          value={node.label ?? ""}
          onChange={(event) =>
            data.onUpdateGroupLabel(node.id, event.target.value)
          }
          className="nodrag nowheel pointer-events-auto absolute left-3 top-2 max-w-[calc(100%-1.5rem)] bg-transparent text-meta font-semibold outline-none placeholder:text-muted-foreground"
          placeholder="Group label"
          aria-label="Canvas group label"
        />
      ) : node.type === "text" ? (
        <textarea
          className="nodrag nowheel pointer-events-auto h-full w-full resize-none bg-transparent p-3 text-meta leading-relaxed outline-none"
          value={node.text}
          aria-label="Canvas text card"
          onChange={(event) => data.onUpdateText(node.id, event.target.value)}
        />
      ) : node.type === "file" ? (
        <button
          type="button"
          className="nodrag pointer-events-auto flex h-full w-full flex-col items-start justify-center gap-1 p-3 text-left hover:bg-surface-2/70"
          onClick={() => data.onOpenFile(node.file)}
        >
          <span className="flex items-center gap-1.5 text-meta font-medium">
            <FileText size={14} className="text-secondary" />
            {data.fileTitle ?? node.file}
          </span>
          <span className="font-mono text-micro text-muted-foreground">
            {node.file}
            {node.subpath ?? ""}
          </span>
        </button>
      ) : safeUrl ? (
        <a
          className="nodrag pointer-events-auto flex h-full w-full flex-col items-start justify-center gap-1 p-3 hover:bg-surface-2/70"
          href={safeUrl}
          target="_blank"
          rel="noreferrer"
        >
          <span className="flex items-center gap-1.5 text-meta font-medium">
            <ExternalLink size={14} className="text-secondary" />
            {urlLabel(safeUrl)}
          </span>
          <span className="line-clamp-2 break-all font-mono text-micro text-muted-foreground">
            {node.url}
          </span>
        </a>
      ) : (
        <div className="flex h-full w-full flex-col justify-center gap-1 p-3">
          <span className="text-meta font-medium">Invalid link</span>
          <span className="break-all font-mono text-micro text-muted-foreground">
            {node.url}
          </span>
        </div>
      )}
    </div>
  );
}

function CanvasNodeHandles({
  nodeId,
  pendingConnection,
  onConnectClick,
}: {
  nodeId: string;
  pendingConnection?: { nodeId: string; side: CanvasSide };
  onConnectClick: (id: string, side: CanvasSide) => void;
}) {
  return (
    <>
      {(["top", "right", "bottom", "left"] as const).map((side) => (
        <SideHandle
          key={side}
          side={side}
          position={positionForSide(side)}
          active={isHandleActive(pendingConnection, nodeId, side)}
          onClick={() => onConnectClick(nodeId, side)}
        />
      ))}
    </>
  );
}

function positionForSide(side: CanvasSide): Position {
  return side === "top"
    ? Position.Top
    : side === "right"
      ? Position.Right
      : side === "bottom"
        ? Position.Bottom
        : Position.Left;
}

function isHandleActive(
  pendingConnection: { nodeId: string; side: CanvasSide } | undefined,
  nodeId: string,
  side: CanvasSide,
): boolean {
  return (
    pendingConnection?.nodeId === nodeId && pendingConnection.side === side
  );
}

function SideHandle({
  side,
  position,
  active,
  onClick,
}: {
  side: CanvasSide;
  position: Position;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <Handle
      id={side}
      type="source"
      position={position}
      className={`!z-10 !size-2.5 !border-2 !border-surface-1 ${active ? "!scale-125 !bg-primary" : "!bg-muted-foreground"}`}
      style={handleInset(position)}
      isConnectable
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        onClick();
        if (!active) return;
        event.preventDefault();
        event.stopPropagation();
      }}
      aria-label={`${active ? "Cancel connection from" : "Connect from"} ${side} handle`}
    />
  );
}

function handleInset(position: Position): {
  top?: number;
  right?: number;
  bottom?: number;
  left?: number;
} {
  if (position === Position.Top) return { top: 4 };
  if (position === Position.Right) return { right: 4 };
  if (position === Position.Bottom) return { bottom: 4 };
  return { left: 4 };
}

function externalUrl(value: string): string | null {
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:"
      ? url.href
      : null;
  } catch {
    return null;
  }
}

function urlLabel(value: string): string {
  try {
    return new URL(value).hostname.replace(/^www\./, "");
  } catch {
    return value;
  }
}
