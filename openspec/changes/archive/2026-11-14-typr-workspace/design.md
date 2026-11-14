# Design

## Context

See `proposal.md` — Why for the motivation. The current state that shapes the approach:

- One package, `typr`, with `src/lib.rs` exporting thirteen modules and `src/main.rs` as the binary.
  Sizes: store 3452 lines with its tests, views 2408, binary plus its tests 1953, Rune-free domain
  modules 1125.
- Three things cross from the backend to Rune today:
  - `src/store/mod.rs:14` `use rune::dirs::AppDirs`, used by `database_path` (`:443`),
    `attachments_root` (`:448`), `attachments_root_from_environment` (`:453`), `open_from_environment`
    (`:462`) and `open_in` (`:468`).
  - `src/store/task.rs:14` `use rune::Waker` — Rune's `Waker` is `Arc<dyn Fn() + Send + Sync>`.
  - `src/attachments.rs:6` `use rune::{Image, ImageCache, Waker}` — genuinely view-level, it turns a
    file into a picture.
- `src/day.rs` mixes two things: pure arithmetic (`ROLLOVER_MS`, `study_day`, `study_day_bounds`) and
  the ambient offset (`local_offset_ms`, `LOCAL_OFFSET_MS`, the `time` dependency). Its only callers
  are `src/history.rs:46` and `src/main.rs:385,400,602`, all desktop code. `Db::due_prompts(now,
  offset, seed)`, `new_prompts(..)` and `stats_for_day(..)` already take the offset as a parameter,
  so the backend never needs to read the zone.
- `time` can only read the local offset on the main thread of a single-threaded program, which is
  why `src/main.rs:602` primes it before the window opens. That is a property of the process that
  hosts the UI, not of the library.
- `[patch.crates-io]` applies only at a workspace root, so four entries that currently sit in the
  package manifest must move up one level once the package becomes a member.
- Rune binds widget callbacks to the application's own state type, which is why every view module
  defines a `Host` trait and `State` implements all of them. The split does not change that; it only
  changes the path other modules are imported from.

## Goals / Non-Goals

**Goals:**

- Two member crates in one workspace: a backend library with no desktop dependencies, and the desktop
  app that depends on it.
- The backend's tests run on a machine with no display and no GPU driver.
- Zero behaviour change: same schema version, same file format, same screens, same snapshots.
- Preserve file history for the moved modules.

**Non-Goals:**

- A network service, protocol or serialisation layer. Reading B was considered and declined.
- A third crate just for the binary, or a published `typr-core` on crates.io.
- Changing the store's protocol, the `Store` trait's shape, or the error type. `StoreError` keeps its
  `Sqlite`/`Io` variants because its consumers are Rust.
- Rewriting `Host` traits, views or scenes. This move edits import paths, not logic.
- Making the backend async or runtime-agnostic beyond what it already does with `std::thread`.

## Decisions

**1. A virtual workspace root with two members, in `crates/`.**

`Cargo.toml` at the root declares `[workspace] members = ["crates/typr-core", "crates/typr"]` and
holds the four `[patch.crates-io]` entries. Alternatives: keep the root as the `typr` package and add
`typr-core` as a member (works, but leaves the root doing two jobs and the patch section beside a
package manifest that no longer owns it); three crates with a separate binary crate (buys nothing
here, the views and the binary always change together).

**2. What moves, and why on that side.**

| Goes to `typr-core` | Stays in `typr` |
|---|---|
| `clock`, `schedule`, `session`, `sides`, `shuffle` | `library`, `editor`, `history`, `study`, `attachments` |
| `store` (db, schema, task, mod) | `main`, `src/tests.rs` |
| `day` arithmetic: `ROLLOVER_MS`, `study_day`, `study_day_bounds` | `day`'s offset source: `local_offset_ms`, `LOCAL_OFFSET_MS` |
| `Queue` | `Screen` |
| `store/tests/*`, `tests/e2e.rs` | view tests, `tests/dirs_root.rs` |

`Queue` is on the core side because `Session` holds one; `Screen` names views and belongs to whoever
draws them. `tests/dirs_root.rs` asserts a property of Rune's directory resolution, so it follows
Rune.

**3. The backend takes a path; the desktop crate owns the layout.**

