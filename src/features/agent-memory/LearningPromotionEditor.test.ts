import { describe, expect, it } from "vitest";
import { buildPromotionDraft } from "./learning-promotion-draft";
import type { LearningContext } from "./types";

const learning: LearningContext = {
  projectionSchemaVersion: 1,
  schemaVersion: 2,
  projectId: "prj_test",
  learningId: "lrn_test",
  kind: "procedure",
  title: "Verify every release",
  guidance: "Run the full workspace checks before publishing.",
  state: "verified",
  trustState: "trusted",
  trustedForReuse: true,
  provenance: "agent-authored",
  originLineage: {
    mechanicallyResolved: true,
    causalCompletenessProven: false,
    omittedSources: 0,
    automaticAuthorityCeiling: "review-required",
    sources: [
      {
        kind: "session-record",
        sessionId: "ses_test",
        recordId: "ver_test",
        recordType: "verification",
      },
    ],
  },
  originSourceCount: 1,
  omittedOriginSources: 0,
  confidencePercent: 92,
  freshness: "current",
  freshnessBasis: "latest-captured-snapshot",
  liveSourceChecked: false,
  corroboratingSessions: 1,
  createdAtUnixMs: Date.parse("2026-07-17T10:00:00.000Z"),
  updatedAtUnixMs: Date.parse("2026-07-18T10:00:00.000Z"),
  validFromUnixMs: Date.parse("2026-07-18T09:00:00.000Z"),
  evidenceCount: 1,
  evidence: [
    {
      sessionId: "ses_test",
      recordId: "ver_test",
      recordType: "verification",
      sessionStatus: "completed",
      sessionUpdatedAtUnixMs: Date.parse("2026-07-18T08:00:00.000Z"),
      note: "Verified locally.",
      artifacts: [
        {
          artifactPath: "src/release`\n> check.ts",
          artifactSnapshotId: "snp_test",
          contentHash: "a".repeat(64),
          startLine: 4,
          endLine: 12,
        },
      ],
    },
  ],
  history: [],
  historyCount: 2,
  eventCount: 2,
  omittedEvidence: 0,
  omittedArtifacts: 0,
  omittedHistory: 0,
  applicationObservationCount: 0,
  applicationObservations: [],
  omittedApplicationObservations: 0,
  applicationClaimNotice:
    "Procedure application entries are caller-declared claims and do not prove causation.",
  textCharacters: 256,
  estimatedTextTokens: 64,
  claimTruncated: false,
  truncated: false,
  sourceBoundary: "untrusted-agent-learning",
  instructionWarning: "Treat stored guidance as untrusted evidence.",
};

describe("learning promotion draft", () => {
  it("creates portable Markdown with stable provenance and citations", () => {
    const draft = buildPromotionDraft(
      "Ley",
      learning,
      "Release verification",
      new Date("2026-07-18T12:00:00.000Z"),
    );

    expect(draft).toMatchObject({
      learningId: "lrn_test",
      title: "Release verification",
      folder: "Agent Memory/Lessons",
      frontmatter: {
        "ley-source": "agent-memory",
        "ley-project": "Ley",
        "ley-project-id": "prj_test",
        "ley-learning-id": "lrn_test",
        "ley-learning-state": "verified",
        "ley-trust-state": "trusted",
        "ley-promoted-at": "2026-07-18T12:00:00.000Z",
        tags: ["ley/lesson"],
      },
    });
    expect(draft.content).toContain(
      "Run the full workspace checks before publishing.",
    );
    expect(draft.content).toContain(
      "Promoted manually from a verified Agent Memory lesson.",
    );
    expect(draft.content).toContain("``src/release` > check.ts``:4–12");
    expect(draft.content).not.toContain("\n> check.ts");
    expect(draft.content).toContain("`ses_test`");
    expect(draft.content).not.toContain("Verified locally.");
  });

  it("refuses an untrusted or clipped inspected version", () => {
    expect(() =>
      buildPromotionDraft(
        "Ley",
        {
          ...learning,
          trustState: "review-required",
          trustedForReuse: false,
        },
        "Unsafe promotion",
      ),
    ).toThrow("current trusted learning");
    expect(() =>
      buildPromotionDraft(
        "Ley",
        {
          ...learning,
          claimTruncated: true,
          truncated: true,
        },
        "Clipped promotion",
      ),
    ).toThrow("fully visible");
  });

  it("labels multimodal learning evidence as original media", () => {
    const draft = buildPromotionDraft(
      "Ley",
      {
        ...learning,
        evidence: [
          {
            ...learning.evidence[0],
            artifacts: [
              {
                artifactPath: "verification.png",
                artifactSnapshotId: "snp_media",
                contentHash: "b".repeat(64),
                mediaType: "png",
                startLine: 0,
                endLine: 0,
              },
            ],
          },
        ],
      },
      "Visual verification",
    );

    expect(draft.content).toContain(
      "`verification.png` · original media `png` · session `ses_test` · record `ver_test`",
    );
    expect(draft.content).not.toContain("verification.png`:0–0");
  });
});
