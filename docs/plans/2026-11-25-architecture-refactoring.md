# Architecture Refactoring Plan

**Goal:** Align the `typr` workspace (`crates/typr-core` and `crates/typr`) with `docs/specs/project-structure/spec.md`, remove the structural cycle and god-module bottlenecks identified in the code architecture review, and settle the split between the synchronous `Db` path and the unwired `store::task` worker.

**Architecture:** Preserve the clean toolkit-free backend / NSH desktop split while:
1. Closing low-risk spec and CI guardrail gaps first.
2. Resolving whether `crates/typr` uses `store::task::Handle` or synchronous `Db` exclusively.
3. Extracting a zero-outbound-dependency domain `model` layer out of `store/mod.rs` and splitting `store/db.rs` by domain (`cards`, `decks`, `review`, `views`, `snapshot`).
4. Moving leaked domain logic out of `crates/typr/src/screens/` into `typr-core` and decomposing `shell_over` into focused controllers.
5. Bringing public structs (`#[non_exhaustive]`) and `ui/` event payloads into compliance with `project-structure/spec.md`.

**Tech Stack:** Rust (workspace `typr-core` + `typr`), SQLite via `rusqlite`, Native Shell (`nsh-*`), `scripts/check-tokens.sh`, GitHub Actions CI.

**Evidence & References:**
- Project boundaries spec: `docs/specs/project-structure/spec.md`
- Interactive architecture diagram: `.archify/architecture-typr-20261125-165033/typr-architecture.html`
- Archify Code Analysis workbench: `.archify/code-analysis-typr-20261125-150900/analysis/repo.analysis.html`
- Related in-repo plan (do not duplicate): `docs/plans/2026-11-19-motion-coverage.md` already covers the 7 unused `ui/motion.rs` constants (`SLOW`, `SLOWER`, `RISE`, `SLIDE`, `ENTER`, `ENTER_SCALE`, `PRESS_SCALE_ICON`).

---

## Decisions

1. **Re-export stability during `typr-core` restructuring.** When domain types and `StoreError` move out of `store/mod.rs` into a leaf `model` (or `types`) module, keep `pub use` re-exports at `typr_core::store::*` during Phase 3 so callers migrate cleanly without a flag-day break; tighten imports in a follow-up commit if desired.
2. **Decision — `store::task` / `trait Store` (Phase 2 = Option B).** Wire `crates/typr/src/shell.rs` onto `Handle::spawn` for full off-thread persistence using an NSH reactive wake callback, removing direct `Db` access from `shell_over` and eliminating UI-thread SQLite stalls.
   - *(Alternative considered and rejected: Option A — retiring `store/task` and `trait Store` to keep `Db::apply` synchronous on the UI thread.)*
3. **Inherent match tables stay flat.** Do not artificially split `store::Command::name` (CC=54) or `keymap::Command::label`/`name` (CC=34); they have nesting depth 1 and exhaustive compiler checking. Record them in Archify config/baseline if adopting `code-analysis check` in CI.
4. **Toolchain drift in working tree.** Reconcile the uncommitted `mise.toml` bump (`rust 1.100.0`, `nsh 0.7.0`) against `rust-toolchain.toml` (`1.99.1`), workspace `Cargo.toml` (`nsh-* = "=0.6.0"`), and `README.md` (`1.99.1`) as an explicit separate commit — do not mix toolchain bumps into refactoring PRs.

---

## Global Constraints

- `typr-core` MUST remain toolkit-free (`cargo tree -p typr-core --edges normal` contains no `nsh-*`, no `log`, no ambient OS path/timezone reads).
- Storage format and schema version (`V12`) MUST NOT change; `crates/typr-core/tests/fixture.rs` stays green untouched.
- Every phase lands behind one locked workspace gate: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, and `bash scripts/check-tokens.sh`.
- Leave `ui/motion.rs` unused tokens to `docs/plans/2026-11-19-motion-coverage.md` so both plans stay orthogonal.

---

## Phases & Tasks

### Phase 1: Low-Risk Spec & Guardrail Hygiene

Small, self-contained fixes with zero architectural risk.

