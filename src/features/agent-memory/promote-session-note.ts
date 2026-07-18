import { createPage, getPageByTitle, listPages } from "@/core/vault/pages";
import type { Page } from "@/infrastructure/database/schema";
import type { PromotedSessionNoteDraft } from "./types";

export async function promoteSessionNote(
  draft: PromotedSessionNoteDraft,
): Promise<{ page: Page; created: boolean }> {
  const existingPromotion = (await listPages()).find(
    (page) =>
      page.frontmatter["ley-source"] === "agent-memory" &&
      page.frontmatter["ley-project-id"] === draft.projectId &&
      page.frontmatter["ley-session-id"] === draft.sessionId,
  );
  if (existingPromotion) {
    return { page: existingPromotion, created: false };
  }

  const titleCollision = await getPageByTitle(draft.title);
  if (titleCollision) {
    throw new Error(
      `A note named “${draft.title}” already exists. Choose a different title.`,
    );
  }

  const page = await createPage({
    title: draft.title,
    folder: draft.folder,
    content: draft.content,
    frontmatter: draft.frontmatter,
  });
  return { page, created: true };
}
