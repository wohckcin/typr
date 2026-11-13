# Design

## Context

`typr` is an empty Rust repo. `Cargo.toml` has no dependencies and `src/main.rs` prints hello, so this design has no legacy to accommodate; its constraints come entirely from Rune and from the data model.

Constraints that shape everything below:

- **Rune is consumed, not modified.** `typr` is a separate repository depending on Rune's `dev` branch at a pinned revision. Rune's `AGENTS.md` binds work inside Rune's repository, not in `typr`.
- **Rune pins Rust 1.99.1** (`rust-toolchain.toml`). `edition = "2024"` is already set here.
- **`[patch.crates-io]` does not propagate** from Rune to a dependent workspace, so any crate Rune patches must be patched identically in `typr` at the same revision. Today that is `winit`, `parley_engine`, `wgpu-hal`, `accesskit_windows`.
- **No storage layer in Rune.** It offers `Prefs` (a JSON file) and `AppDirs`; neither is a database.
- **Headless tooling is the only way to see the UI without clicking.** Debug builds always accept `--snap`, `--tree`, `--script`/`--video`; release builds need the `tooling` feature.
- **Headless runs call the app's `setup` with a no-op waker** (`crates/rune/src/tooling.rs:149-152`) and `AppDirs` honours `RUNE_DIRS_ROOT` (`crates/rune/src/dirs.rs:22,35`), so a scene or a test must never open the user's real database.
- Requirements live in `specs/` (card-library, card-editor, review-scheduling, study-session); this document is about how they are met.

## Goals / Non-Goals

**Goals:**

- A data model in which reverse review, cloze groups and diagram masks are *new rows*, never schema migrations.
- A review history complete enough that FSRS can be computed from it without having recorded anything extra at the time.
- A UI that never performs disk I/O on the frame thread, stays responsive while a large library is written, and survives a quit without losing a committed grade.
- Every screen reachable and assertable from a script, so behaviour can be verified without a human at the window.
- A clock the tests own, so "due tomorrow" and "the next study day" are assertable instead of slept through.

**Non-Goals:**

- Sync, multi-device conflict handling, anything involving a server.
- Rendering math (LaTeX) — Rune has no math renderer and this belongs upstream.
- Editing multiple cards at once, drag-and-drop reordering, custom views.

## Decisions

### Depend on Rune by pinned git revision, repeating its patches

```toml
[dependencies]
rune     = { git = "https://github.com/anomalyco/rune", rev = "<rev>", features = ["native-file-dialog"] }
rune-kit = { git = "https://github.com/anomalyco/rune", rev = "<rev>", features = ["markdown"] }

[patch.crates-io]
winit             = { git = "https://github.com/anomalyco/rune", rev = "<rev>" }
parley_engine     = { git = "https://github.com/anomalyco/rune", rev = "<rev>" }
wgpu-hal          = { git = "https://github.com/anomalyco/rune", rev = "<rev>" }
accesskit_windows = { git = "https://github.com/anomalyco/rune", rev = "<rev>" }
```

