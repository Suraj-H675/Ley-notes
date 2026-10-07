import { invoke } from "@tauri-apps/api/core";

export type ChronicleCaptureMode = "minimal" | "structured" | "full-evidence";

export interface ChronicleCaptureTarget {
  projectId: string;
  locatorId: string;
  localPath: string;
  mode: ChronicleCaptureMode | null;
  grantId: string | null;
  authorized: boolean;
  effective: boolean;
  inactiveReason: string | null;
}

export function readChronicleCaptureTargets(
  projectId: string,
): Promise<ChronicleCaptureTarget[]> {
  return invoke("read_chronicle_capture_targets", { projectId });
}

export function requestChronicleCapture(
  target: ChronicleCaptureTarget,
  mode: ChronicleCaptureMode,
): Promise<boolean> {
  return invoke("request_chronicle_capture", {
    projectId: target.projectId,
    locatorId: target.locatorId,
    mode,
  });
}

export function revokeChronicleCapture(
  target: ChronicleCaptureTarget,
): Promise<void> {
  if (!target.grantId) {
    return Promise.reject(
      new Error("Codex capture has no retained grant to revoke."),
    );
  }
  return invoke("revoke_chronicle_capture", {
    projectId: target.projectId,
    locatorId: target.locatorId,
    expectedGrantId: target.grantId,
  });
}
