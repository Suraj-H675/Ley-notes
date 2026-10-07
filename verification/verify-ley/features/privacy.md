# Capture privacy and egress

A user sets how retained project context may be shared with coding agents and can inspect the stored policy.

## Sub-features

- `egress-set` restricts the project to an explicitly asserted local model.
- `egress-read` shows the persisted restriction.
- `egress-restore` restores the disposable fixture's agent-ok policy.

## How to get to it (user POV)

- Desktop project `Project settings`, then `Capture & privacy`, field `Agent context sharing policy`.
- CLI `ley egress project local-model-only PROJECT --json` and `ley egress list PROJECT --json`.

## Driving it with control-ley and native accessibility

Preconditions: disposable captured project, no host integration installation.

- Run `drive`. Compare `egress-set.json` with the independent `egress-list.json`; `projectPolicy` must be `local-model-only`. Compare `egress-restore.json` with `egress-restored.json`; the final policy must be `agent-ok`.
- For Desktop, open `Project settings` and `Capture & privacy`. Select the visible local-model-only policy through `Agent context sharing policy`, then choose `Apply sharing policy`. Refresh/reopen settings and require the same selection. Read `ley egress list PROJECT --json` using this native run's private state and require agreement. Capture the mutation and readback separately.

## Gotchas

- Local model is an assertion by the user/host, not proof of the downstream provider's execution location.
- A stored policy is not a complete proof of cloud brief rejection; use the brief recipe for that path.
- Never change policy, approve sources, or erase memory in the user's real project. This map does not claim erasure/export or approval coverage.
