# ADR 0010: Explicit consent for MCP learning proposals

- Status: Accepted
- Date: 2026-07-18

## Context

Read-only agents need access to reviewed project lessons, but automatically granting every existing MCP connection permission to create memory would expand its authority and increase poisoning risk. An agent also must not be able to label itself as a user, confirm its own proposal, rewrite a trusted lesson, or permanently dismiss competing knowledge.

The ADR 0009 ledger already validates session evidence, redacts credentials, serializes concurrent writers, and forces agent claims through review. MCP should expose those guarantees with a capability narrower than general learning administration.

## Decision

The default fixed-project MCP process adds two read-only tools:

| Tool | Result |
| --- | --- |
| `ley_learnings_list` | A maximum of 50 summaries; defaults to explicitly trusted lessons whose artifact citations still match the latest captured snapshot |
| `ley_learning_get` | One bounded trust/provenance/freshness pack with capped evidence, artifacts, history, and text |

The list supports explicit `current-trusted`, `needs-review`, and `all` scopes. Default retrieval excludes tentative, contested, rejected, superseded, stale, source-changed, and uncited lessons. Every response states that the live source was not checked, labels learning text as untrusted agent memory, and includes an instruction warning. All MCP results retain the 256 KB serialized limit.

Learning writes are absent by default. A user may add a separate process-start flag:

```bash
ley mcp /path/to/project --allow-learning-proposals
```

The flag adds only `ley_learning_propose`. The tool:

- hardcodes `actor: agent`;
- accepts only `agent-authored` or `inferred` provenance;
- requires one to twenty existing session-record references;
- creates only a tentative, review-required proposal;
- requires a caller-stable request ID and returns a compact idempotent receipt;
- declares `readOnlyHint: false`, `destructiveHint: false`, `idempotentHint: true`, and `openWorldHint: false`.

MCP exposes no learning confirm, correct, reject, supersede, delete, or promote tool. Those user-authority workflows remain local CLI/UI operations. `--allow-session-writes` and `--allow-learning-proposals` are independent, so a host receives only the capability the user selected.

## Consequences

- Existing configurations gain useful read-only lesson retrieval without gaining write authority.
- Normal agent startup can retrieve only trusted, artifact-current lessons by default.
- Agents can suggest durable memory when explicitly authorized but cannot approve their own claims.
- Review and correction remain a human-controlled boundary.
- Stored prompt injection cannot enable a route omitted when the process was constructed.
- Uncited user-confirmed lessons remain inspectable through explicit scopes but are not injected by the trusted-first default.
