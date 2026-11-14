# Tasks

## 1. Logging foundation

- [ ] 1.1 Add `log = "0.4"` as a direct dependency and a `logging` module whose `log::Log` implementation filters by `LevelFilter`, formats one line per record, and hands it to a bounded channel with `try_send`; no file is opened on that thread.

  **Verification**: a unit test writes a record from the test thread, drains the channel, and asserts the formatted line; a second test asserts nothing is sent below the filter.

- [ ] 1.2 Add the writer thread that owns the log file: it creates the logs folder, writes each line with a timestamp from `time`, and counts records dropped when the channel was full, reporting the count on the next successful write.

  **Verification**: a test sends a record through and reads it back from the file; a test that shrinks the channel to 1 and sends 3 records asserts two are dropped and the next write reports the count.

- [ ] 1.3 Rotate once on open: when the file exceeds the size limit, rename it to `typr.log.1` replacing any previous copy, then start an empty file.

  **Verification**: a test pre-creates a file larger than the limit with known content, starts logging, and asserts the rotated copy holds the old content and the new file holds only new records.

- [ ] 1.4 Install the logger from the resolved `AppDirs` (path `<logs()>/typr.log`), reading `TYPR_LOG` for the level with `warn` as the default when it is unset or unparseable.

  **Verification**: a test sets `RUNE_DIRS_ROOT` and `TYPR_LOG=info`, installs the logger, logs, and finds the file under that root; a second test with `TYPR_LOG` unset asserts an informational record is not written.

- [ ] 1.5 Flush at quit: send a flush request on the quit path and join the writer thread with a short timeout, so a record written immediately before exit is on disk.

  **Verification**: a test logs, calls the shutdown routine, then reads the file and finds the record; a second test asserts shutdown twice is harmless.

- [ ] 1.6 Log a failure with its whole cause chain: `fn describe(error: &dyn Error) -> String` joining `Display` with each `source()`.

  **Verification**: a test builds a `StoreError` from an `io::Error` with a known permission message and asserts the description contains both typr's message and the operating-system message.

## 2. Panic boundary and answers

- [ ] 2.1 Copy the in-flight message id into a local before `apply` is called, and wrap the apply-and-publish pair in `catch_unwind`, documenting why `AssertUnwindSafe` is sound.

  **Verification**: a test sends one command that panics and asserts the answer for that id arrives.

- [ ] 2.2 After a caught panic, answer every id still queued: drain the receiver with `try_recv`, answer each with `Closed`, then end the loop and drop the receiver.

  **Verification**: a test queues three commands where the first panics, and asserts all three ids receive an answer, the last two with `Closed`.

- [ ] 2.3 Push exactly one unprompted error event per failure, carrying the panic message read through `downcast_ref` for `&str` and `String`, and the payload's type name otherwise.

  **Verification**: a test asserts the events hold one entry with id 0 whose message contains the panic text; a second test panics with a non-string payload and asserts the type name appears.

- [ ] 2.4 Record the worker's join result in `Handle::close` at error level instead of discarding it, and log it once per handle.

  **Verification**: a test that panics the worker, drops the handle, and asserts the log holds one abnormal-end record.

- [ ] 2.5 Keep later sends answering `Closed` after the boundary fires, and prove the pending-grade path clears.

  **Verification**: a test sends a command after a panic and asserts `StoreError::Closed`; a test drives a grade through the app state while the worker panics and asserts the card becomes answerable again.

## 3. Wire it into the application

- [ ] 3.1 Log every failure the interface already shows, using the chain-aware description while the failure line keeps the single-line message.

  **Verification**: a test refuses a write path, then asserts the failure line is one line and the log holds the cause chain.

- [ ] 3.2 Record startup facts at information level: database path, the schema version read before migration, the version written after, and how many migrations ran.

  **Verification**: a test opens a library from an older version at information level and asserts both versions and the count appear; a test refuses a newer version and asserts the refusal is recorded.

- [ ] 3.3 Log any command slower than the threshold at warning level with its id and duration, and nothing below it.

  **Verification**: a test with a threshold of zero asserts a record per command; a test with a threshold above the measured cost asserts no such record.

- [ ] 3.4 Give the failure line a headless scene so a stoppage is reviewable without a GUI session.

  **Verification**: `RUNE_DIRS_ROOT` set and `--scene` on the failure scene write a non-blank PNG, and the scene list includes it.

## 4. Verification and documentation

- [ ] 4.1 Cover every scenario in `specs/diagnostics/spec.md` with a test at the level where it is observable, and reverse each guard once in a scratch worktree with its own `CARGO_TARGET_DIR` to confirm the test fails without the change.

  **Verification**: the list of scenario-to-test pairs is complete, and each reversed guard has a recorded failing run.

- [ ] 4.2 Update `README.md` with the log path, the `TYPR_LOG` variable, the size limit and the rotated name, and record in `design.md` anything the implementation settled differently.

  **Verification**: the README names the path and variable, and `openspec validate typr-observability --strict` is valid.
