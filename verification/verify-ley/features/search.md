# Project search

A user searches retained project evidence, distinguishes an empty result from a failure, and opens a matching record.

## Sub-features

- `search-match` retrieves the seeded evidence.
- `search-empty` returns a completed empty result.
- `search-scope` changes revision compatibility filtering.
- `search-open` opens the selected historical result.

## How to get to it (user POV)

- Desktop project `Recall` opens search. The header control with accessible name `Search memory` is another entry point.
- CLI `ley search verificationbeacon PROJECT --json`.

## Driving it with control-ley and native accessibility

Preconditions: captured README and the checkpoint from the session recipe.

- Run `drive`. Inspect `search-match.json` for nonempty `results` and `search-miss.json` for an empty array after `zzzxmissingfixturezzzx`. Retain query, revision/freshness fields, and warnings with the output.
- For each Desktop entry point, fill `Search this project’s Agent Memory` with `verificationbeacon`, submit `Search`, and wait for the relevant memories result. Open a result and verify its identity and retained content.
- Return to search and submit `zzzxmissingfixturezzzx`. Require `No matching captured memory`, without an error.
- Change `Filter project memory by revision compatibility` to `Current lineage`, then `All captured history`. Capture the actions and result changes. The nongit helper fixture may have unknown compatibility; use a disposable Git repository for lineage-specific assertions.

## Gotchas

- Typing a query alone does not submit the form.
- Results describe captured history and must retain freshness warnings.
- The helper proves match and empty CLI paths; it does not prove result-opening UI or revision filtering.
