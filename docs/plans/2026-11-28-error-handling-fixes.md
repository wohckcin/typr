# Error Handling Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the findings of the 2026-11-27 error-handling review. A refused command must no longer mark the library as stopped. Refusals and file errors must reach the person, and three error paths that are unsafe or lossy must be closed.

**Architecture:** The core of the fix is one classifier, `StoreError::is_fatal()` in `typr-core`. `StoreBridge` in `typr` uses it so that only fatal errors set `stopped`. Every other error is shown to the person by the callback that asked for the work: the editor's `save_error` and `attach_error`, the study `refused` line, and one new sidebar line, `Sidebar::refuse`. The sidebar's "Library closed" footer and `Sidebar::note` are not touched, and the open library gets no status line. The `typr-core` changes are local: the attachment write switches to `create_new`, a few causes are kept instead of discarded, commands are routed through an exhaustive `Domain` match, and values from the database are decoded strictly. A workspace clippy lints block stops the discarding patterns from coming back.

**Tech Stack:** Rust 2024, `thiserror` 2, `rusqlite`, `zip`, NSH 0.7.1 (`nsh_testing::Headless` for behaviour tests).

**Spec:** No separate spec. The findings come from the review in this session, and each task below quotes the finding it fixes along with its file and line.

## Global Constraints

- Work on branch `fix/error-handling`, not `main`. Commit after each task. Don't push or open a PR unless asked.
- Commits use Conventional Commits: `type(scope): summary`, lowercase, at most 72 characters, and a body that says why.
- After each task, `cargo clippy --workspace --all-targets` reports no warnings and `cargo test --workspace` passes.
- No `unwrap`/`expect` in non-test code, and no new `unsafe` (the existing `set_var` in tests excepted).
- Every user-facing error line comes from `crate::logging::describe(&error)`, so the person sees the whole cause chain once. Never use `error.to_string()`, which drops the source.
- Don't set `TYPR_DATA_DIR` in the shell environment. Headless tests set it inside the `ENV` mutex, as in `crates/typr/tests/shell.rs:143-160`.
- Copy: sentence case, and labels have no trailing period.

## Review Focus

1. **A refusal must not stop the library.** After a duplicate deck name, Diagnostics must not show "The library work stopped…". This is the verified red case. (Task 1: `a_refused_command_does_not_stop_the_library`.)
2. **A real stop must still show.** `WorkerPanicked` and `Closed` must still set the banner. (Task 1: `only_a_stopped_worker_is_fatal`.)
3. **Cancelling a file dialog is not an error.** `Ok(None)` from `pick_paths` must stay silent. Only `Err` shows a line. (Task 3: `a_cancelled_dialog_is_not_an_error`.)
4. **Two pictures with the same name copied at once.** Neither file may be overwritten. (Task 4: `concurrent_attachments_never_share_a_name`.)
5. **A library row holding a mode or kind that typr never writes.** It must be refused with `StoreError::Text`, not read as `Active`/`Text`. (Task 6: `an_unknown_mode_is_refused`.)

---

### Task 1: Only a stopped worker sets the "library stopped" banner

The finding: `store_bridge.rs:216` (submit `Err`) and `:327-333` (`drain_events` `Err`) call `stopped.set(true)` for every error, and nothing ever clears it. A duplicate deck name therefore tells the person that the library stopped. This was verified red in a headless probe.

**Files:**
- Modify: `crates/typr-core/src/error.rs` (add `is_fatal`)
- Modify: `crates/typr/src/shell/store_bridge.rs:215-217`, `:327-333`
- Test: `crates/typr-core/src/error.rs` (unit), `crates/typr/tests/shell.rs`

**Interfaces:**
- Produces: `impl StoreError { pub fn is_fatal(&self) -> bool }`. It returns true only for `WorkerPanicked { .. }` and `Closed`. `Sqlite` and `Io` are **not** fatal: a constraint failure or a full disk refuses one command, and the worker keeps running.

- [ ] **Step 1: Write the failing tests.** Unit test in `error.rs`:

```rust
#[test]
fn only_a_stopped_worker_is_fatal() {
    assert!(StoreError::Closed.is_fatal());
    assert!(StoreError::WorkerPanicked { message: "x".into() }.is_fatal());
    assert!(!StoreError::DuplicateName { what: "deck", name: "Spanish".into() }.is_fatal());
    assert!(!StoreError::BlankName { what: "deck" }.is_fatal());
    assert!(!StoreError::Io(std::io::Error::other("disk full")).is_fatal());
}
```

  Headless test in `crates/typr/tests/shell.rs`. Use `library_on_disk("Spanish")` and the `ENV` lock:

