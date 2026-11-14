# Tasks

## 1. Project and Rune baseline

- [x] 1.1 Add `rust-toolchain.toml` pinning channel `1.99.1` — the version Rune's own toolchain file specifies — and verify `rustc --version` reports it inside the repo
- [x] 1.2 Add `rune`, `rune-kit` and the four `[patch.crates-io]` entries pinned to one Rune `dev` revision in `Cargo.toml`, and verify `cargo build` succeeds on the pinned toolchain
- [x] 1.3 Replace the hello-world `main.rs` with an `AppSpec` that opens an empty window titled "typr", and verify it launches and closes cleanly
- [x] 1.4 Add a `Screen` enum with `Library`, `Editor`, `Session`, `Summary` and a `step` function returning effects, and verify a unit test drives it through each transition by event
- [x] 1.5 Add a `Clock` trait with a real implementation and a test double, and verify a unit test places a prompt ten minutes out and reads it back as due by advancing the double
- [x] 1.6 Write `README.md` with build prerequisites and the run command, and verify the documented command works on this machine. Linux prerequisites come from Rune's README: development packages `libxkbcommon-dev libwayland-dev` (build/link, `crates/rune/README.md:2455-2461`) and runtime `libxkbcommon-x11-0 mesa-vulkan-drivers` (windowed runs; without the first, window creation panics loading `libxkbcommon-x11.so`, `crates/rune/README.md:2467-2472`)

## 2. Store foundation

- [x] 2.1 Create the schema for `deck`, `card`, `prompt`, `tag`, `review_log`, `editor_draft` and `settings` inside a `PRAGMA user_version` migration runner, and verify opening a fresh database applies migration 1 exactly once
- [x] 2.2 Implement the store task: a command queue carrying ids, a `Connection` it owns exclusively, one event per command, and a published snapshot, and verify a test asserts each command produces exactly one event and that `poll` returns false with no commands in flight
- [x] 2.3 Expose the `Store` trait the UI talks to (typed mutations, snapshot queries, `stats`, `history`) and verify every method is reachable without a `Connection` leaking outside the module
- [x] 2.4 Implement the quit path: `on_quit` sends a flush and joins the store task, and verify a test that a grade sent and then quit is present when the database is reopened
- [x] 2.5 Open the database at `<AppDirs::data()>/typr.sqlite3`, treat `AppDirs::new` returning `None` as a reported startup failure, and verify a path that exists but is not a database produces an error rather than being overwritten
- [x] 2.6 Verify migration behaviour both ways: a version 0 database is upgraded to the current version, and a database one version ahead is refused and left byte-identical
- [x] 2.7 Honor `RUNE_DIRS_ROOT` everywhere a path is derived, and verify a test run pointed at a temporary directory creates no files under the real app data directory

## 3. Scheduling

- [x] 3.1 Implement the `Scheduler` trait and the fixed-multiplier scheduler (learn `remembered` → 10 minutes, graduation `remembered` → 1 day, later `remembered` → ×2.5 capped at 365 days, `forgot` in review → `relearn` due in 10 minutes, the success after it → 1 day), and verify unit tests for each branch including the 200-day → 365-day cap
- [x] 3.2 Create one `fwd` prompt for every card with two or more sides and none for one-sided cards, and verify a test that counts prompts per card for both cases, including a card edited from two sides to one and back
- [x] 3.3 Append a `review_log` row for every grade with its phase, recording the graduating grade as `review`, and verify a test where three `learn` grades and two `review` grades yield a review count of two and five history rows
- [x] 3.4 Implement the fold that recomputes a `prompt` row from `review_log`, and verify fold-equality for five graded prompts plus a log containing `rev`, `cloze:1` and `mask:a1` keys that folds to one prompt per key
- [x] 3.5 Implement the due query returning prompts due at the injected clock, ordered by whole study days overdue with ties shuffled, and verify a test that a week-overdue prompt precedes two same-day ones and that a tomorrow-due prompt is excluded
- [x] 3.6 Implement reset-history for one prompt, deleting all rows including `learn`, and verify the card returns to the new-card queue with no due date
- [x] 3.7 Store the new-cards-per-day limit in `settings` (default 20) and count it per study day — earliest `review`-phase record within a day starting 04:00 local — and verify that a card deferred by the limit is still new on the next study day, and that a grade at 23:50 and one at 00:30 fall on the same study day

