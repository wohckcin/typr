# Design

## Context

See `proposal.md` — Why. The pieces this design rests on already exist:

- `StoreError` is a `#[non_exhaustive]` enum derived with `thiserror`; `Sqlite` and `Io` keep their causes reachable through `source()`.
- `Event { id, result: Result<Outcome, StoreError> }` is the only answer channel. `Handle::submit` already answers `StoreError::Closed` once the worker's `Receiver` is gone (`src/store/task.rs:96-103`), and `remember`/`submit_grade` draw that on the failure line.
- `AppDirs::logs()` exists (`crates/rune/src/dirs.rs:92`) and nothing writes to it. `RUNE_DIRS_ROOT` already redirects `AppDirs`, which is the seam tests use.
- `log 0.4.35` is in the lock through Rune, so a direct dependency adds no crates.

The gap this change closes is narrower than "the app goes silent". A worker panic drops its `Receiver`, so later sends do answer `Closed`. What is lost is the batch that was being applied: its ids never receive an event, so `Session::awaiting` stays set, `gradeable()` stays false, and the card is unanswerable until the next send or a leave.

## Goals / Non-Goals

**Goals:**

- A worker panic is visible twice: on screen, through one unprompted error event, and on disk, with the panic message.
- Every id a send returned gets an answer, including the panicking message and everything still queued, so no pending request is orphaned.
- A failure record carries the whole `source()` chain, while the status bar keeps one line.
- The log survives a restart, is bounded in size, and its path follows `AppDirs`.
- Nothing about grading, scheduling or stored data changes.

**Non-Goals:**

- No span timing, no `tracing` subscriber, no per-frame metrics. `--trace` already covers frame cost.
- No crash upload, no telemetry, no JSON sink.
- No supervision that restarts the store task after a panic: a panic means the connection is suspect, and a restart would hide the bug.
- No logging inside Rune.

## Decisions

**The panic boundary sits inside `work`, around one batch, not around the thread.** The worker applies every queued message, then publishes. A `catch_unwind` around that pair answers every id that has no answer yet with `Err(StoreError::Closed)` — which ids those are is the next decision — and pushes one `Event { id: UNPROMPTED, result: Err(..) }` carrying the panic message, so the failure line says "the library stopped" instead of nothing. Alternative considered: a supervisor thread that respawns the worker on panic. Rejected because the `Connection` and the clock live inside the loop, a respawn needs ownership gymnastics, and a restart after a panic is the behaviour that makes such bugs unfindable. A `catch_unwind` around the whole loop was also rejected: after one panic the loop must not continue with a connection whose state is unknown, and the ids of that batch are exactly what needs answering.

**The panic message comes from `catch_unwind`'s payload, read through `downcast_ref`.** A payload that is neither `&str` nor `String` is recorded as `Box<dyn Any>`'s type name, because losing the message is worse than losing its type. `AssertUnwindSafe` is used deliberately and documented: the boundary answers every outstanding id before the loop ends, and the batch vector is dropped, so no partially applied state escapes it.

**Every id `submit` handed out gets exactly one answer, including the two sets the batch never holds.** A `u64` returned from `submit` is a promise, and `Session::awaiting` waits on it. The `batch` vector only holds events that already have their answer, so the boundary has two more sets to close. The message whose `apply` panicked was consumed by that call, so its `message.id` is copied into a local before `apply` is invoked and answered after the panic is caught. Everything still queued in `rx` was accepted earlier — each of those sends returned `Ok(id)` — and those messages die with the `Receiver`, so the boundary drains `try_recv` until it is empty and answers each id with `Closed`. Only then does the loop end and drop the `Receiver`, which is what makes later sends answer `Closed` through `submit`'s own path. Answering only "the ids it was holding" would leave both sets waiting forever, which would make Goal 2 false.

