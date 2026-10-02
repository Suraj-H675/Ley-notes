# ADR 0091: Retire shape-specific recovery MCP wrappers

Status: accepted

## Context

Ley's historical recovery work built a deterministic verifier/writer family for reconstructing supported
Decision, Task, Plan, Problem, unresolved, batch/composite, and isolated observed-Command checkpoints from exact
post-checkpoint evidence. Those core state machines remain valuable compatibility evidence: old schema-v3/v8–v14/v16
session histories must still replay, and the verifier/writer implementations remain heavily tested in `ley-core`.

The focused product later changed the model-facing recovery contract. Current crash recovery is read-only
`ley_session_memory_compile`, live workspace/runtime re-verification with ordinary host tools, and a normal
`ley_checkpoint` only for facts that are supportable now. The old shape-specific verifier/commit routes were
therefore placed in `RETIRED_MODEL_RECOVERY_TOOLS` during R3.

Repository audit confirmed that every server—including legacy-vault compatibility mode—disabled all of those
routes after router construction. Their remaining MCP request/candidate types, method bodies, route-disable list,
and direct MCP tests were unreachable implementation residue rather than a callable compatibility surface.

## Decision

Delete the unreachable model-facing wrapper layer for:

- observed-Command verify/commit;
- generic transition verify;
- batch verify/commit;
- rich-Problem composite verify/commit;
- typed Task/Plan verify plus Task/Plan commit;
- rich-Problem verify/commit; and
- unresolved / minimal structured Decision-or-Problem commits.

Specifically:

1. Remove the corresponding MCP tool methods and wrapper-only request/candidate types/conversions.
2. Remove `RETIRED_MODEL_RECOVERY_TOOLS` and redundant per-route disable calls for methods that no longer exist.
3. Remove direct MCP tests that invoked already-unreachable recovery routes.
4. Keep `ley_session_memory_compile` unchanged as the read-only interruption-evidence surface.
5. Keep every core verifier/writer/state-machine implementation, candidate fingerprint contract, schema-v3/v8–v14/v16
   historical event meaning, replay/validation rule, and recovery-core deterministic test.

## Compatibility and data safety

This is not a new live MCP behavior break. The deleted routes were already disabled on every server and absent
from advertised tool inventories. No current packaged Skill, host workflow, release matrix, or current docs teach
agents to call them.

No stored recovery event is rewritten or deleted. Historical ledgers continue to replay through the same core
logic, and deterministic core tests remain the authority for those old schemas. There is no migration.

The removed MCP request/candidate structs and methods were public Rust items. Repository audit found no non-test
callers, and Ley does not document `ley-mcp` as a stable embeddable Rust-library API; the current crate is version
0.1.0. An external Rust consumer that directly imported those retired wrapper items would need to update. That is
a source-compatibility break, distinct from the unchanged advertised MCP tool surface.

## Consequences

- The MCP implementation finally matches the already-shipped read-only recovery contract.
- Crash/interruption evidence remains inspectable without restoring model-facing structural reconstruction
  authority.
- Historical verifier/writer code remains isolated in core compatibility/state-machine logic rather than being
  accidentally exposed as an agent API.
- Reintroducing a recovery writer requires new controlled evidence that read-only interruption evidence plus live
  re-verification and ordinary checkpointing is materially insufficient.
