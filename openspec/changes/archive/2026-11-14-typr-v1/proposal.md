# Proposal

## Why

`typr` is an empty Rust repo (`Cargo.toml` with zero dependencies, a three-line `main.rs`) intended to become a local-first desktop flashcard app in the shape of Mochi: write markdown cards, review them on a spaced-repetition schedule, keep everything on disk. No code exists, so the first change must fix what "the app" means before any of it is written.

The hard part is not the framework — [Rune](https://github.com/anomalyco/rune) already gives a retained-mode widget layer, markdown rendering, a task runtime for background work and headless screenshots — it is that three independent mechanisms have to agree: a markdown editor with live preview, a database living off the UI thread, and a scheduler whose history is the future's training data. Getting the review model wrong is the expensive mistake, because it is the one thing that cannot be migrated cheaply later.

## What Changes

- Add a deck/card library: nested decks, a card list with search, archive and trash, tags.
- Add a markdown card editor: source and live preview side by side, `---` separating sides within one card.
- Add a local SQLite store behind a `Store` trait, owned by a background task; the UI never touches the disk in its frame loop.
- Add a scheduling model in which the scheduled unit is a **prompt**, not a card: v1 emits exactly one prompt per two-sided card (`key = 'fwd'`) and none for a one-sided note, so reverse direction, cloze groups and diagram masks arrive later as new keys rather than schema changes.
- Add an append-only `review_log` recording every grade with its `phase` (`learn` / `review` / `relearn`), so the history FSRS will need is complete from the first session.
- Add two study queues matching Mochi: **new cards** (learn a card, or cycle it) and **due today** (schedule-driven, overdue-first, shuffled), each drivable by keyboard alone.
- Add a fixed-multiplier scheduler behind a `Scheduler` trait; FSRS replaces it later without touching stored history.
- Out of scope for v1: cloze deletions, diagram cards and masks, templates and dynamic fields, reverse direction, FSRS, LaTeX, `[[references]]` and backlinks, `.mochi` import/export, sync, mobile.

## Capabilities

### New Capabilities

- `card-library`: decks, cards and tags as durable user data; listing, search, archiving and trashing; where the database lives and how it survives a restart.
- `card-editor`: authoring a card as markdown with `---`-separated sides; the source/preview editing surface and its save semantics.
- `review-scheduling`: prompts as the scheduled unit, the scheduler math, the append-only review history, and the due/new queries that the two queues are built from.
- `study-session`: the review loop itself — entering the new-card or due queue, revealing sides in order, grading, keyboard control, and where a session resumes after leaving it.

### Modified Capabilities

None — this is the project's first change and `openspec/specs/` is empty.

## Impact

- `Cargo.toml`: adds `rune` and `rune-kit` (features `markdown`, `native-file-dialog`) pinned to a Rune `dev` revision, plus the four `[patch.crates-io]` entries that revision requires; adds `rusqlite` (bundled) for the store, `thiserror` for the store's error type, and `time` with `local-offset` for the study day. Settings live in a `settings` table, so no serialisation crate is needed. The study day is defined in local time, and `time` can only read the local UTC offset on the main thread, so `src/day.rs` reads it once into a `LazyLock` before the window opens: a run keeps the offset it started with, so a daylight-saving change mid-run delays the rollover by an hour, and an environment where the offset cannot be read falls back to UTC, which shifts the 04:00 boundary. Both were accepted over adding platform plumbing for a background-thread lookup. `unicode-normalization` is there because SQLite's `LIKE` folds case but not accents, and search has to fold both.
- `src/main.rs`: grows from hello-world into an `AppSpec` whose state is a screen enum (Library / Editor / Session / Summary) and whose data layer speaks to a background task.
- New modules: store (connection task, migrations, queries), model (deck/card/prompt/log types), scheduler (trait + fixed multipliers), views (library, editor, session).
- Toolchain: Rune pins Rust `1.99.1`; `edition = "2024"` is already set.
- Runtime: the app writes its SQLite file under the platform data directory, so first launch creates `typr`'s app directory.
