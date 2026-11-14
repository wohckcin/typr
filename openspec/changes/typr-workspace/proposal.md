# Proposal

## Why

`typr` is one crate that holds the store, the scheduler, the session state machine and every Rune
view at once. Nothing forces the backend to stay free of the UI toolkit, so the store already reaches
for `rune::dirs::AppDirs` to find its own file, and handing the library to another front end — a CLI,
a test harness, a service — would drag winit and wgpu along with it. Splitting the package into a
backend crate and a desktop crate makes that boundary real, and makes a whole-tree test run possible
on a machine with no display.

## What Changes

- Turn the repository into a Cargo workspace with two member crates:
  - `typr-core` — the backend library: store, scheduler, session, sides splitter, shuffle, study-day
    arithmetic and the clock. It depends on `rusqlite`, `thiserror` and `unicode-normalization`, and
    on nothing else.
  - `typr` — the desktop app: Rune views, the binary, scenes and their snapshots. It depends on
    `typr-core`, `rune`, `rune-kit` and `time`.
- **BREAKING** the store stops resolving its own location. `database_path`, `attachments_root`,
  `open_from_environment` and `open_in` move to the desktop crate because they are built on
  `rune::dirs::AppDirs`; `typr-core` takes a path and opens it.
- **BREAKING** the study-day offset stops being ambient. `local_offset_ms()` and its `LazyLock` move
  to the desktop crate, because `time` can only read the local zone on the main thread of a
  single-threaded program and only the front end knows when that moment is. `typr-core` keeps
  `study_day(at_ms, offset_ms)` and `study_day_bounds(day, offset_ms)` as pure arithmetic and every
  query continues to receive the offset as an argument.
- The four Rune `[patch.crates-io]` entries move from the package manifest to the workspace root,
  which is the only place a patch can apply, so they are written once instead of in every consumer.
- New capability `project-structure` records the boundaries the split is meant to keep: the backend
  builds and tests without Rune, the backend never reads the machine's timezone itself, and the
  workspace pins Rune's revision once.

## Capabilities

### New Capabilities

- `project-structure`: the crate boundaries and the dependency rules that keep the backend usable
  without the desktop toolkit. Covers what `typr-core` may depend on, how the study-day offset
  reaches it, and where Rune's revision and patch entries live.

### Modified Capabilities

None. No requirement in `card-library`, `card-editor`, `review-scheduling` or `study-session`
changes: the split moves code and changes no behaviour a user can observe. Those capabilities are
still carried by the unarchived `typr-v1` change, and this proposal leaves them alone.

## Impact

- New `Cargo.toml` at the root (virtual workspace) with `members = ["crates/typr-core", "crates/typr"]`
  and the four `[patch.crates-io]` entries; the current package manifest moves to `crates/typr/`.
- `rust-toolchain.toml`, `.github/workflows/ci.yml` and `README.md` stay at the root; CI gains
  `cargo test --workspace` and a check that `typr-core`'s dependency graph contains no Rune crates.
- Modules move: `clock`, `day` (arithmetic only), `schedule`, `session`, `sides`, `shuffle`, `store`
  and `Queue` go to `typr-core`; `library`, `editor`, `history`, `study`, `attachments`, `Screen`,
  `main`, the bin-level tests, `tests/dirs_root.rs` and the new path helpers stay in `typr`.
- All desktop code that names the store changes from `crate::store::…` / `typr::store::…` to
  `typr_core::…`; roughly 2400 lines of view code gains a new import prefix and no new logic.
- `time` and `unicode-normalization` swap sides: `time` becomes a desktop-only dependency,
  `unicode-normalization` stays with the store.
- No schema, storage-format or scheduling change: a library written before the split opens
  unchanged after it.
