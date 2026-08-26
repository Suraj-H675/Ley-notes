# Connect Ley to coding agents

Ley uses three layers together:

1. lifecycle hooks load a bounded continuity brief and capture bounded turn evidence;
2. local stdio MCP provides cited retrieval and typed session writes;
3. a portable agent skill tells the host when and how to preserve meaningful structure.

All three run on the user's machine. The host may send deliberately retrieved context to its model provider. Ley itself makes no network request.

## Before connecting a host

Install the `ley` executable on `PATH`, initialize the project, bind it to the user's chosen filesystem vault, and capture the first snapshot. From a Ley source checkout:

```bash
cargo install --path crates/ley-cli --root "$HOME/.local"
ley init /path/to/project --capture structured
ley bind /path/to/project --vault /path/to/ley-vault
ley ingest /path/to/project
```

Users with access to the repository can install the same CLI directly from
GitHub without machine-specific paths:

```bash
cargo install --git https://github.com/Suraj-H675/Ley-notes.git \
  --locked ley-cli --root "$HOME/.local"
ley --version
```

The repository is currently private, so GitHub-based installation requires an
authenticated collaborator until Suraj deliberately publishes the repository
or separate release artifacts. A local source checkout does not have that
requirement.

Every integration intentionally launches `ley` from `PATH`. On Linux and
macOS, ensure `$HOME/.local/bin` is on the PATH inherited by the agent host; the
command above uses that conventional location. On Windows, install to a
directory already on PATH. Do not continue until `ley --version` succeeds in a
fresh terminal. The packages contain no developer home directory, vault path,
project path, token, or other machine-specific configuration.

The packaged integrations enable session writes and tentative learning proposals in their local MCP process. Host permission controls still apply. Remove `--allow-learning-proposals` from the package's MCP arguments if proposals are not wanted.

## Codex

Install directly from GitHub with a sparse checkout of only the marketplace and plugin bundle:

```bash
codex plugin marketplace add Suraj-H675/Ley-notes --ref main \
  --sparse .agents/plugins \
  --sparse integrations/codex/plugins/ley-memory
codex plugin add ley-memory@ley
```

For local plugin development, the repository also contains a standalone marketplace at `integrations/codex`:

```bash
codex plugin marketplace add /absolute/path/to/Ley-notes/integrations/codex
codex plugin add ley-memory@ley
```

Restart Codex, open `/hooks`, and review the exact Ley hook commands before trusting them. Codex intentionally does not trust newly installed command hooks automatically.

Start a new Codex chat in an initialized project and invoke **@Ley**, or ask Codex to use Ley. `SessionStart` loads the bounded resume pack automatically. In Structured mode, `UserPromptSubmit` stores a bounded, pattern-redacted prompt record and supplies the exact stable Ley session ID; `Stop` stores the paired bounded final response. The local MCP server provides the structured retrieval and checkpoint tools.

## Claude Code

Install the repository marketplace and plugin from an authenticated checkout:

```bash
claude plugin marketplace add https://github.com/Suraj-H675/Ley-notes.git \
  --sparse .claude-plugin integrations/claude-code/ley-memory
claude plugin install ley-memory@ley
```

For local development, load the same self-contained package directly:

```bash
claude --plugin-dir /absolute/path/to/Ley-notes/integrations/claude-code/ley-memory
```

Restart Claude Code. `SessionStart` loads the bounded brief,
`UserPromptSubmit` reasserts the current Ley session and captures the bounded
prompt according to project policy; `Stop` captures the paired bounded response. The package uses
Claude Code's documented `${CLAUDE_PROJECT_DIR}` placeholder rather than
assuming its process working directory.


## What automatic capture does

In Structured and Full Evidence modes, the adapter stores:

- a generated host-session name and continuity goal;
- the stable Ley session ID;
- a bounded, pattern-redacted copy of each observed user prompt;
- a bounded, pattern-redacted copy of the host's final assistant response for each completed turn;
- an opaque Ley-derived turn reference that pairs the two without retaining the host's raw turn identifier.

Minimal mode stores body-free prompt/response observation events so users can see what was omitted without retaining either body.

Each adapter also injects the stable current Ley session ID at session and turn
start so MCP writes continue the same session instead of creating duplicates.
The native turn event is Codex and Claude `UserPromptSubmit`.

It does not automatically store:

- transcript files or transcript paths;
- hidden reasoning;
- tool inputs or complete outputs;
- environment variables;
- arbitrary files outside the approved project capture boundary.

Turn evidence is not a checkpoint and is never promoted into startup context. For substantive work the bundled skill asks the agent to write typed decisions, tasks, problems, attempts, outcomes, solutions, verification, touched artifacts, unresolved items, and handoff through MCP.

Ley deliberately does not add a global tool logger. Tool calls can contain
credentials, large outputs, or irrelevant details, and a hook cannot reliably
infer their durable meaning. Each bundled skill instead records bounded
commands, touched artifacts, and observed outcomes inside meaningful structured
checkpoints.

## Failure and retry behavior

An uninitialized or unbound project returns `{}` and remains untouched. A stable host session maps to the same Ley session after process restart. Codex pairs retries with its documented stable `turn_id`; Claude Code uses the append-only Ley session state because their pre/post events do not share a stable turn identifier. Exact hook retries replay instead of duplicating, while the same prompt submitted after a completed response becomes a new turn. A new host session receives the normal bounded resume pack, including earlier checkpoint evidence and only user-trusted, artifact-current learnings—not captured prompt/response bodies.

The bundled MCP process also starts cleanly in an ordinary workspace, but advertises zero capabilities and no tools or resources. It never initializes or scans that directory. This keeps a globally installed integration quiet and harmless until the user explicitly sets up Ley for that project.

If a captured snapshot is missing or inconsistent, the hook fails rather than inventing context. Run `ley doctor`, restore the binding if needed, then `ley ingest` deliberately.