```rust
#[test]
fn a_refused_command_does_not_stop_the_library() -> Result<(), String> {
    // ENV lock + TYPR_DATA_DIR = library_on_disk("Spanish"), as in a_tile_opens_the_card…
    // wait_for("New Deck"); press("New Deck"); type_text("text:Deck name", "Spanish"); press_key("enter");
    // open Settings, press "Open diagnostics"
    let stopped = app.screen().texts().iter().any(|t| t.starts_with("The library work stopped"));
    assert!(!stopped, "a duplicate name is a refusal, not a stoppage");
    Ok(())
}
```

  Use `texts().iter().any(starts_with)`, not `shows()`. `shows()` needs an exact full-text match.
- [ ] **Step 2: Run them.** `cargo test -p typr-core only_a_stopped_worker_is_fatal` should fail to compile (no `is_fatal`). `cargo test -p typr --test shell a_refused_command` should FAIL on the assert.
- [ ] **Step 3: Implement.** Add `is_fatal` as a `matches!`. In both bridge `Err` arms, replace `stopped.set(true)` with `if error.is_fatal() { stopped.set(true) }`. Keep the `log::error!`, but log a non-fatal error at `warn` with the text "store refused a command: …".
- [ ] **Step 4: Run.** Both tests pass, and `cargo test -p typr --test diagnostics` still passes.
- [ ] **Step 5: Commit.** `fix(store): mark the library stopped only when the worker is gone`

### Task 2: Show refusals where the person asked

The finding: these callbacks drop `Err`:
- sidebar CreateDeck (`shell/mod.rs:408`)
- new card (`shell/editor.rs:104`)
- opening a draft (`shell/editor.rs:70-73`, `_ => None`)
- the study queue (`shell/study.rs:102-113`: it logs, finds no entries, and returns silently)

**Files:**
- Modify: `crates/typr/src/shell/mod.rs:401-414`, `crates/typr/src/shell/editor.rs:60-110`, `crates/typr/src/shell/study.rs:85-120`
- Modify: `crates/typr/src/screens/sidebar.rs` (add `refusal` beside `notice`, and one line that renders it)
- Test: `crates/typr/tests/shell.rs`

**Interfaces:**
- Consumes: `crate::logging::describe(&StoreError) -> String`.
- Produces: `Sidebar::refuse(&self, why: impl Into<SharedString>)`, which sets a new `refusal: Signal<Option<SharedString>>`. `Sidebar::select` clears it. It renders as one `text_sm` line in `tokens::destructive` above the utility rows, and only while it is `Some`. `notice`, `note` and `library_state` (`sidebar.rs:418-425`) stay as they are: the library's open state is never shown.

- [ ] **Step 1: Write the failing test** in `shell.rs`, using the same setup as Task 1:

```rust
#[test]
fn a_duplicate_deck_name_is_explained_in_the_sidebar() -> Result<(), String> {
    // … create "Spanish" again as in Task 1, then:
    app.wait_for(r#"a deck under that parent called "Spanish" already exists"#)?;
    Ok(())
}
```

  Copy the exact text from the `DuplicateName` display. The log in the probe showed `a deck under that parent called "Spanish" already exists`. If `describe` adds a prefix, match with `starts_with`/`contains` over `texts()` instead.
- [ ] **Step 2: Run it.** It should FAIL (times out).
- [ ] **Step 3: Implement.**
  - Add `refusal` and `refuse` to `Sidebar`, render the line, and clear it in `select`.
  - `sidebar_picker`: on `Err(error)`, call `state.refuse(describe(&error))`.
  - `new_card`: on `Err(error)`, call `sidebar.refuse(describe(&error))`. `sidebar` is already a parameter, so move it into the closure.
  - `open`: on `Err(error)`, `log::warn!` it and `save_error.set(Some(describe(&error).into()))`, then still open the editor with no draft, since the saved card is intact. `Ok(_)` other than `Draft` stays `None`.
  - Study queue: on `Err`, keep the log, and call `refused.set(true)` plus `sidebar.refuse(…)`: "The study queue could not be loaded: {describe}". The study screen isn't open yet, so the sidebar is where the person looks. Pass `Sidebar` into the queue starter the same way `new_card` takes it.