## 4. Card library

- [x] 4.1 Implement deck creation, nesting and reparenting with cycle rejection and per-parent name uniqueness, and verify tests for a move into the deck's own child and for a duplicate name under the same parent
- [x] 4.2 Implement card creation, edit and reassignment between decks, and verify a test that a moved card keeps its `prompt` row and due date
- [x] 4.3 Implement tags stored on the card, case-insensitive matching with first-seen casing preserved, and verify a test that filters by `grammar` and matches `Grammar`, and that a `#` heading produces no tag
- [x] 4.4 Implement card search over `search_text` with case and diacritic folding, and verify `cancion` finds `Canción` and that results stay within the selected deck's subtree
- [x] 4.5 Implement archive for cards and decks with the archived view and restore, and verify an archived due card is absent from the due queue and its due date survives restore

  **Open**: `Command::ArchiveCard` and `Db::archive_card` exist and are tested, but nothing in `src/` calls them and no control switches `Library::mode`, so the archived view cannot be reached. The remaining work is the screen, not the store.
- [x] 4.6 Implement the studyable predicate (not archived, not trashed, no archived ancestor), and verify a card under an archived deck is absent from both queues while a sibling deck's cards remain
- [x] 4.7 Implement trash, restore and empty, and verify restored cards keep their history while emptied ones are gone with their `review_log` rows and appear nowhere in the library

  **Open**: `trash_card`, `restore_card` and `empty_trash` exist and are tested, but no screen calls them, so deletion, restore and emptying are unreachable.
- [x] 4.8 Build the library view: sidebar of decks, virtualised card list as a growth stack — `col().growth(GrowthSpec::default()).virtual_rows(count, row_height, key, build)` — tag filter and search box, and verify its scene renders with 5000 seeded cards while building only the mounted rows (assert the build closure ran for far fewer than 5000 indices)
- [x] 4.9 Add the archived and trash views: a control that switches `Library::mode` between active, archived and trash, a restore action in each, and an empty-trash action behind a confirmation dialog, with a headless scene per view

  **Verification**: a scripted run archives a card, switches to the archived view, restores it, and finds it active again; a second run trashes a card, switches to trash, empties it behind the dialog, and asserts the card and its `review_log` rows are gone; a scene per mode snaps non-blank
- [ ] 4.10 Add deck actions to the sidebar: create, rename, archive and reparent, using the store commands that already exist, with the deck name editable in place and the archive action behind a confirmation when the deck holds cards

  **Verification**: a scripted run renames a deck, moves it under another, and archives it, then asserts the sidebar shows the new name, the new parent, and the deck's cards absent from both queues; a duplicate name under one parent is refused and shown as a failure
- [ ] 4.11 Add tag editing to the editor: a control that sets a card's tags through `Command::SetTags`, showing the tags the card holds and refusing duplicates that differ only by case

  **Verification**: a scripted run adds "Grammar" then "grammar" and asserts one tag remains, displayed with the first casing, and that the library's tag filter lists it once

## 5. Card editor

- [x] 5.1 Implement the side splitter: a line separates sides when, ignoring surrounding spaces and a trailing carriage return, it is exactly `---`, outside fenced code; empty sides are dropped. Verify unit tests for two sides, three sides, `***`, padded `---`, CRLF, `---` inside a fence, and a trailing separator
- [x] 5.2 Build the editor as a split surface with a `TextArea` and a rendered `Markdown` preview, and verify typing `**bold**` changes the preview without a mode switch
- [x] 5.3 Cache parsed sides per card revision through `rune_kit::markdown::cache::parse_cached`, and verify a test that editing one card leaves other cards' cached parses untouched
- [x] 5.4 Implement save, the unsaved-changes indicator, and the leave-confirmation dialog, and verify a test that abandoning an edited card prompts save/discard/stay
- [x] 5.5 Persist the in-progress draft in `editor_draft` and restore it on launch marked unsaved, and verify by editing, quitting, and relaunching
- [x] 5.6 Refuse to save a card whose sides are all whitespace and report the reason, and verify with a whitespace-only card
- [x] 5.7 Implement the image resolver over `<AppDirs::data()>/attachments/`, refusing escaping paths and drawing a placeholder for missing files, and verify an attached image renders in the preview and in study while an escaping reference does not load
- [x] 5.8 Implement the attach action that copies a chosen file into the attachments directory and inserts its reference, and verify the copied file exists and the card text references it

