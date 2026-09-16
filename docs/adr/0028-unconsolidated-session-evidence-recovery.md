# ADR 0028: Unconsolidated session evidence recovery

Status: accepted

## Context

Ley already retains immutable, bounded prompt/response observations and supports rich typed checkpoints. A host crash, interruption, or missed checkpoint can therefore leave trustworthy source evidence in the session ledger without the structured decisions/tasks/problems/handoff that later agents need.

Automatically summarizing every turn would create noisy micro-memories and could launder untrusted prompt/agent text into durable claims. The first Memory Compiler slice therefore needs to improve capture reliability without inventing semantics or new authority.

## Decision

Ley adds `compile_session_memory` and read-only MCP tool `ley_session_memory_compile` as the first Memory Compiler slice.

The compiler deterministically defines its recovery window as prompt/response events whose immutable event sequence is strictly greater than the latest structured checkpoint event sequence. It does not claim that an older checkpoint semantically captured every preceding fact; it only identifies evidence that definitely occurred later.
Checkpoint event sequence is replay metadata derived from the authoritative event envelope. It is not added to existing checkpoint event payloads or request fingerprints, so durable event compatibility and exact retry semantics remain unchanged.

The recovery pack reports one of four states:

- `no-unconsolidated-evidence` — no turn evidence exists after the latest checkpoint;
- `reviewable-evidence` — retained post-checkpoint turn bodies are paired and complete enough for deliberate review;
- `partial-evidence` — at least one body is truncated/omitted or a prompt/response is unpaired or uncorrelated;
- `metadata-only` — post-checkpoint observations exist but capture policy/capacity retained no bodies.

Every returned body remains explicitly `untrusted-user-prompt` or `untrusted-agent-output`. The compiler does not create a checkpoint, learning, resolution, verified outcome, or trusted knowledge. A prompt-only crash may support an unresolved request; it does not prove that any work was completed.

The tool is bounded to 1–100 recent recovery records and 1,000–64,000 retained text characters, with defaults of 20 and 16,000. It reports omitted records and capture/compiler truncation. Existing MCP serialized-output limits remain a final bound.
A recovery writer should use the pack's `sessionEventCount` as `expectedEventCount` on `ley_session_checkpoint`. Ley revalidates that count under the existing session writer lock before appending. If newer evidence arrived after compilation, the write fails closed and the caller must recompile. Exact retries of a checkpoint request remain idempotent.

Lifecycle startup does not inject the recovery bodies. When the same host session resumes with post-checkpoint evidence, the adapter emits only a count/state signal and directs the agent to the explicit compiler tool. Normal startup context therefore remains free of automatic turn bodies.

## Consequences

- Crash/missed-checkpoint recovery becomes discoverable without making transcript-like evidence normal startup context.
- The first Memory Compiler slice is deterministic and reversible: its pack is a disposable projection over immutable session events.
- Recovery still requires the agent to interpret evidence and deliberately write typed structure; no model-produced structure is silently trusted.
- Minimal capture remains private by construction: recovery can report that observations exist without reconstructing omitted text.
- Concurrent host activity cannot be hidden by a stale recovery write when `expectedEventCount` is used.
- This slice does not yet infer missing decisions/problems/resolutions automatically, verify transition coverage/preservation/faithfulness, consolidate across sessions, or propose reusable learnings. Those remain later Memory Compiler work and require evaluation before automation.