- [ ] **Step 4: Run** `cargo test -p typr --test shell --test sidebar --test study`. Expected: PASS.
- [ ] **Step 5: Commit.** `fix(shell): show refused commands instead of dropping them`

### Task 3: Editor and settings file errors reach the screen

The finding: these file errors are lost:
- `screens/editor/mod.rs:376` and `:402` drop dialog errors with `answer.ok().flatten()`.
- `:382` drops the diagram's attach error with `if let Ok`.
- `:429` shows `error.to_string()`, an `io::Error` with no path.
- `settings.rs:194` throws away the result of `reveal_path`.

**Files:**
- Modify: `crates/typr/src/screens/editor/mod.rs:44-46`, `:370-430`
- Modify: `crates/typr/src/screens/settings.rs:190-195`
- Test: `crates/typr/src/screens/editor/mod.rs` (unit)

**Interfaces:**
- Consumes: `attach(root, source) -> Result<String, StoreError>` from Task 4. **Run Task 4 before this task.**
- Produces: `fn first_picked<E: std::fmt::Display>(answer: Result<Option<Vec<PathBuf>>, E>) -> Result<Option<PathBuf>, String>`, private in `editor/mod.rs`. `attach_image(source, root) -> Result<String, StoreError>`.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn a_cancelled_dialog_is_not_an_error() {
    assert_eq!(first_picked::<String>(Ok(None)), Ok(None));
    assert_eq!(first_picked::<String>(Ok(Some(vec![]))), Ok(None));
}
#[test]
fn a_failed_dialog_says_why() {
    let err = first_picked(Err("portal unavailable")).unwrap_err();
    assert!(err.contains("portal unavailable"));
}
```
- [ ] **Step 2: Run.** `cargo test -p typr --lib first_picked` should fail to compile.
- [ ] **Step 3: Implement.**
  - Write `first_picked`. Its error copy is `"The file picker failed: {error}"`.
  - Both buttons use it. On `Err(why)`, call `attach_error.set(Some(why.into()))`.
  - The diagram button handles `attach_image`'s `Err` the same way as the attach button. Both use `describe(&error)`, which now names the path through Task 4.
  - In settings, on `Err(error)` from `reveal_path`, `log::warn!` it. Showing a line isn't worth a new signal, because the path is already printed beside the button.
- [ ] **Step 4: Run** `cargo test -p typr --lib` and `cargo test -p typr --test editor`. Expected: PASS.
- [ ] **Step 5: Commit.** `fix(editor): report picker and attach failures with their path`

### Task 4: Transfer writes can't overwrite, and they keep their cause

The findings:
- `transfer/target.rs:57-73` checks `exists()` and then calls `fs::write`. That is a TOCTOU race that can overwrite an existing picture.
- `attach` returns a bare `io::Error` with no path.
- `mochi.rs:594` labels every `by_name` failure "the archive holds no picture", including I/O and corruption errors.

**Files:**
- Modify: `crates/typr-core/src/transfer/target.rs:30-74`
- Modify: `crates/typr-core/src/transfer/mochi.rs:593-599` and the `attach_bytes` call site that maps its error
- Test: `crates/typr-core/src/transfer/target.rs` (unit), `crates/typr-core/tests/transfer.rs`

**Interfaces:**
- Produces:
  - `pub fn attach(root: &Path, source: &Path) -> Result<String, StoreError>`. A failed read gives `Read { path: source }` and a failed write gives `Write { path: root.join(name) }`.
  - `attach_bytes` keeps `std::io::Result<String>`, because mochi already wraps it.
  - `write_attachment` uses `OpenOptions::new().write(true).create_new(true)` and moves to the next `{stem}-{n}{ext}` on `ErrorKind::AlreadyExists`, the same way `copy_new` in `transfer/mod.rs:105` does.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn concurrent_attachments_never_share_a_name() {
    let root = tempdir();
    let names: Vec<String> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..16u8).map(|i| { let r = &root;
            s.spawn(move || attach_bytes(r, "a.png", &[i]).unwrap()) }).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let unique: std::collections::HashSet<_> = names.iter().collect();
    assert_eq!(unique.len(), 16);
    for (i, name) in names.iter().enumerate() {
        assert_eq!(std::fs::read(root.join(name)).unwrap(), vec![i as u8]);
    }
}
#[test]
fn a_missing_source_is_named_in_the_error() {
    let err = attach(&tempdir(), Path::new("/nope/cat.png")).unwrap_err();
    assert!(err.to_string().contains("/nope/cat.png"), "{err}");
}
```

  Use whatever temp-dir helper the module's tests already use. The race test can pass by luck before the fix. Run it with `--test-threads=1` in a loop of 20, and record in the commit body how often it failed before the fix.

  For mochi, add a test in `tests/transfer.rs`: an archive whose picture entry is stored corrupted (a bad CRC or truncated data) must give an error that does **not** say "holds no picture". The fixture helpers are in `crates/typr-core/tests/fixture.rs`.