`dev` is fast-forward-only, so a revision is a stable target and upgrades are deliberate. Alternatives: a vendored copy of Rune (control, but every upstream fix becomes a manual merge); developing `typr` as a crate inside the Rune workspace (faster iteration, but couples an application's release to a framework monorepo). Chosen because an app that ships should not be able to break because a framework moved.

### One prompt per review unit; `key` is the extension point

The scheduler stores state per `(card, key)`, where `key` is `fwd` today. Later work adds rows rather than columns:

| Feature | Key added | Prompts per card |
|---|---|---|
| v1 sides | `fwd` | 1 |
| reverse direction | `rev` | +1 |
| cloze groups | `cloze:<n>` | +1 per group |
| diagram masks | `mask:<id>` | +1 per mask group |

Sides never multiply prompts: Mochi treats a multi-sided card as one review that reveals sides in sequence, and so does this design. That keeps "how many things do I owe today" identical to what the user wrote.

### History is append-only and carries its phase

```
review_log(card_id, key, phase, at, grade, interval_ms, due_at, duration_ms)
```

`phase` is `learn` / `review` / `relearn`. The alternative — writing nothing when a new card is graded `forgot`, which is what the UI does to the schedule — was rejected because FSRS seeds a card's initial stability from its *first* rating (`w0..w3` per grade; binary maps forgot→Again, remembered→Good). A card forgotten four times before it stuck would otherwise be indistinguishable from one remembered first try, and that signal cannot be reconstructed later.

The consequence is that the log is not a review count. Queries that mean "reviews" filter `phase <> 'learn'`, and that filter lives inside the store:

```
Store::due(frame)        -> prompt ids       -- a query on the published snapshot, not a round trip
Store::stats(from, to)   -> counts           -- phase <> 'learn'
Store::history(card)     -> all rows         -- nothing filtered; the user asked for history
```

No view assembles these queries itself, so no view can accidentally count learning cycles as study. Resetting a prompt deletes every row including `learn`, because a reset deliberately returns the card to its pre-seed state.

### `prompt` is a derived cache, rebuildable from the log

```
deck(id, parent_id, name, archived)
card(id, deck_id, markdown, archived, trashed_at, search_text, created_at, updated_at)
prompt(card_id, key, phase, due_at, interval_ms, reps, lapses)   -- derived
tag(card_id, tag)
review_log(card_id, key, phase, at, grade, interval_ms, due_at, duration_ms)
editor_draft(card_id, text, updated_at)
settings(key, value)
```

`search_text` is the card's markdown with tags appended and characters folded for case and diacritics at write time, because SQLite's `LIKE` folds case for ASCII only. `trashed_at` is null for a live card; the trash view is `trashed_at IS NOT NULL` and emptying the trash deletes those rows with their log.

`review_log` is the truth; `prompt` is what the due query needs to be fast. The invariant is testable: folding the log reproduces every `prompt` row. That is why "recomputed from history" appears as a spec scenario — it is the check that keeps the cache honest, and it is what makes an FSRS migration a fold over existing rows rather than a data conversion. The prompt-key extension point is proven at the store level rather than in a v1 UI scenario: folding a log that contains `rev`, `cloze:1` and `mask:a1` rows reproduces one `prompt` row per key, which is the whole claim the table above makes.

### Scheduling behind a trait; fixed multipliers now

```
trait Scheduler { fn grade(&self, s: &PromptState, g: Grade, at: Timestamp) -> Outcome; }
```

v1 implementation follows Mochi's two-step entry: `remembered` in `learn` → due in 10 minutes (still today), `remembered` in `review` → 1 day for the first success, then ×2.5 capped at 365 days; `forgot` in `review` or `relearn` → 10-minute relearning step, and the success after it starts again from 1 day. Mochi's own constants are undocumented, so matching its *feel* (binary grades, adaptive spacing) is the goal, not its numbers. The `fsrs` crate exists on crates.io and is being maintained; it slots in behind this trait, reading `review_log` and writing both `prompt` and future rows. Its exact release is not pinned here — the trait is what v1 owes it.

### The study day is a local calendar day starting at 04:00

The due queue selects `due_at <= now`, which is the honest reading of a schedule; the *new-card cap* and every "today" label use a study day instead. A study day starts at 04:00 local time, so studying past midnight does not hand out a second batch of new cards. Working in whole days also keeps the queue order semantic: prompts are grouped by how many whole study days overdue they are, shuffled inside a group, and ordered between groups by urgency, so a reshuffle can reorder a group without breaking the most-overdue-first rule.

The clock is a trait with a real implementation and a test double. Nothing in the domain layer calls `SystemTime::now()` directly, which is what lets "the next day" and "ten minutes later" be assertions rather than sleeps, and what keeps a `--script` run reproducible.

### The new-card cap is one global number per study day

The limit is library-wide and per study day, not per deck: Mochi exposes a single "limit new cards per day" setting in Review Settings, and per-deck caps would make "how much can I learn today" depend on how many decks exist. Default 20, stored in `settings`. A card counts against a day when its earliest `review`-phase record falls on that day, so a card deferred by the limit stays new and is offered tomorrow. `settings` also holds the cap and nothing else in v1; it exists now because a cap in code is a cap the user cannot change.

### SQLite, one writer task, `Store` as the only door

`rusqlite` with the `bundled` feature, WAL enabled, file at `<AppDirs::data()>/typr.sqlite3`. `AppDirs::new(app_id)` returns `Option` (`crates/rune/src/dirs.rs:53`); an app id of `typr` is passed and a `None` is reported to the user as a startup failure rather than silently writing beside the executable.

```
+----------------+   Cmd { id, op }               +---------------------+
|  UI task       | ------------------------------> | store task          |
|  (frame loop)  |                                 | owns the Connection |
|                | <------------------------------ | applies + logs      |
|  never awaits  |   Event { id, result } + Waker  | publishes snapshot  |
+----------------+   on writes only                +---------------------+
```

Every command carries an id and gets exactly one event back, so "committed before the next card is shown" is a fact about the protocol rather than a hope: grading sends `Cmd::Grade`, and the session advances on that command's event, not on an optimistic local update. `AppSpec::poll` returns `true` only when an event arrived or a relearning step landed, and the snapshot is republished only when the command wrote (`Outcome::writes`), so a query costs no library re-read and idle frames cost nothing. Views read the snapshot the store task publishes, not the database.

Quitting is the one case where the frame thread blocks, and it does so without a hook: `AppSpec` carries no `on_quit` (that lives on the `App` builder, `crates/rune/src/app.rs:449`), a normal window close returns `Ok` from `App::run` and never calls `std::process::exit` (`crates/rune/src/tooling.rs:233-238`), so the state owning the handle is dropped and `Drop for Handle` closes it, which drains the queue and joins the store task. Everything before that point is asynchronous; this join is bounded by the flush and cannot grow with the size of the log.

Alternatives: `redb` (pure Rust, but no SQL and no FTS5, which search will want), JSON/JSONL files (readable, but every due query and statistic becomes a full scan, and a crash mid-rewrite costs cards), SQLite on the UI thread (simplest, and rejected because one bulk write freezes the window).

Search filters the published snapshot: every card carries its `search_text` folded once at write time (NFD, combining marks dropped, lowercased), and the query is folded the same way, so matching is a substring test in memory with no round trip and no `LIKE` metacharacters to escape. FTS5 is in the same file if a library ever outgrows the snapshot. Long lists use a growth stack rather than building every row: `col().growth(GrowthSpec::default()).virtual_rows(count, row_height, key, build)` (`crates/rune/src/el/virtual_rows.rs:80-88`; `growth` is documented as required alongside the list, `crates/rune/src/el/growth.rs:7-9`), which mounts only the viewport's rows and builds them inside `build`. `rune_kit::List` builds all of its rows, so it is not used for the card list.

### One explicit state machine per screen

Not a Rune requirement — a convention this project adopts because a `enum Screen` plus `fn step(state, event) -> Effects` is drivable from a script, assertable in a test without a window, and impossible to leave in an unreachable state.

```
          +----------+  open card   +----------+
          | Library  | -----------> | Editor   |
          +----+-----+ <----------- +----------+
               | start queue    save/cancel
               v
          +----------+  queue empty  +----------+
          | Session  | ------------> | Summary  |
          +----------+ <------------ +----------+
             |  quit mid-queue   "back"
             +--> Library (progress already committed)
```

### Sides are split before markdown is parsed

The source is split on a line whose trimmed content is exactly `---` (outside fenced code), then each side is parsed as its own markdown document. Splitting first means the separator can never be reinterpreted as a setext heading underline or a thematic break, and it keeps `***`/`___` available to the user. The renderer caches parsed sides with the free function `rune_kit::markdown::cache::parse_cached` (`crates/rune-kit/src/markdown/cache.rs:43`), keyed by card revision, so typing in one card does not re-parse the rest.

Images are resolved by an app-supplied resolver: `rune-kit` asks the application for image bytes (`crates/rune-kit/src/markdown/image.rs:9-10`) and this app answers from `<AppDirs::data()>/attachments/`, refusing any path that escapes that directory. Attaching a file copies it there with a generated name and inserts the reference, which keeps a card's text portable inside the library.

### Every screen gets scenes, because there is no designer

Each view registers `AppSpec::scenes` covering its states (empty library, card list, new-card queue, due queue, revealed side, summary), and CI builds debug and runs `--snap` per scene. Screenshot diffs are the regression test for layout regressions that unit tests cannot see.

Scenes must not touch the user's data. Because headless runs hand `setup` a no-op waker and `poll` is never driven, a scene builds its state directly from a fixture (`init` ignores the waker) and CI sets `RUNE_DIRS_ROOT` to a temporary directory for any scene that opens a store at all. A test that needs a store, a clock or a "next day" uses the same seams: `RUNE_DIRS_ROOT` for paths, an injected clock for time, and a fixture library for content.

## Risks / Trade-offs

- **Rune's `dev` branch moves under us** → pin the revision; upgrade on purpose, reading its changelog; the four `[patch.crates-io]` entries must be bumped with it or the build breaks confusingly.
- **Framework gaps bound v2 features** (LaTeX, `[[refs]]`) → accepted. They are upstream commits plus a revision bump; the alternative was abandoning Rune's editor, markdown and accessibility work.
- **Derived `prompt` drifts from `review_log`** → the fold-equality invariant is asserted in tests and re-checked on startup in debug builds.
- **`--snap` is unavailable in release** → CI runs screenshots in debug; the `tooling` feature is enabled for release builds used in QA so the same flags work there.
- **A bulk edit touches many cards** → each command is a transaction and the store publishes a snapshot per batch, so the window stays live and the list shows the change as it lands instead of appearing hung.
- **A grade lost on quit** → `on_quit` flushes and joins the store; the session also advances only on the store's acknowledgement, so nothing is shown as saved before it is.
- **Single writer means slow grades on a huge log** → acceptable: a grade is one insert plus one update; the log is append-only and indexed by `(card_id, key, at)`.

## Migration Plan

The app has no users and no data, so there is no migration. The forward path is established now regardless: the database carries `PRAGMA user_version`, migrations run in order inside a transaction on open, and a newer database opened by an older build is refused with a message rather than silently downgraded. Both halves are tested before the first release — "a version 0 database is upgraded to the current version" and "a database one version ahead is refused and left unmodified". Rolling back means installing the previous build; it will refuse a database it does not understand.

## Open Questions

- Exact review-phase key bindings. The new-card queue is documented as `SPACE` = remembered, `F` = forgot, arrows to navigate; the due queue's reveal key is unverified. Deferred to implementation, resolved by matching the new-card bindings so one muscle memory covers both.
