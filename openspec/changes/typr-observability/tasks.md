# Tasks

The layout this change works in: `crates/typr-core` holds the store and must keep exactly three normal dependencies, so everything here that logs lives in `crates/typr`.

## 1. Backend reporting

- [x] 1.1 Add `StoreError::WorkerPanicked { message: String }` to `crates/typr-core/src/store/mod.rs` with a message naming the stoppage, and make sure it stays `#[non_exhaustive]`-compatible for the desktop's match arms.

  **Verification**: a test asserts the variant's `to_string()` names the stoppage and that `source()` stays `None`.

- [x] 1.2 Copy the in-flight message id into a local before `apply` is called, and wrap the apply-and-publish pair in `catch_unwind`, documenting why `AssertUnwindSafe` is sound.

  **Verification**: a test sends one command that panics and asserts the answer for that id arrives.

- [x] 1.3 After a caught panic, answer the panicking id and every id still queued with `Closed`: drain the receiver with `try_recv`, then end the loop and drop the receiver.

  **Verification**: a test queues three commands where the first panics, and asserts all three ids receive an answer, the last two with `Closed`.

- [x] 1.4 Push exactly one unprompted event per failure (`UNPROMPTED` as the id) carrying `WorkerPanicked`, with the panic message read through `downcast_ref` for `&str` and `String` and the payload's type name otherwise.

  **Verification**: a test asserts the events hold one entry with id 0 whose message contains the panic text; a second test panics with a non-string payload and asserts the type name appears.

- [x] 1.5 Add `WorkerStatus` (`Arc<Mutex<Option<StoreError>>>`) to `crates/typr-core/src/store/task.rs`, fill it from the loop's end including a panic, and expose it as `Handle::status()`.

  **Verification**: a test panics the worker, drops the handle, and asserts the status still answers the panic afterwards; a clean shutdown leaves it `None`.

- [x] 1.6 Capture the schema facts during migration and expose them: `SchemaFacts { found, written, applied }` from `Db::schema_facts()`.

  **Verification**: a test opens a library at version 2 and asserts `found == 2`, `written == CURRENT_VERSION`, `applied == 1`; a test that refuses a newer version still reports the version found.

- [x] 1.7 Keep later sends answering `Closed` after the boundary fires, and prove the pending-grade path clears.

  **Verification**: a test sends a command after a panic and asserts `StoreError::Closed`; a test drives a grade through the app state while the worker panics and asserts the card becomes answerable again.

## 2. Logging, in the desktop crate

- [x] 2.1 Add `log = "0.4"` to `crates/typr/Cargo.toml` and a `crates/typr/src/logging.rs` whose `log::Log` implementation filters by `LevelFilter`, formats one line per record, and hands it to a bounded channel with `try_send`; no file is opened on that thread.

  **Verification**: a unit test writes a record from the test thread, drains the channel, and asserts the formatted line; a second test asserts nothing is sent below the filter; `cargo tree -p typr-core --edges normal` still names no logging crate.

- [x] 2.2 Add the writer thread that owns the log file: it creates the logs folder, writes each line with a timestamp from `time`, and counts records dropped when the channel was full, reporting the count on the next successful write.

  **Verification**: a test sends a record through and reads it back from the file; a test that shrinks the channel to 1 and sends 3 records asserts two are dropped and the next write reports the count.

- [x] 2.3 Rotate once on open: when the file exceeds the size limit, rename it to `typr.log.1` replacing any previous copy, then start an empty file.

  **Verification**: a test pre-creates a file larger than the limit with known content, starts logging, and asserts the rotated copy holds the old content and the new file holds only new records.

- [x] 2.4 Install the logger from the resolved paths (`<logs()>/typr.log`), reading `TYPR_LOG` for the level with `warn` as the default when it is unset or unparseable.

  **Verification**: a test sets `RUNE_DIRS_ROOT` and `TYPR_LOG=info`, installs the logger, logs, and finds the file under that root; a second test with `TYPR_LOG` unset asserts an informational record is not written.

- [x] 2.5 Flush after the run loop, not during teardown: `main` calls the shutdown routine once `rune::run` returns. The writer answers a flush request on a reply channel and the caller waits with `recv_timeout`, because `std::thread::JoinHandle` has no timed join; a timeout is recorded and the process continues.

  **Verification**: a test logs, calls the shutdown routine, then reads the file and finds the record; a second test asserts shutdown twice is harmless.

- [x] 2.6 Log a failure with its whole cause chain: `fn describe(error: &dyn Error) -> String` joining `Display` with each `source()`.

  **Verification**: a test builds a `StoreError` from an `io::Error` with a known permission message and asserts the description contains both typr's message and the operating-system message.

## 3. Wire it into the application

- [x] 3.1 Log every failure the interface already shows, using the chain-aware description while the failure line keeps the single-line message.

  **Verification**: a test refuses a write path, then asserts the failure line is one line and the log holds the cause chain.

- [x] 3.2 Record startup facts at information level from `Db::schema_facts()` and `paths`: database path, the version found, the version written, how many migrations ran, and the log path.

  **Verification**: a test opens a library from an older version at information level and asserts both versions and the count appear; a test refuses a newer version and asserts the refusal is recorded.

- [x] 3.3 Record the worker's end at quit: `main` keeps the `WorkerStatus`, and once `rune::run` returns it records a panicked end at `error` before flushing.

  **Verification**: a test panics the worker, drops the handle, runs the shutdown path, and finds the abnormal-end record in the file; the ordering test in 2.5 covers the record surviving to disk.

- [x] 3.4 Log any command whose answer takes longer than the threshold at warning level with its name, id and duration, and nothing below it, measuring from submit to answer in the application.

  **Verification**: a test with a threshold of zero asserts a record per command; a test with a threshold above the measured cost asserts no such record.

- [x] 3.5 Give the failure line a headless scene so a stoppage is reviewable without a GUI session.

  **Verification**: `RUNE_DIRS_ROOT` set and `--scene` on the failure scene write a non-blank PNG, and the scene list includes it.

## 4. Verification and documentation

- [ ] 4.1 Cover every scenario in `specs/diagnostics/spec.md` with a test at the level where it is observable, and reverse each guard once in a scratch worktree with its own `CARGO_TARGET_DIR` to confirm the test fails without the change.

  **Verification**: the list of scenario-to-test pairs is complete, and each reversed guard has a recorded failing run.

- [ ] 4.2 Keep the boundary in the backend and the sink in the app: extend the CI guard so it fails if `typr-core` ever gains `log`, a file helper or a timestamp formatter.

  **Verification**: the CI step passes on the current graph, and fails when `log` is added to `crates/typr-core/Cargo.toml` in a scratch worktree.

- [ ] 4.3 Update `README.md` with the log path, the `TYPR_LOG` variable, the size limit and the rotated name, and record in `design.md` anything the implementation settled differently.

  **Verification**: the README names the path and variable, and `openspec validate typr-observability --strict` is valid.