- [ ] **Step 2: Run.** `cargo test -p typr-core attach`. The path test fails, and the race test fails at least once across the loop.
- [ ] **Step 3: Implement.**
  - Rewrite `write_attachment` around `create_new`.
  - `attach` wraps its errors with the path variants. Mochi's existing `attach_bytes` mapping stays.
  - In `picture`, `zip::result::ZipError::FileNotFound` keeps the current `Archive` reason. Any other `ZipError` becomes `Archive { reason: format!("picture {entry:?} could not be read: {error}") }`, so the real cause is kept.
- [ ] **Step 4: Run** `cargo test -p typr-core`. Expected: PASS. Then run `cargo test -p typr --test editor` for the `attach_image` callers.
- [ ] **Step 5: Commit.** `fix(transfer): create attachments atomically and keep their cause`

### Task 5: Every command is routed by an exhaustive match

The finding: `store/db/mod.rs:22-77` (`DISPATCHERS`) and `command.rs:244-262` (`NAMERS`) find a command's handler at run time and end in `unreachable!`. A new `Command` variant compiles but panics the worker.

**Files:**
- Modify: `crates/typr-core/src/command.rs`, `crates/typr-core/src/store/db/mod.rs:22-77`
- Test: `crates/typr-core/src/command.rs` (unit)

**Interfaces:**
- Produces:
  - `pub(crate) enum Domain { Deck, Diagram, Card, CardState, Review, View, Transfer }`.
  - `impl Command { pub(crate) fn domain(&self) -> Domain }`: one `match` that lists every variant, with **no wildcard arm**.
  - `Db::dispatch` and `Command::name` each `match self.domain()` and call that domain's existing function.
  - The domain functions keep their signatures. A `None` from them now means the command was put in the wrong domain. `dispatch` turns that into `unreachable!("{} is routed to {:?}", command.name(), domain)`, which can only fire if `domain()` and a handler disagree. The test below pins that.

- [ ] **Step 1: Write the failing test:**

```rust
#[test]
fn every_domain_names_what_it_routes() {
    // one representative command per Domain; each name() must not panic and domain() must match
    for (command, domain) in samples() { assert_eq!(command.domain(), domain, "{}", command.name()); }
}
```

  `samples()` builds one `Command` per domain from the existing test fixtures.
- [ ] **Step 2: Run.** It should fail to compile (no `Domain`).
- [ ] **Step 3: Implement** `Domain` and `domain()`, then replace both arrays.
- [ ] **Step 4: Run** `cargo test -p typr-core`. Expected: PASS. The existing store tests exercise every handler.
- [ ] **Step 5: Commit.** `refactor(store): route commands by an exhaustive match`

### Task 6: Strict decoding, typed open errors, and the small items

The findings:
- `store/db/core.rs:138-144` `mode_parse` and `model.rs:183-188` `CardKind::parse` read unknown text as a default. `review.rs:557-601` refuses unknown text instead.
- `error.rs:32-36`: the "deck name already uses" doc comment sits on `Missing`.
- `paths.rs:16-57`: `OpenError` is written by hand, and its `Io` display names no path.
- `logging/mod.rs:134` ignores `set_boxed_logger` failing.

**Files:**
- Modify: `crates/typr-core/src/store/db/core.rs:138-144`, `crates/typr-core/src/model.rs:182-188`, `crates/typr-core/src/store/db/snapshot.rs:71,231`
- Modify: `crates/typr-core/src/error.rs:32-36`
- Modify: `crates/typr/src/paths.rs:15-57` and its `?` sites
- Modify: `crates/typr/src/logging/mod.rs:134`
- Test: `crates/typr-core/src/store/db/core.rs` (unit), `crates/typr/src/paths.rs` (unit)

