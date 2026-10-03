# ADR 0091: Retire shape-specific recovery writers and verifiers

Status: accepted

## Context

Ley historically built deterministic verifiers and bound writers for reconstructing Decision, Task, Plan,
Problem, unresolved, batch/composite, and isolated observed-Command checkpoints from post-checkpoint evidence.
The focused product later changed the current interruption workflow to read-only `ley_session_memory_compile`,
live workspace/runtime re-verification, and an ordinary `ley_checkpoint` only for facts that remain supportable.

The former MCP wrappers were already disabled on every server. A further core audit found no CLI, MCP, host, or
Desktop production caller for the `verify_*_memory_transition*` / `commit_*_memory_transition*` APIs or the
recovered checkpoint producers. The writer and verifier implementation therefore no longer serves a current
product path. Persisted recovery events remain part of supported historical session data: replay, history
validation, per-record evidence bindings, learning provenance, continuity import, and erasure depend on them.

## Decision

1. Remove the public core shape-specific recovery verifier and commit functions, their public policy/result/input
   types, limits, and the private recovery event producers.
2. Do not create new `RecoveryCheckpointRecorded` events. Keep ordinary checkpoint/session writes unchanged.
3. Remove state-machine, eligibility, overlap, coverage, and write-time re-verification logic used only to decide
   or append new recovery events.
4. Retain exact persisted-event compatibility for schemas v3, v8-v13, and v16, including checkpoint projections,
   event/history validation, provenance/evidence bindings, and versioned candidate/binding fingerprint checks.
   Schema-v14 tool-observation events remain ordinary session evidence and are validated as before.
5. Keep the Memory Compiler command-candidate projection read-only. Its guidance directs callers to inspect live
   state and record an ordinary checkpoint only when the evidence supports it.
6. Test historical compatibility with persisted-event fixtures and stable fingerprint contracts, not retired
   recovery writers.

## Compatibility and data safety

No stored event is rewritten or deleted. Existing v3, v8-v13, and v16 recovery checkpoints continue to replay and
validate, and their learning provenance remains resolvable. Continuity import and session erasure continue to
operate on the same persisted history. There is no migration.

The retired Rust APIs were public exports from `ley-core`; removing them is a source-compatibility break for
external Rust consumers that imported them. Ley does not document `ley-core` as a stable embeddable API, and the
crate is version 0.1.0. Ordinary checkpoint APIs remain available.

## Consequences

- Current interruption recovery has one path: inspect bounded read-only Memory Compiler evidence, re-check live
  state, then use an ordinary checkpoint for supportable facts.
- Core retains only the private compatibility code needed to validate historical recovery bytes and provenance.
- Reintroducing a recovery verifier or writer requires new controlled evidence and a separate decision; replay
  compatibility alone does not justify restoring those APIs.