- [x] **1.1 Eliminate test-only directories (`spec.md` lines 100–106)**
  - Move `crates/typr-core/src/library_view.rs` → `crates/typr-core/src/library_view/mod.rs` (keeping `library_view/tests.rs` beside it).
  - Move `crates/typr-core/src/store/view.rs` → `crates/typr-core/src/store/view/mod.rs` (keeping `store/view/tests.rs` beside it).
  - *Verify:* `cargo test -p typr-core --locked` and confirm zero directories under `crates/` contain only `tests.rs`.

- [x] **1.2 Close `scripts/check-tokens.sh` and CI coverage gaps**
  - Add `crates/typr/src/screens/trash.rs` and `crates/typr/src/screens/library.rs` to `screens=(...)` in `scripts/check-tokens.sh`.
  - Wrap `nsh_base::TreeItem` (`screens/sidebar.rs:173`) and `nsh_base::CommandItem` (`screens/keymap.rs:103`) behind `crate::ui` re-exports/builders and tighten the `base=` regex in `scripts/check-tokens.sh` to catch any `nsh_base::` type outside `theme`/`tokens_of`.
  - Decide whether variable geometry `px(left|top|width|height)` in `screens/editor/diagram.rs:212-215` belongs inside a dedicated `ui/` positioned-box helper so `check-tokens.sh` can forbid all `px(` in screens; add negative test coverage in `crates/typr/tests/tokens.rs`. *(Resolved via `tokens::points(...)` and tightened `\b(px|rgb|rgba|hsl|hsla)\(` + `nsh_base::` checks.)*
  - Evaluate adding `tests/command.test.tsx` and `tests/library.test.tsx` (headless-compatible cases) to `.github/workflows/ci.yml:132`. *(Evaluated: `command.test.tsx` skips when `!platform.desktop`; `library.test.tsx` still asserts removed sidebar rows `"All cards"`/`"Diagnostics"`, so CI stays focused on `desktop.test.tsx`.)*
  - *Verify:* `bash scripts/check-tokens.sh` and `cargo test -p typr --test tokens --locked`.

- [x] **1.3 Remove panic risk, backend error leak, and untested inbox gap**
  - Replace `.expect("the card's own deck lists it")` at `crates/typr-core/src/store/db.rs:1318` with `StoreError::Missing { what: "card", id: card }`.
  - Move `NoDataDir` out of `typr_core::store::StoreError` (`store/mod.rs:940`) into a desktop startup error type in `crates/typr/src/paths.rs` (its only constructor is `paths.rs:34`).
  - Add unit tests for `crates/typr-core/src/inbox.rs` (`inbox_cards`, `Inbox::of_key`, archived ancestor exclusion, and 64-hop parent cycle guard).
  - *Verify:* `cargo test -p typr-core --locked && cargo test -p typr --locked`.

---

### Phase 2: Resolve `store::task` / `trait Store` Architecture (Decision Gate)

Execute **either** 2A or 2B per Decision 2 above.

- [~] **2A (Option A — Synchronous `Db` + targeted async I/O — skipped per Decision 2):**
  - Confirm no external consumer depends on `typr_core::store::Handle` or `trait Store`.
  - Remove `crates/typr-core/src/store/task/` and `pub trait Store` from `store/mod.rs` (along with `store/tests/protocol.rs` or keep only the synchronous command contract tests), leaving `Db::apply(&Command, now_ms)` as the single backend door.
  - Audit the 34 `Command` variants unused by `crates/typr/src/`: tag intentionally planned commands vs. dead legacy commands, and document which specs own each planned group.
  - If heavy operations (`ImportArchive`, `ExportLibrary`) exceed `SLOW_COMMAND_MS`, run only those on a dedicated scoped thread rather than routing every synchronous read/write through an unused 53-method actor.

- [x] **2B (Option B — Full Background Store Worker):**
  - Connect `crates/typr/src/shell.rs` to `Handle::spawn` with an NSH reactive wake callback, removing direct `Db` ownership from `shell_over`.
  - Route `ExportLibrary` / `ImportArchive` / `apply` / `quiet_apply` through `Handle` and drain `Event`s on wake, eliminating UI-thread SQLite stalls.
  - Delete `crates/typr/src/shell.rs`'s ad-hoc `db.snapshot()` republishing in favor of the worker's published `Arc<Snapshot>`.