**Interfaces:**
- Produces:
  - `fn mode_parse(text: &str) -> Result<CardMode, StoreError>` and `CardKind::parse(value: &str) -> Result<CardKind, StoreError>`. Unknown text gives `StoreError::Text { column: "mode" | "kind", value }`. Callers in `snapshot.rs` use `?`, which works because `StoreError: From<rusqlite::Error>`.
  - `OpenError` as `#[derive(Debug, thiserror::Error)]` with `NoDataDir`, `Io { path: PathBuf, #[source] source: io::Error }` (display `"could not prepare the library folder {path}"`), and `#[error(transparent)] Store(#[from] StoreError)`. The `From<io::Error>` impl goes away, and each `?` on an io call gets `.map_err(|source| OpenError::Io { path: dir.clone(), source })`.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn an_unknown_mode_is_refused() {
    assert!(matches!(mode_parse("frozen"), Err(StoreError::Text { column: "mode", .. })));
    assert_eq!(mode_parse("trash").unwrap(), CardMode::Trash);
}
#[test]
fn an_unknown_kind_is_refused() {
    assert!(matches!(CardKind::parse("video"), Err(StoreError::Text { column: "kind", .. })));
}
// paths.rs
#[test]
fn a_folder_error_names_the_folder() {
    let e = OpenError::Io { path: "/x/typr".into(), source: std::io::Error::other("denied") };
    assert!(e.to_string().contains("/x/typr"));
}
```
- [ ] **Step 2: Run.** All three fail to compile.
- [ ] **Step 3: Implement.**
  - Make both parsers return `Result` and update the two `snapshot.rs` callers.
  - Move the misplaced doc comment from `Missing` to `DuplicateName`, and give `Missing` "An id that names no row of that kind."
  - Convert `OpenError` to `thiserror`.
  - In `logging`, `if let Err(error) = log::set_boxed_logger(..)` should `log::warn!("a logger was already installed, so typr's log file stays empty: {error}")`. That message goes to the logger already installed.
- [ ] **Step 4: Run** `cargo test --workspace`. Expected: PASS. Then run `cargo test -p typr --test startup` for `OpenError`.
- [ ] **Step 5: Commit** each concern on its own:
  - `fix(store): refuse card modes and kinds typr never wrote`
  - `fix(paths): name the folder that could not be prepared`
  - `docs(error): put each variant's comment on its own variant` (fold the logger change into this commit or the paths one)

### Task 7: Workspace lints keep the patterns out

**Files:**
- Modify: `Cargo.toml` (add `[workspace.lints.clippy]`), `crates/typr-core/Cargo.toml`, `crates/typr/Cargo.toml` (add `[lints] workspace = true`)
- Modify: the remaining `map_err(|_| …)` sites

**Interfaces:** none.

- [ ] **Step 1: Add the lints:**

```toml
[workspace.lints.clippy]
map_err_ignore = "warn"
let_underscore_must_use = "warn"
```
- [ ] **Step 2: Run** `cargo clippy --workspace --all-targets`. Expected: warnings at the sites below, and no others once Tasks 3-6 are in.
- [ ] **Step 3: Resolve each one.** If the discarded error carries nothing the message lacks, annotate it with `#[expect(clippy::map_err_ignore, reason = "…")]`. Otherwise keep the cause.

  | Site | Action |
  | --- | --- |
  | `store/task/mod.rs:131` | Expect. A `SendError` only means the worker is gone, which is `Closed`. |
  | `store/schema.rs:408` | Expect. The recorded version is in `Corrupt`. |
  | `store/settings.rs:177` | Expect. The value is in `Setting`. |
  | `store/settings.rs:289` | Expect. This is UI copy for the cap field. |
  | `fsrs/mod.rs:177` | Expect. The text is in `Setting`. |
  | `transfer/csv.rs:154` | Expect. The cell is quoted in the reason. |
  | `store/db/review.rs:563` | Expect. A `fmt::Error` writing to a `String` cannot happen. |
- [ ] **Step 4: Run** `cargo clippy --workspace --all-targets` (no warnings) and `cargo test --workspace` (PASS).
- [ ] **Step 5: Commit.** `build(lints): warn on discarded errors across the workspace`

---

**Left out on purpose:**
- `Archive`/`Malformed` keep `reason: String`. Every caller already formats a reason that names the bad part, and a typed reason enum would add a type for each message without changing what the person reads.
- `check_new_card_cap -> Result<u32, String>` stays, because it is UI validation copy.
- `ViewMode::parse` defaulting to `Grid` stays. It is documented, and the value is a preference, not data.