**The sink is a channel sender, and one writer thread owns the file.** The `log::Log` implementation filters by level, formats the record into a line, and hands it to a bounded `std::sync::mpsc` channel with `try_send`; it never opens, locks or writes a file. A single writer thread started at init owns the `File`, does the rotation, and writes each line with a timestamp from `time`. The reason is a hard rule, not a preference: Rune requires no file I/O in the frame loop, and the failure sites that most need logging — `drain` and `poll` in `src/main.rs` — run on the frame thread. Rune's own `AGENTS.md` states the rule for its repo, and typr keeps it because a blocking write in a frame is a visible stall. `env_logger` was rejected for two reasons: the path comes from `AppDirs`, which it does not know about, and it writes to stderr, which is invisible when the app is started from a launcher. `println!`/`eprintln!` were rejected because Rune's tooling owns stdout.

**Rotation happens once, on open.** If `typr.log` is larger than 1 MiB at startup it is renamed to `typr.log.1`, replacing any previous one. A per-write size check was considered and rejected: it costs a `metadata` call per line for a file that a session fills at most a few hundred times, and a log that only rotates between runs is still bounded because the app does not run for weeks.

**The level comes from `TYPR_LOG`, defaulting to `warn`.** Parsed with `LevelFilter::from_str` once, before the store task starts, so a `debug` run is one env var away and a shipped build is quiet. Rune keeps its own filtering; typr never sets a global level for crates it does not own.

**The chain walker lives with the sink, not with `StoreError`.** `fn describe(error: &dyn Error) -> String` joins `Display` with each `source()`. The status bar keeps calling `to_string()`, so the screen stays one line and the error type stays the app's vocabulary; only the log is verbose. Putting `source()` walking into `Display` was rejected: it would put a multi-line string into a widget that renders one.

**Startup records the facts a bug report needs, at `info`.** Database path, the `user_version` read before migration and after, how many migrations ran, and the log path itself. Without these a reader cannot tell whether a library was migrated, refused, or never opened.

**A command slower than 250 ms is logged at `warn` with its id and timing.** It is cheap (one `Instant` per command, not per frame) and it is what turns "the UI felt slow" into a number. Reusing Rune's `--trace` was rejected: that records frames, not commands, and does not exist in a normal run.

**`close` records the join result.** `let _ = worker.join()` becomes a match that logs at `error` when the thread ended in a panic, so quitting after a panic leaves a record even if the screen was already gone. The writer thread and its sender are process-wide statics, so the log is still reachable during `Drop for Handle`. The flush runs in `main` after `rune::run` returns, never inside `close`: Rune has no quit hook, `Drop for Handle` runs during teardown inside `rune::run`, and a flush placed there would run before this record is written.

## Risks / Trade-offs

- **A double panic aborts the process.** A panic inside the panic handler (or in a `Drop` during unwinding) calls `abort`. The handler is kept to: build strings, lock the events `Mutex` through the poison-tolerant `lock`, push events, and `try_send` log lines. It never calls `Db`, never touches SQLite, and never writes a file.
- **A full channel drops records.** The channel is bounded so a burst cannot grow memory without limit, and `try_send` never blocks the frame thread. A dropped record is counted, and the count is written as one line the next time a record does fit, so a flood is visible as "n records dropped" rather than as silence.
- **The flush order is load-bearing and invisible.** `Drop for Handle` logs the join result and the flush must come after it, in `main` after `rune::run` returns; moving the flush into `close` or a `Drop` would silently drop exactly the record that matters. One test pins the order so the mistake fails loudly. The wait uses a reply channel with `recv_timeout` because `std::thread::JoinHandle` has no timed join, and a timeout is logged rather than aborting the quit.
- **A rotated log loses history.** With one previous file, a second failure-heavy run replaces the first. The cap and the count are both constants in one place, so raising them is a one-line change.
- **`AssertUnwindSafe` hides a real invariant question.** The batch vector and the `Db` may be left part-applied when a panic lands mid-transaction. The transaction rollback in `Db::apply` is what keeps the database consistent, and the design leans on it: the boundary answers ids and stops the loop, it never retries the batch.
- **Panics outside the worker are still unrecorded.** A panic in a view or the frame loop is caught by Rune's own handling, which this change does not touch. Installing `std::panic::set_hook` was considered and rejected for now: it would have to coexist with Rune's hook, and the store task is where a long-running failure actually lives.
