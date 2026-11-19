# Migrate OpenSpec to superpowers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Retire OpenSpec from typr while preserving the promise specs, the one unfinished change and the README's description of where work lives.

**Architecture:** Move `openspec/specs/` verbatim to `docs/specs/`, fold the `motion-coverage` change into a single plan, delete the CLI tree and its skills, then point the README at the new locations. Files are moved with `git mv` so history follows them.

**Tech Stack:** Git, Markdown, the superpowers brainstorming/writing-plans/executing-plans process.

**Spec:** `docs/specs/2026-11-19-migrate-openspec-to-superpowers-design.md`

## Global Constraints

- No `openspec` command after the cutover.
- Specs move byte-for-byte; only paths change.
- The 37 archived changes are not copied anywhere; git history is the archive.
- `nsh`, `nsh-design-guides` and `run-typr` skills stay.

## Review Focus

- A path renamed by hand loses git history — both moves use `git mv`.
- The `motion-coverage` conversion drops the reduced-motion rules — task 2's Decisions and Acceptance sections carry the design decisions and all three requirements.
- A dangling skill symlink survives — task 3 deletes both symlinks and targets.
- README still advertises OpenSpec — task 4 rewrites the sentence and the stale-reference check pins it.

---

### Task 1: Move the promise specs

**Files:**
- Move: `openspec/specs/*` → `docs/specs/*`

**Interfaces:**
- Produces: 18 spec directories under `docs/specs/`, byte-identical to their sources.

- [ ] **Step 1: Move the whole specs tree in one rename**

```bash
mkdir -p docs && git mv openspec/specs docs/specs
```

- [ ] **Step 2: Verify the count and the absence of the old path**

```bash
find docs/specs -mindepth 1 -maxdepth 1 -type d | wc -l   # 18
git ls-files 'docs/specs/*/spec.md' | wc -l                # 18
test ! -e openspec/specs
```

Expected: `18`, `18`, no output from the test.

- [ ] **Step 3: Commit**

```bash
git add -A docs openspec && git commit -m "refactor: move capability specs under docs/specs"
```

### Task 2: Fold motion-coverage into a superpowers plan

**Files:**
- Create: `docs/plans/2026-11-19-motion-coverage.md`
- Read only: `openspec/changes/motion-coverage/proposal.md`, `design.md`, `tasks.md`, `specs/motion/spec.md`

**Interfaces:**
- Consumes: the four `motion-coverage` artifacts.
- Produces: one plan with all 14 tasks and the three acceptance requirements.

- [ ] **Step 1: Write the plan from the existing artifacts**

Use the writing-plans header. In "Decisions" copy the five decisions from `design.md:41-58` and the reduced-motion rule from `design.md:60-64`. In "Tasks" copy the 14 checkbox lines verbatim from `tasks.md`, grouped under their six headings. In "Acceptance" copy the three requirement statements and all six scenarios from `specs/motion/spec.md`; do not merge them into the promise specs.

- [ ] **Step 2: Verify content survived the conversion**

```bash
grep -c '^- \[ \]' docs/plans/2026-11-19-motion-coverage.md       # 14
grep -c '^#### Scenario:' docs/plans/2026-11-19-motion-coverage.md   # 6
grep -c '^### Requirement:' docs/plans/2026-11-19-motion-coverage.md  # 3
```

Expected: `14`, `6`, `3`.

- [ ] **Step 3: Commit**

```bash
git add docs/plans/2026-11-19-motion-coverage.md && git commit -m "docs: convert motion-coverage into a superpowers plan"
```

### Task 3: Delete the OpenSpec machinery

**Files:**
- Delete: `openspec/` (config, active change, 37 archives)
- Delete: `.claude/skills/openspec-*` (6 symlinks)
- Delete: `.agents/skills/openspec-*` (6 target dirs)
- Delete: `.agents/skills/.openspec-target`

**Interfaces:**
- Consumes: nothing.
- Produces: no OpenSpec file or skill in the working tree.

- [ ] **Step 1: Remove the OpenSpec tree and skills**

```bash
git rm -r openspec .agents/skills/.openspec-target
for skill in apply-change archive-change explore propose sync-specs update-change; do
  git rm ".claude/skills/openspec-$skill"
  git rm -r ".agents/skills/openspec-$skill"
done
```

- [ ] **Step 2: Confirm no symlink dangles and no OpenSpec skill remains**

```bash
for skill in apply-change archive-change explore propose sync-specs update-change; do
  test ! -e ".claude/skills/openspec-$skill"
  test ! -e ".agents/skills/openspec-$skill"
done
test ! -e openspec
test ! -e .agents/skills/.openspec-target
```

Expected: no output from the commands.

- [ ] **Step 3: Commit**

```bash
git commit -m "chore: remove OpenSpec CLI tree and skills"
```

### Task 4: Repoint the README and verify the cutover

**Files:**
- Modify: `README.md:6-7`

**Interfaces:**
- Consumes: `docs/specs/`, `docs/plans/`.
- Produces: a README that names the new homes.

- [ ] **Step 1: Rewrite the status sentence**

Replace the two lines with:

```markdown
Status: the port from Rune to Native Shell is merged; what the app promises lives in
`docs/specs/`, and plans in flight in `docs/plans/`.
```

- [ ] **Step 2: Verify no stale live reference remains**

```bash
grep -rIn --exclude-dir=.git --exclude-dir=target 'openspec\|opsx' README.md nsh.json mise.toml Cargo.toml .gitignore scripts .github 2>/dev/null || true
```

Expected: no output. The migration docs under `docs/` intentionally name OpenSpec, so the search is scoped to live config/metadata paths.

- [ ] **Step 3: Verify the surviving skill set**

```bash
ls .claude/skills
```

Expected: `nsh`, `nsh-design-guides`, `run-typr` only.

- [ ] **Step 4: Commit**

```bash
git add README.md && git commit -m "docs: point README at docs/specs and superpowers plans"
```
