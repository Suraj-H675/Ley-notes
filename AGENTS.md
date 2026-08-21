### Subagent delegation

The parent Codex session retains responsibility for:

* understanding user intent;
* architecture and planning;
* scope decisions;
* delegation decisions;
* synthesis;
* verification;
* Git/commit decisions;
* final acceptance.

Use custom agents selectively when delegation materially improves speed, context isolation, quality, or usage efficiency.

#### Luna

Use `luna_worker` for bounded and well-specified work such as:

* routine implementation;
* repetitive or mechanical edits;
* focused repository exploration;
* test additions;
* documentation work;
* evidence gathering;
* straightforward bug fixes;
* high-volume transformations;
* isolated implementation slices.

#### Terra

Use `terra_worker` for implementation requiring more judgment, including:

* complex or context-heavy implementation;
* difficult debugging;
* integration-heavy changes;
* security-sensitive implementation;
* nontrivial algorithms;
* concurrency-sensitive behavior;
* broad but bounded refactors;
* wider-blast-radius production changes.

#### Parent-only work

Keep work in the parent session when it primarily involves:

* architecture;
* ambiguous requirements;
* scope negotiation;
* consequential design decisions;
* cross-cutting synthesis;
* final review or acceptance;
* trivial changes where delegation overhead exceeds the benefit.

#### Delegation rules

* Use one worker by default.
* Use multiple workers only for genuinely independent work.
* Never assign overlapping write ownership to concurrent workers.
* Every worker assignment must state:

  * objective;
  * exact file or subsystem ownership;
  * interfaces and invariants;
  * constraints and prohibited actions;
  * verification requirements;
  * expected report format.
* Treat worker reports as claims, not proof.
* The parent must inspect the actual files and diff after delegated implementation.
* The parent must rerun proportionate verification before accepting the result.
* Never delegate solely to demonstrate subagent usage.
* Never silently substitute another role or model if the requested custom agent is unavailable.
* Workers must not commit, push, publish, contact external systems, or perform destructive operations unless the user explicitly authorizes those actions.
* For `/review`, Luna and Terra may be used as optional read-only coverage helpers, but their availability must never be an acceptance prerequisite. The parent reviewer owns the final verdict.
