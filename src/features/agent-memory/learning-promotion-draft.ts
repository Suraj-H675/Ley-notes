import type { LearningContext, PromotedLearningNoteDraft } from "./types";

export const PROMOTION_FOLDER = "Agent Memory/Lessons";

export function buildPromotionDraft(
  projectName: string,
  learning: LearningContext,
  title: string,
  promotedAt = new Date(),
): PromotedLearningNoteDraft {
  if (!learning.trustedForReuse || learning.claimTruncated) {
    throw new Error(
      "Only a fully visible current trusted learning can be promoted.",
    );
  }
  const citations = learning.evidence.flatMap((evidence) =>
    evidence.artifacts.map(
      (artifact) =>
        `${inlineCode(artifact.artifactPath)}:${artifact.startLine}–${artifact.endLine} · session ${inlineCode(evidence.sessionId)} · record ${inlineCode(evidence.recordId)}`,
    ),
  );
  const sourceTrail =
    citations.length > 0
      ? `\n\n## Source trail\n\n${citations.map((citation) => `- ${citation}`).join("\n")}`
      : `\n\n## Source trail\n\n- Session evidence is recorded under learning ${inlineCode(learning.learningId)}.`;
  const promotedAtIso = promotedAt.toISOString();
  const validFromIso = new Date(learning.validFromUnixMs).toISOString();
  const provenance = [
    "---",
    "",
    "> [!info] Ley provenance",
    "> Promoted manually from a verified Agent Memory lesson. Check live project files when current source accuracy matters.",
    `> Learning: ${inlineCode(learning.learningId)}`,
    `> Confidence at promotion: ${learning.confidencePercent}%`,
    `> Valid version since: ${validFromIso}`,
  ].join("\n");

  return {
    learningId: learning.learningId,
    title,
    folder: PROMOTION_FOLDER,
    content: `${learning.guidance}\n\n${provenance}${sourceTrail}\n`,
    frontmatter: {
      "ley-source": "agent-memory",
      "ley-project": projectName,
      "ley-project-id": learning.projectId,
      "ley-learning-id": learning.learningId,
      "ley-learning-kind": learning.kind,
      "ley-learning-state": learning.state,
      "ley-trust-state": learning.trustState,
      "ley-freshness": learning.freshness,
      "ley-confidence": learning.confidencePercent,
      "ley-valid-from": validFromIso,
      "ley-promoted-at": promotedAtIso,
      tags: ["ley/lesson"],
    },
  };
}

function inlineCode(value: string): string {
  const safeValue = Array.from(value, (character) => {
    const code = character.codePointAt(0) ?? 0;
    return code <= 31 || code === 127 ? " " : character;
  }).join("");
  const longestFence = Math.max(
    0,
    ...Array.from(safeValue.matchAll(/`+/g), (match) => match[0].length),
  );
  const fence = "`".repeat(longestFence + 1);
  return `${fence}${safeValue}${fence}`;
}
