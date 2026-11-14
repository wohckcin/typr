# Proposal

## Why

A failure in typr currently leaves no durable trace. The worst case is silent: if the store task panics, the ids in the batch it was applying never receive an event, so a pending grade leaves the card unanswerable (`Session::awaiting` stays set, `gradeable()` returns false) and the screen says nothing about why. Nothing is written to disk, and stderr — the only place the panic hook prints — is invisible when the app is launched from a desktop launcher.

## What Changes

- Add a panic boundary around the store task in `typr-core`: a panic answers the ids in flight and everything still queued with `StoreError::Closed`, publishes one unprompted event carrying a new `StoreError::WorkerPanicked`, and lets the session clear its pending grade. The backend reports; it does not log.
- Make the worker's end observable after the handle is gone: `Handle` exposes a `WorkerStatus` the application keeps, so a panicked worker can be recorded at quit instead of being swallowed by `let _ = worker.join()`.
- Expose what a report needs as data, not as log lines: `Db::schema_facts()` answers the version found, the version written and how many migrations ran.
- Add a log file at `<AppDirs::logs()>/typr.log` in the desktop crate, capped and rotated to `typr.log.1`, written through the `log` facade with the level taken from `TYPR_LOG` (default `warn`), so a bug report has something to attach.
- Log the cause chain rather than the top message: a failure records every `source()` link, while the status bar keeps its single line and the typed error.
- Record the facts a support question always asks for: database path, schema version before and after migration, and any command whose answer took longer than a threshold, measured where the application waits for it.
- Out of scope: span-level timing (`tracing`), crash reporting services, log upload, structured JSON sinks, and logging inside Rune.

## Capabilities

### New Capabilities

- `diagnostics`: what typr records about itself — the panic boundary around the store task, the log file and its rotation and level, the content a failure record carries, and the guarantee that a stoppage is visible on screen as well as on disk.

### Modified Capabilities

None. `project-structure` requires the backend to hold exactly three normal dependencies and to know nothing of `AppDirs` or `time`, and this change keeps that: the log sink, the rotation, the level and the timestamps all live in `crates/typr`, and `typr-core` gains no logging crate.

## Impact

- `crates/typr-core/src/store/task.rs`: the worker loop gains a panic boundary, an answer for the orphaned ids, and a `WorkerStatus` it publishes when the thread ends.
- `crates/typr-core/src/store/mod.rs`: `StoreError::WorkerPanicked { message }`; `Db::schema_facts()` and the `SchemaFacts` it returns, captured during migration.
- `crates/typr/src/logging.rs` (new): sink, rotation, level, chain walker.
- `crates/typr/src/main.rs`: installs the logger once `paths` resolves, records the worker's end and flushes after `rune::run` returns, and logs each failure it already draws.
- `crates/typr/Cargo.toml`: adds `log = "0.4"`. It is already in the lock through Rune, so the build gains no new crates. `crates/typr-core/Cargo.toml` is unchanged.
- Tests: a panicking store task reports instead of hanging, the log file holds the cause chain, the level filter is honoured, and a rotated file keeps the previous run.
- Nothing in the review loop changes: grades, schedules and stored data are untouched.
