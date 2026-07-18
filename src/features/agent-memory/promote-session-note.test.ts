import { beforeEach, describe, expect, it } from "vitest";
import { createPage, renamePage } from "@/core/vault/pages";
import { resetDb } from "@/test/helpers";
import { promoteSessionNote } from "./promote-session-note";
import type { PromotedSessionNoteDraft } from "./types";

const draft: PromotedSessionNoteDraft = {
  sessionId: "ses_test",
  projectId: "prj_test",
  title: "Session handoff",
  folder: "Agent Memory/Sessions",
  content: "## Handoff\n\nContinue here.\n",
  frontmatter: {
    "ley-source": "agent-memory",
    "ley-project-id": "prj_test",
    "ley-session-id": "ses_test",
  },
};

describe("session note promotion", () => {
  beforeEach(() => resetDb());

  it("creates once and finds the linked note after a rename", async () => {
    const first = await promoteSessionNote(draft);
    expect(first.created).toBe(true);
    await renamePage(first.page.id, "Renamed handoff");

    const repeated = await promoteSessionNote({
      ...draft,
      title: "Another title",
    });
    expect(repeated.created).toBe(false);
    expect(repeated.page.id).toBe(first.page.id);
    expect(repeated.page.title).toBe("Renamed handoff");
  });

  it("does not reuse an unrelated title collision", async () => {
    await createPage({ title: draft.title });
    await expect(promoteSessionNote(draft)).rejects.toThrow("already exists");
  });
});
