# ADR 0083: Separate reader projection schema identity from durable ledger schema

Status: Accepted

## Context

`ley_session_get`, `ley_session_turns_get`, and `ley_learning_get` are derived read projections over
durable append-only ledgers. Historically their `schemaVersion` field simply mirrored the underlying
session or learning ledger schema. That field is useful provenance—for example, a Procedure-claiming
context-utility observation advances a session to schema v15—but it does **not** describe the JSON
shape of the reader itself.

This became ambiguous as the projections evolved independently. Session Context gained bounded
verification evidence, context-utility coverage, terminal finish identity, and other read-only fields
without changing the meaning of the durable session ledger. Learning Context likewise gained origin
lineage and bounded Procedure application history while the durable learning ledger remained v2.

Consumers therefore need two explicit identities rather than interpreting ledger evolution as reader
compatibility.

## Decision

The three long-lived provenance readers add an independent projection version:

- `ley_session_get` → `projectionSchemaVersion: 1`;
- `ley_session_turns_get` → `projectionSchemaVersion: 1`;
- `ley_learning_get` → `projectionSchemaVersion: 1`.

The existing `schemaVersion` field is preserved unchanged and continues to mean the underlying durable
ledger schema version.

Projection schema versions describe the serialized read contract only. They do not change stored
events, replay fingerprints, authority, trust, source freshness, or historical interpretation.
Future incompatible reader-shape changes should advance the relevant projection version separately
from ledger schema evolution. Additive fields may be handled according to the reader's compatibility
policy without rewriting historical ledgers.

## Consequences

- a schema-v15 session can truthfully report `schemaVersion: 15` and
  `projectionSchemaVersion: 1` at the same time;
- a learning can report durable learning schema v2 while its reader remains projection v1;
- desktop/agent consumers can reason about reader compatibility without guessing from event history;
- existing consumers that already interpret `schemaVersion` as ledger provenance remain compatible;
- no migration, persistence rewrite, or derived cache is introduced.

## Evaluation

MCP regressions require a schema-v15 Procedure-application session to retain
`projectionSchemaVersion: 1` through `ley_session_get`, and the bounded turns reader to expose its own
projection v1. The Procedure application evaluation also requires `ley_learning_get` to report durable
learning schema v2 alongside projection v1. TypeScript contracts mirror these fields explicitly.
