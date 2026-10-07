# Ley domain language

This glossary names the Project Brain concepts used by the current product contract. It describes the domain,
not a particular database schema or UI.

## Project Brain

One durable logical software project in Ley. A Brain has a stable identity independent of any folder, clone,
worktree, branch, or machine. It may exist before source code exists.

## Repository attachment

The Project Brain's explicitly associated code repository/workspace. The initial product allows zero or one
logical repository attachment per Brain. The attachment does not define the Brain's identity.

## Working copy

One explicitly authorized machine-local checkout, clone, or worktree of the attached repository. Multiple
working copies may belong to the same repository attachment. A path or copied Ley marker is not authorization.

## Locator

A machine-local or external place where Ley observed a Source or working copy. A Locator can change or be revoked;
it is never the identity of the thing it locates and never grants authority by itself.

## Source

A stable project-owned identity for an evolving input such as a specification, repository file, imported file or
folder, or pasted reference. Two Sources remain distinct even when their content is identical.

## SourceVersion

One immutable retained representation of a Source at a point in time. Its identity is distinct from both the
Source and the representation's content hash. A SourceVersion records what Ley actually retained, including when
that differs from the original because of redaction, extraction, or another transformation.

## Capture occurrence

The historical observation that a particular SourceVersion entered or was observed by the Brain at a particular
time and scope. Re-observing identical retained content can create another occurrence without creating another
SourceVersion.

## Session

One identifiable coding-agent work session associated with a Project Brain. Session identity and supported host
metadata do not imply Ley observed every action performed during the session.

## Episode

One supported observable occurrence in project history, such as a visible prompt, response, tool observation, or
other captured event. A model-inferred grouping such as “the authentication debugging episode” is interpretation,
not an observed Episode.

## Evidence reference

An exact link from a claim or human action to retained SourceVersion or Episode evidence. Evidence integrity proves
what Ley retained or observed; it does not by itself prove that an interpretation is true.

## Human action

An explicit user-originated decision such as accepting, rejecting, correcting, or superseding a reviewed target,
or granting/revoking a permission. Ley preserves the exact target the user acted on. Agent-generated text cannot
self-declare a Human action.

## Assertion

A claim or relationship derived about the project. Unreviewed Assertions are interpretation and may be rebuilt.
If a Human action targets an Assertion, the exact reviewed version becomes durable historical evidence of what the
user acted on.

## Evidence basis

How a claim is supported: directly observed, reported, or inferred. Producer identity is tracked separately.

## Adoption

The project's human decision state for a reviewed claim, such as proposed, accepted, rejected, or superseded.
Adoption is separate from factual truth and from permission to access tools, files, networks, or model providers.

## Applicability

Whether knowledge applies to a particular project/source/revision/branch/worktree context now. Scope and current
assessment are separate; Git ancestry is evidence about revision relationships, not semantic authority.

## Project generation

The Brain's durable write epoch. Destructive or permission-revoking changes can advance it so work started against
an older generation cannot silently commit as if nothing changed.

## Removed

No longer active/current input, while retained history remains available according to policy.

## Erased

Ley-managed retained content for the erased scope has been deleted according to the product's deletion contract.
Minimal tombstone identity may remain solely to prevent stale work from recreating erased state.

## Canonical history

Durable observations, retained evidence, explicit human actions, and lifecycle facts whose loss would change what
actually happened or what the user explicitly decided.

## Derived state

Rebuildable interpretation or acceleration structures such as candidate Assertions, summaries, rankings,
embeddings, search indexes, or graph layouts. Derived state never becomes a second authority over canonical history.
