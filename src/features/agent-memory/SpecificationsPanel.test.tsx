import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Page } from "@/infrastructure/database/schema";
import { SpecificationsPanel } from "./SpecificationsPanel";
import type { SpecificationAuthorityList } from "./types";

const api = vi.hoisted(() => ({
  approve: vi.fn(),
  read: vi.fn(),
  revoke: vi.fn(),
  verifyVault: vi.fn(),
  updateFrontmatter: vi.fn(),
}));

vi.mock("./api", () => ({
  approveAgentProjectSpecification: api.approve,
  readAgentProjectSpecifications: api.read,
  revokeAgentProjectSpecification: api.revoke,
  verifyAgentProjectNoteVault: api.verifyVault,
}));

vi.mock("@/core/vault/pages", () => ({
  updatePageFrontmatter: api.updateFrontmatter,
}));

const emptyAuthority: SpecificationAuthorityList = {
  projectId: "prj_test",
  specifications: [],
  current: 0,
  changed: 0,
  missing: 0,
  privacyNotice: "Only approval metadata is stored privately.",
};

const activeNote: Page = {
  id: "page_spec",
  title: "Offline product",
  lcTitle: "offline product",
  path: "Specs/Offline product.md",
  content: "# Offline product\n\n## Acceptance criteria\n\n- Works offline.\n",
  frontmatter: { owner: "Suraj" },
  aliases: [],
  createdAt: 1,
  updatedAt: 2,
  deletedAt: null,
};

describe("SpecificationsPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.read.mockResolvedValue(emptyAuthority);
    api.verifyVault.mockResolvedValue(undefined);
    api.updateFrontmatter.mockResolvedValue(undefined);
    vi.stubGlobal("crypto", {
      randomUUID: () => "12345678-1234-4123-8123-123456789abc",
    });
  });

  it("verifies the bound vault before designating and approving the exact visible note", async () => {
    const approved: SpecificationAuthorityList = {
      ...emptyAuthority,
      current: 1,
      specifications: [
        {
          approval: {
            projectId: "prj_test",
            specificationId: "spec_12345678123441238123123456789abc",
            relativePath: activeNote.path,
            contentHash: `sha256:${"a".repeat(64)}`,
            approvedAtUnixMs: 1_700_000_000_000,
          },
          state: "current",
          currentContentHash: `sha256:${"a".repeat(64)}`,
        },
      ],
    };
    api.approve.mockResolvedValue(approved);

    render(
      <SpecificationsPanel
        projectPath="/projects/ley"
        vaultPath="/vaults/private"
        activeNote={activeNote}
      />,
    );

    fireEvent.click(
      await screen.findByRole("button", { name: "Approve current note" }),
    );

    await waitFor(() => {
      expect(api.verifyVault).toHaveBeenCalledWith(
        "/projects/ley",
        "/vaults/private",
      );
      expect(api.updateFrontmatter).toHaveBeenCalledWith("page_spec", {
        owner: "Suraj",
        "ley-type": "specification",
        "ley-spec-id": "spec_12345678123441238123123456789abc",
      });
      expect(api.approve).toHaveBeenCalledWith(
        "/projects/ley",
        "/vaults/private",
        "spec_12345678123441238123123456789abc",
        "Specs/Offline product.md",
      );
    });
    expect(api.verifyVault.mock.invocationCallOrder[0]).toBeLessThan(
      api.updateFrontmatter.mock.invocationCallOrder[0],
    );
    expect(api.updateFrontmatter.mock.invocationCallOrder[0]).toBeLessThan(
      api.approve.mock.invocationCallOrder[0],
    );
    expect(
      (await screen.findAllByText("current")).length,
    ).toBeGreaterThanOrEqual(1);
  });

  it("does not mutate Markdown when the open vault is not the project binding", async () => {
    api.verifyVault.mockRejectedValue(
      new Error("Open the bound vault before approving a Specification."),
    );

    render(
      <SpecificationsPanel
        projectPath="/projects/ley"
        vaultPath="/vaults/wrong"
        activeNote={activeNote}
      />,
    );
    fireEvent.click(
      await screen.findByRole("button", { name: "Approve current note" }),
    );

    expect(
      await screen.findByText(/Open the bound vault before approving/),
    ).toBeVisible();
    expect(api.updateFrontmatter).not.toHaveBeenCalled();
    expect(api.approve).not.toHaveBeenCalled();
  });

  it("shows a changed approval and revokes authority without editing the note", async () => {
    const changed: SpecificationAuthorityList = {
      ...emptyAuthority,
      changed: 1,
      specifications: [
        {
          approval: {
            projectId: "prj_test",
            specificationId: "spec_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            relativePath: activeNote.path,
            contentHash: `sha256:${"a".repeat(64)}`,
            approvedAtUnixMs: 1_700_000_000_000,
          },
          state: "changed",
          currentContentHash: `sha256:${"b".repeat(64)}`,
        },
      ],
    };
    api.read.mockResolvedValue(changed);
    api.revoke.mockResolvedValue(emptyAuthority);

    render(
      <SpecificationsPanel
        projectPath="/projects/ley"
        vaultPath="/vaults/private"
        activeNote={activeNote}
      />,
    );

    expect(
      (await screen.findAllByText("changed")).length,
    ).toBeGreaterThanOrEqual(1);
    fireEvent.click(screen.getByRole("button", { name: "Revoke authority" }));
    await waitFor(() => {
      expect(api.revoke).toHaveBeenCalledWith(
        "/projects/ley",
        "/vaults/private",
        "spec_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      );
    });
    expect(api.updateFrontmatter).not.toHaveBeenCalled();
    expect(
      await screen.findByText(/No Specification revisions are approved/),
    ).toBeVisible();
  });
});
