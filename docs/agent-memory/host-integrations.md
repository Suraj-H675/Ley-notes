# Connect Ley to coding agents

Ley uses three layers together:

1. lifecycle hooks load a bounded continuity brief and save a final-response fallback;
2. local stdio MCP provides cited retrieval and typed session writes;
3. a portable agent skill tells the host when and how to preserve meaningful structure.

All three run on the user's machine. The host may send deliberately retrieved context to its model provider. Ley itself makes no network request.

## Before connecting a host

Install the `ley` executable on `PATH`, initialize the project, bind it to the user's chosen filesystem vault, and capture the first snapshot:

```bash
cargo install --path crates/ley-cli
ley init /path/to/project --capture structured
ley bind /path/to/project --vault /path/to/ley-vault
ley ingest /path/to/project
```

The packaged integrations enable session writes and tentative learning proposals in their local MCP process. Host permission controls still apply. Remove `--allow-learning-proposals` from the package's MCP arguments if proposals are not wanted.

## Codex

The repository contains a Codex marketplace at `integrations/codex`:

```bash
codex plugin marketplace add /absolute/path/to/Ley-notes/integrations/codex
codex plugin add ley-memory@ley
```

Restart Codex, open `/hooks`, and review the exact Ley hook commands before trusting them. Codex intentionally does not trust newly installed command hooks automatically.

## Claude Code

For a local development install:

```bash
claude --plugin-dir /absolute/path/to/Ley-notes/integrations/claude-code/ley-memory
```

The package passes `claude plugin validate --strict`. A published marketplace can install the same directory without changing its plugin layout.

## Gemini CLI

Link the local extension and restart Gemini CLI:

```bash
gemini extensions link /absolute/path/to/Ley-notes/integrations/gemini-cli/ley-memory
```

Gemini substitutes the active `${workspacePath}` into the MCP and hook commands.

## What automatic capture does

In Structured mode, the adapter stores:

- a generated host-session name and continuity goal;
- the stable Ley session ID;
- a bounded, credential-redacted copy of the host's final assistant response for each completed turn.

It does not automatically store:

- user prompts;
- transcript files or transcript paths;
- hidden reasoning;
- tool inputs or complete outputs;
- environment variables;
- arbitrary files outside the approved project capture boundary.

The final-response checkpoint is a fallback, not a claim of complete capture. For substantive work the bundled skill asks the agent to write typed decisions, tasks, problems, attempts, outcomes, solutions, verification, touched artifacts, unresolved items, and handoff through MCP.

## Failure and retry behavior

An uninitialized or unbound project returns `{}` and remains untouched. A stable host session maps to the same Ley session after process restart. Exact hook retries replay the same append-only request instead of creating duplicates. A new host session receives the normal bounded resume pack, including earlier checkpoint evidence and only user-trusted, artifact-current learnings.

The bundled MCP process also starts cleanly in an ordinary workspace, but advertises zero capabilities and no tools or resources. It never initializes or scans that directory. This keeps a globally installed integration quiet and harmless until the user explicitly sets up Ley for that project.

If a captured snapshot is missing or inconsistent, the hook fails rather than inventing context. Run `ley doctor`, restore the binding if needed, then `ley ingest` deliberately.
