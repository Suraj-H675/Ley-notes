# ADR 0001: Local agent project boundary

- Status: Accepted
- Date: 2026-07-16

## Context

Ley must remember work across agent sessions without silently scanning unrelated directories, putting transcripts into code repositories, or making one agent host the storage authority. The user approved central durable memory in a filesystem vault plus a minimal `.ley/` identity and capture-policy folder inside each initialized project.

The initial executable boundary must work without a running GUI, account, network service, or model API. It must later be reusable by the Tauri desktop shell and a local stdio MCP server.

## Decision

Create a Rust workspace with:

- `ley-core`: filesystem-safe project identity, policy, storage, and later agent-memory domain behavior;
- `ley-cli`: local commands such as `ley init` and `ley doctor`;
- the existing Tauri application as another workspace member.

`ley init` creates exactly:

```text
.ley/
├── project.json
├── capture.json
└── .leyignore
```

Initialization stages the complete directory and atomically renames it into place. It never overwrites an existing identity or silently changes capture consent. A repeated initialization is an idempotent read.

`project.json` contains only schema version, random stable project ID, display name, and creation time. It contains no absolute path, vault binding, agent credential, transcript, memory, or provider identity.

`capture.json` defaults to Structured capture, the approved project root (`.`), Git-ignore awareness, bounded input sizes, and raw transcripts disabled. Full Evidence must be selected explicitly before `storeRawTranscripts` can become true.

`.leyignore` uses Git-style patterns and layers additional capture exclusions after project `.gitignore` rules. Defaults exclude generated directories and credential containers, but do not use broad terms such as `*secret*` that could hide legitimate source files. Content redaction remains a separate ingestion defense.

JSON files follow JSON Schema Draft 2020-12, reject unknown fields, and carry an explicit integer schema version. Runtime validation additionally enforces safe relative paths, internally consistent byte limits, regular non-symlink metadata files, and a 1 MiB metadata-file cap at the trust boundary.

## Evidence

- Cargo workspaces provide one shared lockfile/output directory and workspace-wide commands: <https://doc.rust-lang.org/stable/cargo/reference/workspaces.html>.
- Git documents hierarchical, last-match-wins ignore behavior and the intended distinction between shared and local exclusions: <https://git-scm.com/docs/gitignore>.
- JSON Schema Draft 2020-12 is the latest published meta-schema and supports the conditional used to bind raw transcripts to Full Evidence: <https://json-schema.org/draft/2020-12>.
- MCP security guidance requires explicit consent and warns about filesystem access outside expected directories for local servers: <https://modelcontextprotocol.io/docs/tutorials/security/security_best_practices>.
- The checked-in Graphify reference independently uses a 1 MiB text/config input cap and layers its own ignore rules after `.gitignore`; it is prior art, not an authority.

## Consequences

- Desktop, CLI, hooks, and MCP can share one local identity implementation.
- A project can be moved without invalidating its identity because no absolute path is stored in the repository.
- Durable session memory is not implemented by this ADR and must live in the selected filesystem vault, not this project folder.
- A future scanner must preview its resolved include/exclude set before the first capture and must enforce canonical containment even when ignore patterns are malformed.
- Ley refuses `.ley` directory and metadata-file symlinks instead of following repository-controlled paths outside the project boundary.