## 6. Study session

- [x] 6.1 Build the new-card queue over studyable two-sided cards with no scheduling state, deck-scoped, and verify a new card appears there, not in the due queue, and that a one-sided card appears in neither
- [x] 6.2 Implement `forgot` in the learning queue as a move to the end of the remaining queue with no due date, and verify with a four-card queue that the forgotten card returns last
- [x] 6.3 Implement sequential side reveal with the grade offered only after the last side, in both queues, and verify a three-sided card produces one review and that grade actions are unavailable before the last reveal
- [x] 6.4 Bind the keyboard actions (reveal, remembered, forgot, forward, backward) and show the bindings in the session, and verify a scripted run of a whole queue that uses no pointer, that moving backward shows a graded card read-only, and that moving forward past an ungraded card leaves it queued
- [x] 6.5 Advance the session only on the store's acknowledgement for the grade and implement resume, and verify that quitting after 5 of 12 due cards leaves 7 in the next session
- [ ] 6.6 Implement re-shuffling the due queue within equal-overdue groups, and verify the order changes inside a group, the order between groups does not, and every `prompt.due_at` stays identical

  **Open**: the seeded shuffle inside `Db::due_prompts` is tested, but no key or button re-rolls a queue, and `Session::Entry` does not carry its overdue day, so the session cannot regroup. Task 6.9 is the missing half.
- [x] 6.7 Add prompts that fall due mid-session to the running due queue, and verify that a `forgot` grade producing a 10-minute relearning step returns within the same session when the clock is advanced
- [x] 6.8 Build the session summary: grades recorded in this session, prompts due at that moment, and a way back to the library, and verify against a scripted session's known grade sequence
- [ ] 6.9 Give the queue a reshuffle the user can reach: carry each entry's overdue study day so a new seed can reorder entries within equal days only, bind it to a key and a button, and keep every prompt's `due_at` untouched

  **Verification**: a scripted session with four prompts on one overdue day and two on another reshuffles twice and asserts the within-day order changed at least once, the two-day boundary order never changed, and every `prompt.due_at` is identical afterwards; the key and the button both produce a reorder

## 7. Integration and headless verification

- [x] 7.1 Build the card history view: review count excluding `learn` rows and the full record list with phase, grade, time and due date, and verify it against a card with three learn and two review rows
- [x] 7.2 Register scenes for every screen state (empty library, populated list, editor with unsaved changes, new-card queue, due queue with revealed side, summary, card history) driven from fixtures, and verify `--list-scenes` reports all of them and that no scene writes under the real app data directory
- [x] 7.3 Add `--snap` output per scene and verify every registered scene produces a non-blank PNG
- [ ] 7.4 Add the CI workflow: Ubuntu runner, install the Linux packages Rune's README lists (`libxkbcommon-dev libwayland-dev` to build, `libxkbcommon-x11-0 mesa-vulkan-drivers` for windowed runs), build debug, run the test suite, run `--snap` per scene, and verify the workflow is green on a pushed branch
- [x] 7.5 Run an end-to-end scripted session (create deck, create card, learn it, advance the clock a day, review it, check stats) and verify the recorded `review_log` matches expectations
- [x] 7.6 Verify a release build with the `tooling` feature accepts `--snap` and `--script`, and record the command in `README.md`

## Workflow follow-up

- Archive the change after review requirements are satisfied.
- Verify the archived result and that `openspec/specs/` now contains the four capabilities.
