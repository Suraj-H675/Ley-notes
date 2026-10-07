# Deterministic Project Brain import

M2 creates an inspectable Brain from selected local repository material without model access. This is a CLI/core
contract. It does not verify the later redesigned Desktop, MCP retrieval, or coding-host capture workflow.

## Entry points

- `ley brain create` creates a path-independent Brain.
- `ley brain attach` explicitly authorizes a selected working-copy locator.
- `ley brain import` observes that selected root and atomically publishes filtered Sources/SourceVersions and inventory.
- `ley brain contents` reads successful observation and attempt state separately.
- `ley brain versions` and `version` inspect retained history by identity.
- `ley brain move-locator` explicitly updates a moved folder association.

## Drive and evidence

Use a new evidence directory and the parent skill's disposable Linux environment:

```sh
verification/verify-ley/scripts/control-ley launch "$proof_dir"
verification/verify-ley/scripts/control-ley drive "$proof_dir"
verification/verify-ley/scripts/verify-project-import "$proof_dir"
verification/verify-ley/scripts/control-ley cleanup "$proof_dir"
```

Run cleanup after a failed proof too. The M2 script uses the copied binary and existing scratch ownership checks,
adds a separate selected project, and records every CLI command alongside the transition-runtime proof.

The assertions cover import without Git, filter disclosures, valid Unicode and newline paths, redaction, exact
retained evidence, identical-request replay, fresh unchanged observations, changed versions, deletion, conservative
rename continuity, scan failure without a new current inventory, retry, and explicit folder movement. Codex and
Claude launch traps fail the proof if import invokes a provider executable. The fixture originals remain outside
Brain erasure and no repo marker is written by M2.

Inspect `m2-contents-first.json`, `m2-contents-changed.json`, `m2-contents-failed.json`, and
`m2-historical-read.json` together. A failing latest attempt must preserve the preceding successful observation and
inventory. Historical version bytes must remain equal after a live file changes. `run.json` records the passed
M2 cases and the same binary hash as the transition proof.

## Other required coverage

Core/CLI tests cover generation and revocation fences, Source/Brain erasure, Git provenance, duplicate-content
rename ambiguity, boundary attacks, bounds, and portability refusal. Report those commands independently of the
behavioral script. Successful CLI proof does not imply Desktop, MCP, host integration, or portable Brain backup
verification.
