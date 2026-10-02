import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SpecificationsPanel } from "./SpecificationsPanel";
import type { ApprovedSourceAuthorityList } from "./types";

const api = vi.hoisted(() => ({
  approve: vi.fn(),
  openSource: vi.fn(),
  read: vi.fn(),
  reapprove: vi.fn(),
  revoke: vi.fn(),
}));

vi.mock("./api", () => ({
  approveAgentProjectFileSource: api.approve,
  openAgentProjectMarkdownSource: api.openSource,
  readAgentProjectApprovedSources: api.read,
  reapproveAgentProjectFileSource: api.reapprove,
  revokeAgentProjectApprovedSource: api.revoke,
}));

const emptyAuthority: ApprovedSourceAuthorityList = {
  projectId: "prj_test",
  sources: [],
  current: 0,
  changed: 0,
  missing: 0,
  legacyIssues: [],
  privacyNotice: "Only explicit approved-source authority is stored privately.",
};

describe("SpecificationsPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.read.mockResolvedValue(emptyAuthority);
  });

  it("approves an exact project-relative source without a vault or caller-generated id", async () => {
    const approved: ApprovedSourceAuthorityList = {
      ...emptyAuthority,
      current: 1,
      sources: [
        {
          approval: {
            projectId: "prj_test",
            sourceId: "spec_12345678123441238123123456789abc",
            sourceKind: "project-file",
            displayName: "AGENTS.md",
            projectRelativePath: "AGENTS.md",
            contentHash: `sha256:${"a".repeat(64)}`,
            approvedAtUnixMs: 1_700_000_000_000,
          },
          state: "current",
          currentContentHash: `sha256:${"a".repeat(64)}`,
        },
      ],
    };
    api.approve.mockResolvedValue(approved);

    render(<SpecificationsPanel projectPath="/projects/ley" />);

    const input = await screen.findByRole("textbox", {
      name: "Project-relative source path",
    });
    expect(input).toHaveValue("AGENTS.md");
    fireEvent.click(
      screen.getByRole("button", { name: "Approve exact revision" }),
    );

    await waitFor(() => {
      expect(api.approve).toHaveBeenCalledWith("/projects/ley", "AGENTS.md");
    });
    expect((await screen.findAllByText("current")).length).toBeGreaterThanOrEqual(
      1,
    );
    expect(screen.getByText("project file")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Open externally" }));
    await waitFor(() => {
      expect(api.openSource).toHaveBeenCalledWith(
        "/projects/ley",
        "spec_12345678123441238123123456789abc",
      );
    });
  });

  it("reapproves and revokes a changed project-file authority", async () => {
    const changed: ApprovedSourceAuthorityList = {
      ...emptyAuthority,
      changed: 1,
      sources: [
        {
          approval: {
            projectId: "prj_test",
            sourceId: "spec_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            sourceKind: "project-file",
            displayName: "docs/requirements.md",
            projectRelativePath: "docs/requirements.md",
            contentHash: `sha256:${"a".repeat(64)}`,
            approvedAtUnixMs: 1_700_000_000_000,
          },
          state: "changed",
          currentContentHash: `sha256:${"b".repeat(64)}`,
        },
      ],
    };
    const refreshed: ApprovedSourceAuthorityList = {
      ...changed,
      current: 1,
      changed: 0,
      sources: changed.sources.map((source) => ({
        ...source,
        state: "current",
        approval: {
          ...source.approval,
          contentHash: `sha256:${"b".repeat(64)}`,
        },
      })),
    };
    api.read.mockResolvedValue(changed);
    api.reapprove.mockResolvedValue(refreshed);
    api.revoke.mockResolvedValue(emptyAuthority);

    render(<SpecificationsPanel projectPath="/projects/ley" />);

    expect((await screen.findAllByText("changed")).length).toBeGreaterThanOrEqual(
      1,
    );
    expect(
      screen.queryByRole("button", { name: "Open externally" }),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Reapprove revision" }));
    await waitFor(() => {
      expect(api.reapprove).toHaveBeenCalledWith(
        "/projects/ley",
        "spec_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      );
    });

    fireEvent.click(
      await screen.findByRole("button", { name: "Revoke authority" }),
    );
    await waitFor(() => {
      expect(api.revoke).toHaveBeenCalledWith(
        "/projects/ley",
        "spec_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      );
    });
    expect(
      await screen.findByText(/No active approved sources/),
    ).toBeVisible();
  });

  it("keeps unresolved legacy approvals separate from active authority", async () => {
    const authority: ApprovedSourceAuthorityList = {
      ...emptyAuthority,
      legacyIssues: [
        {
          projectId: "prj_test",
          sourceId: "spec_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
          displayName: "Specs/Old.md",
          approvedContentHash: `sha256:${"c".repeat(64)}`,
          approvedAtUnixMs: 1_700_000_000_000,
          reason: "changed",
        },
      ],
    };
    api.read.mockResolvedValue(authority);
    api.revoke.mockResolvedValue(emptyAuthority);

    render(<SpecificationsPanel projectPath="/projects/ley" />);

    expect(
      await screen.findByText("Legacy approvals needing review"),
    ).toBeVisible();
    expect(screen.getByText("Specs/Old.md")).toBeVisible();
    fireEvent.click(
      screen.getByRole("button", { name: "Dismiss legacy approval" }),
    );
    await waitFor(() => {
      expect(api.revoke).toHaveBeenCalledWith(
        "/projects/ley",
        "spec_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      );
    });
  });
});