- [x] **2.Gate:** Re-run `cargo test --workspace --locked` and Archify `code-analysis prepare` to confirm reduced surface duplication.

---

### Phase 3: Break `typr-core` Cycles & Split `store/db.rs`

Target the 13-file Archify cycle (`COUP-2ef222904980`), file concentration (`COUP-aebc2d9ee810`), and `Db::dispatch` complexity (`COMP-ec99815aad99`).

- [x] **3.1 Extract leaf domain types & errors out of `store/mod.rs`**
  - Create `crates/typr-core/src/model.rs` (or `types.rs`) holding pure value types with zero persistence imports: `Id`, `Phase`, `Grade`, `CardKind`, `Deck`, `CardRef`, `CardSummary`, `Settings`, `Snapshot`, `TagBranch`, `DuePrompt`, `NewPrompt`, `Stats`, `ReviewRow`, `MaskRect`, `MaskRow`, and `Folded`/`folded()`.
  - Give `fsrs::weights` its own typed error (e.g., `FsrsError`) converted via `#[from]` into `StoreError`, eliminating the `fsrs → store` edge. *(Extracted `error.rs` + `command.rs` + `TransferTarget`, breaking all `typr-core` cycles.)*
  - Point `schedule`, `edits`, `inbox`, `name`, `reference`, `session`, and `transfer` at `crate::model` instead of `crate::store` (re-exporting from `store/mod.rs` to preserve external paths).
  - *Verify:* Re-run Archify `code-analysis prepare` — `COUP-2ef222904980` (13-file cycle) drops to 0.

- [x] **3.2 Split `crates/typr-core/src/store/db.rs` (2,203 LOC) by domain**
  - Keep `store/db/mod.rs` focused on `Db`, `open`, `apply`, `staged`, `named_write`, and a thin `dispatch` router delegating to domain submodules:
    - `store/db/decks.rs` — deck CRUD, tree movement, default/reverse/view settings
    - `store/db/cards.rs` — card CRUD, drafts, trash/restore, template/field/reference/mask updates
    - `store/db/review.rs` — grading, scheduler selection, due/new queues, history, stats
    - `store/db/views.rs` — saved view CRUD and `cards_for_view`
    - `store/db/snapshot.rs` — `Db::snapshot` assembly and row folding
  - Keep `Db::dispatch` under ~120 lines (thin delegation arms) and every submodule under ~500 lines.
  - *Verify:* All 353+ `store/tests/*` and `tests/fixture.rs` pass unchanged; Archify `COUP-aebc2d9ee810` and `COMP-ec99815aad99` clear.

- [x] **3.3 Local core cleanups**
  - Deduplicate the xorshift PRNG between `crates/typr-core/src/shuffle.rs` and `crates/typr-core/src/store/view/mod.rs`.
  - Flatten `template::write_range` (`crates/typr-core/src/template.rs:178-245`, Archify warning `COMP-d1a07f6f4205`, nesting depth 6) using early returns / small token helpers.
  - *Verify:* `cargo test -p typr-core --locked`.

---

### Phase 4: Move Screen Domain Logic to `typr-core` & Decompose `shell_over`

Align with `spec.md` ("Card and deck edits are decided in the backend") and break up the 776-line `shell_over` god component.

- [x] **4.1 Relocate leaked domain rules into `typr-core` with headless tests**
  - Move `check_new_card_cap` (`screens/settings.rs:27`) to reuse `store/settings.rs` validation directly.
  - Move `due_counts` (`screens/sidebar.rs:559`) into `typr-core` alongside `inbox.rs` so sidebar counts and inbox filtering share one archived-deck / 64-hop walker.
  - Move `schedule_of` (`screens/grid.rs:57`, dropping the duplicate `const DAY_MS`) into `typr_core::schedule` or `typr_core::model`.
  - Move `entry_for` (`screens/study.rs:37`), `cards_in`/`tiles_for` filtering (`screens/library.rs:11,26`), and `review_count` (`screens/history.rs:22`) into `typr-core` where UI-free, leaving screens with pure view-model formatting.
  - *Verify:* Each moved rule has unit tests in `crates/typr-core` passing headless; `crates/typr` tests remain green.