`typr-core` keeps `Db::open(&Path)` and `Db::open_in_memory()`-style entry points and loses every
helper that resolves a directory. The desktop crate gains a `paths` module holding `database_path`,
`attachments_root`, `open_from_environment` and `open_in`, built on `AppDirs`. Alternatives: give
core a `Paths { data: PathBuf }` type (moves policy into the library without removing the Rune
dependency that produced the paths); adopt the `directories` crate in core (a new dependency, and
`RUNE_DIRS_ROOT` already provides the override tests and scene runs rely on).

**4. The offset is an argument, and its source moves to the desktop crate.**

`day.rs` splits along the line above. `study_day` and `study_day_bounds` stay pure functions of
`(instant, offset)`; `local_offset_ms()` and the `LazyLock` move beside `main`, and the `time`
dependency becomes desktop-only. Alternatives: a `TimeZone` trait in core injected at construction
(hides a parameter every query already takes explicitly); re-reading the offset per call (fails off
the main thread on Unix — this is why it was cached in the first place).

**5. `Waker` becomes a local alias in the core crate.**

`pub type Waker = Arc<dyn Fn() + Send + Sync>` — the same type, declared where it is used, so the
store worker keeps its "wake the UI" callback without importing Rune. Alternative: have the worker
only push events and let the app poll on a timer (adds latency to every command's visible effect for
no gain).

**6. Import churn is mechanical and compiler-driven.**

Desktop modules change `use crate::store::…` and `typr::store::…` to `typr_core::…`. No module is
rewritten, no function is renamed. The move is done one module group at a time so the compiler states
the next missing import rather than a wall of errors.

**7. CI states the boundary, and the workspace is tested as one.**

`cargo test --workspace --locked` replaces `cargo test --locked`; a new step fails the build when
`cargo tree -p typr-core --edges normal` names a Rune crate; the scene loop runs `-p typr` because
scenes belong to the desktop crate.

**8. Names: `typr-core` and `typr`.**

The library is the backend, the app keeps the project's name. If a different pair is wanted, the
change is a rename inside this refactor, not a redesign.

## Risks / Trade-offs

- **Wide, shallow diff** (roughly 2400 lines of import changes) → do it in module-group commits with
  `cargo check --workspace` between each; the compiler is the checklist.
- **A silent behavioural change slipping in with the move** → the move commits contain no logic
  edits; the full suite and the ten scene snapshots run before and after, and the snapshots are
  compared by size and drawn-content assertions already in the tests.
- **`Cargo.lock` churn** → regenerate once at the end of the move, commit it, and keep `--locked` in
  CI so a stale lock fails loudly instead of drifting.
- **Test hooks that are `pub(crate)`** (`drop_card_table_for_test`, `append_record_for_test`,
  `record_total`) → they are used only from `src/store/tests/`, which moves with them; a GUI test
  that reached for one would fail to compile, which is the desired signal.
- **Dev-dependency duplication** (`tempfile` needed by both crates' tests) → duplicated in each
  manifest; harmless.
- **Two lockstep manifests to bump** → the patch entries live only at the root, so a Rune bump is
  still one file plus whichever member names the revision (only `typr` does).
- **`AppDirs` reachability regression** → `RUNE_DIRS_ROOT` must keep working for scenes and tests;
  `tests/dirs_root.rs` guards it from the desktop side.

## Migration Plan

1. Add the workspace root manifest, move the package to `crates/typr/`, and add an empty
   `crates/typr-core`; `cargo check --workspace` must pass with all code still in the desktop crate.
2. `git mv` module groups into `typr-core` in dependency order — `clock`, `sides`, `shuffle`, `day`
   arithmetic, `schedule`, `session`, `store` — fixing imports until `cargo check -p typr-core`
   passes without Rune.
3. Remove the three leaks: drop `AppDirs` from the store, alias `Waker` locally, move the offset
   source out of `day`.
4. Add the desktop `paths` and `day_offset` modules and point `main` at them.
5. Move tests, regenerate and commit `Cargo.lock`, update CI and `README.md`.
6. Verify: `cargo fmt --check`, clippy with the existing flags, `cargo test --workspace --locked`,
   `cargo tree -p typr-core --edges normal`, and all ten scenes snapping.

Rollback: no data migration is involved, so reverting the commits restores the single-crate layout
and the existing library file keeps working throughout.

## Open Questions

- Whether a future CLI should be its own member (`crates/typr-cli`) rather than a mode of the desktop
  binary. Deferrable: it depends on the core crate existing, and the layout above accepts a third
  member without restructuring.
