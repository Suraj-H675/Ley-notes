import { beforeEach, describe, expect, it } from "vitest";
import { createPage, listPages, renamePage } from "@/core/vault/pages";
import { db } from "@/infrastructure/database/db";
import { promoteLearningNote } from "./promote-learning-note";
import type { PromotedLearningNoteDraft } from "./types";

const draft: PromotedLearningNoteDraft = {
  learningId: "lrn_test",
  title: "Verified release procedure",
  folder: "Agent Memory/Lessons",
  content: "Run every release check.\n",
  frontmatter: {
    "ley-source": "agent-memory",
    "ley-learning-id": "lrn_test",
    "ley-trust-state": "trusted",
  },
};

describe("promote learning note", () => {
  beforeEach(async () => {
    await db.delete();
    await db.open();
  });

  it("creates one ordinary note and reopens it after a user rename", async () => {
    const first = await promoteLearningNote(draft);
    expect(first.created).toBe(true);
    expect(first.page.path).toBe(
      "Agent Memory/Lessons/Verified release procedure.md",
    );

    const renamed = await renamePage(first.page.id, "My release checklist");
    const replay = await promoteLearningNote(draft);

    expect(replay).toMatchObject({
      created: false,
      page: {
        id: first.page.id,
        title: "My release checklist",
        path: renamed.path,
      },
    });
    expect(await listPages()).toHaveLength(1);
  });

  it("refuses to overwrite an unrelated note with the same title", async () => {
    await createPage({
      title: draft.title,
      content: "Human-authored note.",
    });

    await expect(promoteLearningNote(draft)).rejects.toThrow(
      "Choose a different title",
    );
    expect(await listPages()).toHaveLength(1);
  });
});