- [x] **4.2 Split `crates/typr/src/shell.rs` (`shell_over`, 776 LOC)**
  - Extract cohesive submodules under `crates/typr/src/shell/`:
    - `shell/mod.rs` — `App`, `shell`, `shell_over` layout composition, `Screen` enum
    - `shell/store_bridge.rs` — `write` / `apply` / `quiet_apply`, `SLOW_COMMAND_MS` tracking, `import_from` / `export_to`
    - `shell/navigation.rs` — `visible`, `selection`, `step`, `select_end`, `open_selected`, keyboard dispatch
    - `shell/study.rs` — `start_session`, `overdue_days`, `wake_session`, interval tick
    - `shell/editor.rs` — editor draft/template/diagram reconstruction and event wiring (`shell.rs:785-880`)
  - *Verify:* `cargo test -p typr --test shell --locked` and `nsh test tests/desktop.test.tsx`.

---

### Phase 5: Public API Evolution & UI Event Hygiene

Bring both crates into full compliance with `spec.md` lines 137–174.

- [x] **5.1 Add `#[non_exhaustive]` + constructors / `Default` to public-field structs**
  - `crates/typr-core` (30 structs): prioritize high-churn DTOs first (`CardSummary`, `Deck`, `Settings`, `Snapshot`, `ImportReport`, `Policy`, `PromptState`, `Outcome`, `Event`, `Binding`, `ViewSpec`, `SavedView`), then the remaining 18; provide `Default` or named constructors so cross-crate callers never use struct literals.
  - `crates/typr` (28 structs): add `#[non_exhaustive]` to screen props/models (`EditorProps`, `CardText`..`DiagramStart`, `Tile`, `TrashRow`, `Record`, `SlowCommand`, `Diagram`) and `ui/*Props` (confirming `#[derive(Props)]` compatibility with `#[non_exhaustive]` in `nsh-core` first; if the macro rejects it, document the exact exemption in `spec.md`). *(Confirmed: `#[derive(Props)]` accepts `#[non_exhaustive]`.)*
  - *Verify:* Compile all integration tests (`crates/typr-core/tests/*`, `crates/typr/tests/*`) as external-crate callers with `--locked`.

- [x] **5.2 Convert raw `ui/` event payloads to named act objects**
  - Replace lone tuple values and state-named variants in `crates/typr/src/ui/`:
    - `ui/choice.rs`: `TabRowEvent::Select(usize)` → named struct `{ index }`; `ChooseEvent::Change(Option<SharedString>)` → named struct `{ value }`
    - `ui/input.rs`: `FieldEvent::Change(String)` → named struct `{ text }`
    - `ui/toggle.rs`: `CheckEvent::Change(bool)` / `ToggleEvent::Change(bool)` → named struct `{ checked }` / `{ on }`
    - `ui/tree.rs`: `TreeEvent::Selected` / `Expanded` → act-named variants with named fields (`Select(TreeSelect { id })`, `Expand(TreeExpand { expanded })`)
  - Extract a shared focus-ring style helper (`tokens::focus_ring`) in `crates/typr/src/ui/tokens.rs` to replace the 8 identical `.focus_visible(...)` chains across `ui/choice.rs`, `ui/toggle.rs`, `ui/segmented.rs`, `ui/input.rs`, and `ui/button.rs`.
  - *Verify:* `cargo test --workspace --locked` and `nsh test`.

---

## Final Verification Gate

Run once after each phase (and after all phases complete):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -W clippy::cast_possible_truncation
cargo test --workspace --locked
bash scripts/check-tokens.sh
node /Users/nick/.agents/skills/archify/bin/archify.mjs code-analysis prepare . --out .archify/code-analysis-post-refactor --language rust --locale en
```

Success criteria:
- Zero Archify `warning` findings (excluding any explicitly accepted flat `Command::name`/`label` tables).
- Zero `coupling/import-cycle` findings in `typr-core`.
- Zero test-only directories; zero `scripts/check-tokens.sh` blind spots; all `spec.md` requirements verified green.
