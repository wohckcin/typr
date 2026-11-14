# Tasks

## 1. Workspace skeleton

- [x] 1.1 Add a virtual workspace manifest at the repository root: `[workspace] members = ["crates/typr-core", "crates/typr"]`, the four Rune `[patch.crates-io]` entries, and `rust-toolchain.toml` left where it is. Move the current package manifest and `Cargo.lock` to `crates/typr/` with `git mv` so file history follows the files. **Verify**: `cargo metadata --format-version 1 | jq '.packages | length'` reports two packages, the root manifest parses as a workspace, and `cargo check --workspace` passes with every module still in the desktop crate.
- [x] 1.2 Create `crates/typr-core/` with an empty `lib.rs` and exactly three dependencies: `rusqlite` (bundled), `thiserror`, `unicode-normalization`. **Verify**: `cargo tree -p typr-core --edges normal` lists no Rune, wgpu, winit, parley or accesskit crate, and `cargo check -p typr-core` passes.
- [ ] 1.3 Regenerate and commit the single root `Cargo.lock`. **Verify**: `cargo build --workspace --locked` succeeds from a clean `target/` for both members, which proves the lock file covers the workspace rather than one package.

## 2. The pure domain moves

- [ ] 2.1 `git mv` `clock`, `sides` and `shuffle` (with their `tests.rs` siblings) into `crates/typr-core/src/`, and re-export them from its `lib.rs`. **Verify**: those modules' existing tests run under `cargo test -p typr-core` and `cargo check --workspace` passes.
- [ ] 2.2 Split `day.rs`: keep `ROLLOVER_MS`, `study_day` and `study_day_bounds` in `crates/typr-core/src/day.rs` as pure arithmetic; move `local_offset_ms()` and its `LazyLock` to `crates/typr/src/day_offset.rs`, and point `src/main.rs:385,400,602` and `src/history.rs:46` at it. **Verify**: `cargo tree -p typr-core -i time` reports that `time` is not in the tree, `cargo test -p typr-core day` passes, and `cargo test -p typr` still covers the formatting in `history::when` and the offset priming.
- [ ] 2.3 Move `schedule` and `session` (with their tests) into the core crate, and split `screen.rs`: `Queue` goes to `typr-core`, `Screen` stays in the desktop crate. **Verify**: the schedule and session test suites (including `a_reshuffle_reorders_only_within_one_due_day`) run under `cargo test -p typr-core`, and `cargo check --workspace` passes.
- [ ] 2.4 Point the desktop crate's imports for those modules at `typr_core::` — no logic edits, only paths. **Verify**: `cargo check --workspace` is clean and the desktop crate's tests are unchanged in count and outcome.

## 3. The store moves and loses its Rune reach

- [ ] 3.1 `git mv` `store/` (db, mod, schema, task and the `tests/` split) into `crates/typr-core/src/store/`. **Verify**: `cargo test -p typr-core` runs the store suites and `cargo tree -p typr-core --edges normal` still lists no Rune crate apart from the two leaks in 3.2 and 3.3.
- [ ] 3.2 Replace `use rune::Waker` in the store with `pub type Waker = Arc<dyn Fn() + Send + Sync>` declared in the core crate. **Verify**: `cargo test -p typr-core` still passes the worker tests (`one_event_per_command`, `dropping_the_handle_writes_what_was_sent`, `a_poisoned_lock_is_still_usable`) and the core crate names no Rune type anywhere in `src/`.
- [ ] 3.3 Delete `database_path`, `attachments_root`, `attachments_root_from_environment`, `open_from_environment` and `open_in` from the store, leaving `Db::open(&Path)` as the only way in. Add `crates/typr/src/paths.rs` with those four helpers built on `AppDirs`, and point `State::setup` and `Editor`'s attachment root at it. **Verify**: `cargo tree -p typr-core --edges normal` names no Rune crate at all, `cargo test -p typr-core` passes, and the desktop crate's `tests/dirs_root.rs` still proves `RUNE_DIRS_ROOT` keeps the real data folder untouched.
- [ ] 3.4 Move `tests/e2e.rs` to `crates/typr-core/tests/e2e.rs` and keep `tests/dirs_root.rs` in the desktop crate. **Verify**: `cargo test -p typr-core --test e2e` passes with `DISPLAY` and `WAYLAND_DISPLAY` unset, which is the headless claim in `project-structure` checked where it can be checked.

## 4. The desktop crate settles

- [ ] 4.1 Point every view and the binary at `typr_core::` (`library`, `editor`, `study`, `history`, `main`, `src/tests.rs`) and fix whatever the compiler reports. **Verify**: `cargo test --workspace` passes with the same test counts as before the move.
- [ ] 4.2 Run every scene to a snapshot and compare against the pre-move run. **Verify**: all ten scenes write a non-empty PNG under a temporary `RUNE_DIRS_ROOT`, the set of scene names is unchanged, and each image's byte size is within a few bytes of the pre-move run recorded in the change's notes.
- [ ] 4.3 Update `README.md`: workspace layout, `cargo run -p typr`, `cargo test --workspace`, and the note that Rune's revision and patch entries live in one root manifest. **Verify**: following the README from a clean checkout reaches a running app and a passing test run.

## 5. Enforce the boundary

- [ ] 5.1 Update `.github/workflows/ci.yml`: `cargo test --workspace --locked`, a step that fails when `cargo tree -p typr-core --edges normal` matches `rune|wgpu|winit|parley`, and a scene loop that runs `cargo run -p typr` (or the `-p typr` binary). **Verify**: the YAML parses, and every step is replayed locally with the new flags.
- [ ] 5.2 Add the compatibility check the `project-structure` spec asks for: keep a library file written before the split (schema version 3) under `crates/typr-core/tests/data/`, open it after the move, and grade a card. **Verify**: the test passes and the file's `user_version` is unchanged afterwards.
- [ ] 5.3 Run the whole gate once from a clean tree. **Verify**: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::cast_possible_truncation`, `cargo test --workspace --locked`, the dependency-graph check and the scene loop all pass; `openspec validate typr-workspace --strict` is valid.

## Workflow follow-up

- Archive `typr-workspace` once the gate in 5.3 is green, which adds `project-structure` to `openspec/specs/`.
- Decide, before archiving, whether `typr-v1` should be archived first so the four behavioural capabilities exist in `openspec/specs/`; they are independent of this change.
