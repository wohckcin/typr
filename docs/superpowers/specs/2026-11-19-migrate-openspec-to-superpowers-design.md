# Migrate OpenSpec to superpowers — design

## Intent

Retire the OpenSpec CLI workflow from typr. Keep everything the project has written down about what the app promises, keep the one unfinished change usable, and make superpowers the only planning process.

## Goals

- The 18 capability specs survive as plain Markdown under `docs/specs/`.
- `motion-coverage` survives as one superpowers plan under `docs/superpowers/plans/`, with its decisions, 14 tasks and 3 acceptance requirements intact.
- No OpenSpec CLI, config, skills, active change or archive remains in the tree.
- README points readers at the new homes of promises and plans.

## Non-Goals

- Implementing any of the 14 motion-coverage tasks.
- Rewriting spec text or reformatting the plan's original content.
- Copying the 37 archived changes into `docs/`; they remain in git history.

## Constraints

- No `openspec` command is run after the cutover.
- Spec files move byte-for-byte; only their paths change.
- Superpowers conventions apply: specs and plans live under `docs/superpowers/`; the durable promise record is the plain `docs/specs/` tree.

## Approaches

1. **Content-preserving cutover** (chosen). Move the promises, fold the active change into a plan, delete the machinery. One migration, no dual sources of truth, all written intent retained.
2. **Fresh start.** Delete the whole `openspec/` tree and start superpowers from an empty `docs/`. Rejected: it throws away the only written record of what the app promises.
3. **Run both systems.** Rejected: two places claim to describe the same work, and the OpenSpec skills would keep firing on `openspec` requests.

## Decisions

- `openspec/specs/*` → `docs/specs/*` (18 dirs, unchanged content).
- `openspec/changes/motion-coverage/` → `docs/superpowers/plans/2026-11-19-motion-coverage.md`. The proposal supplies the design, `tasks.md` supplies the checkbox sequence, and the delta spec's three requirements become the acceptance section — an unimplemented requirement does not belong in the promise record.
- `openspec/changes/archive/` (37 changes) and the rest of `openspec/` are deleted; git history is the archive.
- Remove both the six `.claude/skills/openspec-*` symlinks and their six `.agents/skills/openspec-*` targets, plus the `.agents/skills/.openspec-target` marker. `nsh`, `nsh-design-guides` and `run-typr` stay.
- `README.md:6-7` names `docs/specs/` as the promise record and `docs/superpowers/plans/` as work in flight.
- New work follows brainstorming → writing-plans → executing-plans.

## Builder notes

No further decisions are left to the executor. The migration is file moves, one document conversion, deletions and a README edit; verification is a count and a stale-reference search.
