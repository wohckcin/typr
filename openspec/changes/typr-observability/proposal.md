# Proposal

## Why

A failure in typr currently leaves no durable trace. The worst case is silent: if the store task panics, the ids in the batch it was applying never receive an event, so a pending grade leaves the card unanswerable (`Session::awaiting` stays set, `gradeable()` returns false) and the screen says nothing about why. Nothing is written to disk, and stderr — the only place the panic hook prints — is invisible when the app is launched from a desktop launcher.

## What Changes

- Add a panic boundary around the store task: a panic answers the ids in flight with `StoreError::Closed`, publishes one unprompted error event so the failure line explains the stoppage, and lets the session clear its pending grade.
- Record the store task's exit instead of discarding it: `Handle::close` keeps the `JoinHandle` result, so a panicked worker is reported rather than swallowed.
- Add a log file at `<AppDirs::logs()>/typr.log`, capped and rotated to `typr.log.1`, written through the `log` facade with the level taken from `TYPR_LOG` (default `warn`), so a bug report has something to attach.
- Log the cause chain rather than the top message: a failure records every `source()` link, while the status bar keeps its single line and the typed error.
- Record the facts a support question always asks for: database path, schema version before and after migration, and any command that took longer than a threshold.
- Out of scope: span-level timing (`tracing`), crash reporting services, log upload, structured JSON sinks, and logging inside Rune.

## Capabilities

### New Capabilities

- `diagnostics`: what typr records about itself — the panic boundary around the store task, the log file and its rotation and level, the content a failure record carries, and the guarantee that a stoppage is visible on screen as well as on disk.

### Modified Capabilities

None. `openspec/specs/` is still empty; the four typr-v1 capabilities live inside their change until it is archived.

## Impact

- `src/store/task.rs`: the worker loop gains a panic boundary and an answer for the orphaned ids; `close` stops discarding the join result.
- `src/main.rs`: initialises the logger after `AppDirs` resolves, and reports a failure through the same chain-aware helper the log uses.
- New module: `logging` (sink, rotation, level, chain walker).
- `Cargo.toml`: adds `log = "0.4"` as a direct dependency. It is already in the lock through Rune, so the build gains no new crates.
- Tests: a panicking store task reports instead of hanging, the log file holds the cause chain, the level filter is honoured, and a rotated file keeps the previous run.
- Nothing in the review loop changes: grades, schedules and stored data are untouched.
