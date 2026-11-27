# UI Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the 14 findings of the 2026-11-27 UI/UX visual review of typr, so study is reachable, the sidebar navigates and highlights consistently, and Settings, the grid, the editor and Trash read cleanly in light and dark.

**Architecture:** Every change stays inside the existing layers. Styling goes in the app's `ui` layer (`crates/typr/src/ui/*`), using colours from `tokens` only. Screen composition goes in `crates/typr/src/screens/*`, and wiring goes in `crates/typr/src/shell/*`. Most fixes are deletions or re-orderings. The only new logic is three small pure functions: `empty_message`, `human_size` and the grid gap. Each has a unit test.

**Tech Stack:** Rust, NSH (`nsh-core` signals, `nsh/base` tree and virtual list), `nsh_testing::Headless` for behaviour tests, and `nsh test nsh.json --target macos` for screenshots.

**Spec:** [docs/specs/2026-11-27-ui-review-fixes-design.md](../specs/2026-11-27-ui-review-fixes-design.md). Its findings table maps each finding to its task here, and its decisions are repeated in the tasks that carry them. The design rules are in `.claude/skills/nsh-design-guides/references/design-guides.md`; check the Finishing checklist at lines 1536-1641 and the Design review checklist at 1642 onward.


## Global Constraints

- Theme colours come from `crates/typr/src/ui/tokens.rs` only. No raw colour values in screens.
- Commits use Conventional Commits: `type(scope): summary`, lowercase, at most 72 characters.
- Work on a branch `fix/ui-review`, not `main`. Don't push or open a PR unless asked.
- After each task, `cargo clippy -p typr --all-targets` must report no warnings and `cargo test -p typr` must pass. Run `cargo test --workspace` once at the end.
- Each task that changes pixels must be checked in a screenshot, in light and dark, with `sh .claude/skills/run-typr/shoot.sh <dir>` or the steps given in the task. Behaviour tests alone don't verify layout.
- Don't set `TYPR_DATA_DIR` in the shell's environment, because it breaks `library.test.tsx`. Set it per command only.
- Copy rules: sentence case, no trailing period on labels, and a count follows the word ("Review due 3").

## Review Focus

1. **An empty library.** With zero cards, the review buttons must be disabled and must not open a blank study screen. The inbox empty copy must not say "deck". (Task 4 test: `review_buttons_are_disabled_when_their_queue_is_empty`.)
2. **A search query that hides everything on an inbox filter.** The empty copy must say no cards match the search, not "Nothing is due". (Task 4 test: `a_query_takes_precedence_in_the_empty_message`.)
3. **The keyboard moves the tree selection.** After Task 3 flattens Inbox, Down/Up must still move the selection and the fill must follow it. (Task 2 test: `the_tree_reports_the_row_the_arrow_moves_to`.)
4. **Long deck names and long paths in a narrow window.** They must wrap inside their column, never paint past the border. (Task 5: screenshot step at the default phone-width window.)
5. **Dark mode selection contrast.** `tokens::hover()` must differ visibly from `background()` in dark. (Task 2: the dark screenshot step.)

---

### Task 1: Fix the screenshot script

**Files:**
- Modify: `.claude/skills/run-typr/shoot.sh:37`

**Interfaces:**
- Produces: a working `sh .claude/skills/run-typr/shoot.sh <dir>`, which every later task's visual check uses.

- [ ] **Step 1: Reproduce.** Run `sh .claude/skills/run-typr/shoot.sh /tmp/typr-before`. Expected: it fails waiting for "Show review counts", and fewer than six PNGs are written.
- [ ] **Step 2: Fix.** Replace `await app.waitFor("Show review counts");` with `await app.waitFor("New cards a day");`.
- [ ] **Step 3: Verify.** Run it again into `/tmp/typr-before`. Expected: six non-blank PNGs. Keep this directory as the baseline for every later task.
- [ ] **Step 4: Commit.** `git commit -am "fix(run-typr): wait for a settings label that still exists"`

### Task 2: Tree rows show the selection and share the list rows' leading edge

**Files:**
- Modify: `crates/typr/src/ui/tree.rs` (`render_item`)
- Test: `crates/typr/tests/sidebar.rs`

**Interfaces:**
- Consumes: `tokens::row_fill(selected: bool) -> AdaptiveColor`, and the horizontal padding `list_row` uses in `crates/typr/src/ui/choice.rs` (`.p_1()` around line 47, plus its glyph gap).
- Produces: a tree row whose fill is `row_fill(entry.id == selected)`. No new API.

- [ ] **Step 1: Write the failing test** in `crates/typr/tests/sidebar.rs`, in the existing `Probe` / `start()` style:

```rust
#[test]
fn the_tree_reports_the_row_the_arrow_moves_to() -> Result<(), String> {
    let mut app = start();
    app.press("Spanish 1")?;
    app.wait_for("selected=spanish")?;
    app.keyboard_press("down")?;           // use the harness's key call; see tests/button.rs and persistence.rs
    app.wait_for("selected=verbs")?;
    Ok(())
}
```

- [ ] **Step 2: Run it.** `cargo test -p typr --test sidebar the_tree_reports`. If it already passes, the keyboard model is fine and the test stays as a regression guard. The fill itself is checked visually in Step 5.
- [ ] **Step 3: Implement.** In `render_item`, read `props.selected.get_untracked()` and give the row `.bg(tokens::row_fill(selected == Some(entry.id)))`, `.rounded(tokens::radius)` and the same horizontal padding as `list_row`, so the glyphs line up on one spine. Make it `.w_full()` so the fill spans the column. If `render_item` isn't re-run when the selection changes, read `props.selected` inside a `move ||` passed to `.bg(...)` instead.
- [ ] **Step 4: Run the tests.** `cargo test -p typr --test sidebar --test a11y`. Expected: PASS.
- [ ] **Step 5: Visual check.** Run `shoot.sh /tmp/typr-t2`, then copy its steps and add presses for "Due today" and "Spanish". In light and dark:
  - the pressed tree row has the same fill as Settings does when selected;
  - the tree glyphs and the New Deck and Utilities glyphs line up in one column (±1px).
- [ ] **Step 6: Commit.** `git commit -am "fix(ui): fill the selected tree row and align it with list rows"`

### Task 3: Sidebar rows that all do something

**Files:**
- Modify: `crates/typr/src/screens/sidebar.rs` (`items_from`, `sidebar`, `search_row`, `state_row`, `library_state`)
- Modify: `crates/typr/src/shell/mod.rs` (`sidebar_picker`: drop the `"import"` and `SEARCH_KEY` branches)
- Modify: `crates/typr/src/screens/library.rs` or `shell/mod.rs:166` (search placeholder)
- Test: `crates/typr/tests/sidebar.rs`, `crates/typr/tests/a11y.rs`, `crates/typr/tests/persistence.rs`, `tests/desktop.test.tsx`, `tests/library.test.tsx`

**Interfaces:**
- Produces: `items_from` returns `inbox`, `inbox:due` and `inbox:new` as three top-level Navigation rows with `parent: None`. It returns no `dashboard` and no `import` row, and Utilities is `settings` then `trash`. `SEARCH_KEY` and `search_row` are deleted.

**Decisions (flag to the user if they disagree):**
- Inbox becomes a flat row. This removes the fold, which was the cause of finding 4, rather than adding a chevron.
- Search stays only as the library field. Its placeholder becomes `Search cards  /`, built from `search_chord()`, so the shortcut stays discoverable.
- Import stays only in Settings.
- The footer shows only when the library is closed.

- [ ] **Step 1: Update the tests first.**
  - In `sidebar.rs`:
    - delete `pressing_the_dashboard_changes_nothing`;
    - change the `items_from` assertion near line 246 to the new list (no "Import"; `items[0].key == "inbox"` and not disabled);
    - change line 170's `at("Search")` ordering check to start at "Inbox";
    - change line 196 to assert `!open.screen().shows("Library open")`.
  - In `a11y.rs`:
    - delete the Dashboard assertion at line 116-118;
    - change line 209's list to `["Settings", "Trash"]`.
  - In `persistence.rs:56-57`, reach Import via `app.press("Settings")` first.
  - In `desktop.test.tsx:10`, drop "Search", and at line 15 expect no "Library open".
  - In `library.test.tsx:11`, drop "Search".
  - Add:

```rust
#[test]
fn pressing_inbox_opens_it_and_keeps_its_filters_visible() -> Result<(), String> {
    // Real shell over a seeded library (copy the library fixture from tests/shell.rs).
    app.press("Inbox 3")?;
    app.wait_for("Due today")?;            // still visible: nothing folded
    app.wait_for("Search cards")?;         // the library screen is showing
    Ok(())
}
```

- [ ] **Step 2: Run them to see them fail.** `cargo test -p typr --test sidebar --test a11y --test persistence`. Expected: FAIL on Dashboard, Import and Search still being present.
- [ ] **Step 3: Implement.**
  - In `items_from`:
    - delete the `dashboard` item and the `import` push;
    - build `inbox:due` and `inbox:new` with `parent: None`, and set `inbox.has_children = false`.
  - Delete `search_row` and `SEARCH_KEY`, along with their `sidebar_picker` branch.
  - Delete the `"import"` branch.
  - Make `state_row` render nothing while `library_open` is true.
  - Delete the `Group::Navigation` doc mention of the dashboard (line 24) and `Icon::Trend` if nothing else uses it.
- [ ] **Step 4: Run the tests.** `cargo test -p typr`, then `nsh test .`. Expected: PASS.
- [ ] **Step 5: Visual check.** Run `shoot.sh /tmp/typr-t3`. The sidebar should read: Inbox, Due today, New cards, Decks, New Deck, Settings, Trash. There should be no footer and no Search row. The library field should show `Search cards  /`.
- [ ] **Step 6: Commit.** `git commit -am "fix(sidebar): drop dead and duplicate rows and open inbox on press"`

### Task 4: Start a review from the inbox, and say why a filter is empty

**Files:**
- Modify: `crates/typr/src/screens/library.rs` (add `empty_message`)
- Modify: `crates/typr/src/screens/grid.rs:92-131` (`grid` takes the empty message; New card only for a deck)
- Modify: `crates/typr/src/shell/mod.rs:156-175` (`library_view`: a review row above the search field)
- Test: `crates/typr/tests/library.rs`, `crates/typr/tests/grid.rs`, `crates/typr/tests/shell.rs`

**Interfaces:**
- Consumes: `cards_in(&Snapshot, Option<&str>, now_ms) -> Vec<CardSummary>`, `StudyController::start(Queue)`, and `Sidebar::selected_key() -> Option<SharedString>`.
- Produces:
  - `pub fn empty_message(key: Option<&str>, query: &str) -> &'static str` in `screens/library.rs`.
  - `grid(tiles, chosen, empty: impl Fn() -> SharedString + 'static, on_new: Option<impl Fn() + Clone + 'static>, on_open)`, where `on_new: None` hides the New card button.

**Copy:**

| key / state | message | New card shown |
|---|---|---|
| query non-empty (any key) | `No cards match your search` | no |
| `inbox:due` | `Nothing is due. You're done for today` | no |
| `inbox:new` | `No new cards to learn` | no |
| `inbox` | `No cards are due or new` | no |
| `deck:*` or other | `This deck has no cards yet` | yes |

Review buttons: `Review due N` and `Review new N`, where N is `cards_in(.., Some("inbox:due"|"inbox:new"), now).len()`. Each is disabled when N is 0, and they show on every library view.

- [ ] **Step 1: Write the failing tests.**

```rust
// tests/library.rs
#[test]
fn each_filter_says_why_it_is_empty() {
    assert_eq!(empty_message(Some("inbox:due"), ""), "Nothing is due. You're done for today");
    assert_eq!(empty_message(Some("inbox:new"), ""), "No new cards to learn");
    assert_eq!(empty_message(Some("inbox"), ""), "No cards are due or new");
    assert_eq!(empty_message(Some("deck:3"), ""), "This deck has no cards yet");
    assert_eq!(empty_message(None, ""), "This deck has no cards yet");
}
#[test]
fn a_query_takes_precedence_in_the_empty_message() {
    assert_eq!(empty_message(Some("inbox:due"), "verb"), "No cards match your search");
}
```

In `tests/shell.rs` (real shell, seeded library: 1 due card and 0 new):
- `review_due_opens_the_study_screen`: press "Review due 1", then wait for the study screen's reveal control.
- `review_buttons_are_disabled_when_their_queue_is_empty`: press "Review new 0", and the library screen (`Search cards`) is still shown.
- `an_empty_due_filter_offers_no_new_card_button`: on a library with 0 due, press "Due today", wait for "Nothing is due. You're done for today", and assert `!shows("New card")`.

- [ ] **Step 2: Run them to see them fail.** `cargo test -p typr --test library --test shell`. Expected: they don't compile, because `empty_message` is missing.
- [ ] **Step 3: Implement.**
  - Add `empty_message` with a `match` that checks the query first.
  - Thread `empty` and `on_new: Option<_>` through `grid`, and update the grid's call sites, including `crates/typr/tests/grid.rs`.
  - In `library_view`, add a `flex_row().gap_2()` above the search field holding two `Variant::Default` buttons. Each button's `.disabled` and `.label` read a `computed` count from `snapshot_signal`, and `.on_press` calls `study.start(Queue::Due)` or `study.start(Queue::New)`.
- [ ] **Step 4: Run the tests.** `cargo test -p typr`. Expected: PASS.
- [ ] **Step 5: Visual check.** Press Due today with nothing due, then a deck, in light and dark. The empty copy should match the table, and the review row should sit above the search field without wrapping at the default window width.
- [ ] **Step 6: Commit.** `git commit -am "feat(library): start reviews from the inbox and explain empty filters"`

### Task 5: Settings that fit their panel

**Files:**
- Modify: `crates/typr/src/screens/settings.rs` (`header`, `cap_control`, `path_control`, the schema row, `Values::query`, `row_matches`)
- Test: `crates/typr/tests/settings.rs`

**Interfaces:**
- Produces: `pub fn human_size(bytes: u64) -> String` in `screens/settings.rs`. `row_matches` and `Values::query` are deleted.

**Decision (flag to the user):** drop the breadcrumb and "Find a setting". With 8 rows the search adds a control and saves nothing. The screen's title becomes plain `Settings`, using the `text_lg` the other screen titles use, if they have one; otherwise `text_sm` in `foreground`.

- [ ] **Step 1: Write the failing tests, and delete the dead ones.**
  - Delete:
    - `a_row_matches_by_a_case_insensitive_part_of_its_label`
    - `typing_a_name_hides_the_other_rows`
    - `a_query_nothing_matches_says_so`
    - `clearing_the_query_restores_the_rows`
  - Add:

```rust
#[test]
fn a_library_size_reads_in_units() {
    assert_eq!(human_size(512), "512 bytes");
    assert_eq!(human_size(131_072), "128 KB");
    assert_eq!(human_size(5_242_880), "5.0 MB");
}
#[test]
fn the_screen_has_no_breadcrumb_or_setting_search() -> Result<(), String> {
    let mut app = start();                 // existing Probe
    app.wait_for("New cards a day")?;
    assert!(!app.screen().shows("General"));
    assert!(!app.screen().shows("Find a setting"));
    Ok(())
}
```

  And change `the_screen_reports_where_the_library_is` to expect `Schema 12 · 128 KB` (or the fixture's equivalent), not raw bytes.

- [ ] **Step 2: Run them to see them fail.** `cargo test -p typr --test settings`. Expected: FAIL.
- [ ] **Step 3: Implement.**
  - `human_size`: under 1024 is `"{n} bytes"`; under 1 MiB is `"{n/1024} KB"` as a whole number; otherwise `"{:.1} MB"`.
  - In `header`, keep only the title.
  - In `cap_control`:
    - wrap the field in `view().w(px(80.0))` instead of `flex_1()`;
    - give Apply `.size(Size::Sm)` (`ButtonProps::size` exists) so it matches the field's height.
  - In `path_control`, give the path text `.flex_1().min_w_0()` so it wraps inside the row instead of overflowing. Keep it unelided, as the existing comment requires, and keep the button wrapping under it.
- [ ] **Step 4: Run the tests.** `cargo test -p typr --test settings --test persistence`. Expected: PASS.
- [ ] **Step 5: Visual check.** Run `shoot.sh /tmp/typr-t5` and look at the settings shots in light and dark at the default window. Check that:
  - no path text crosses the panel's border;
  - the cap field is about 80px wide;
  - Apply and the field share a top and bottom edge (±1px).
- [ ] **Step 6: Commit.** `git commit -am "fix(settings): fit paths and the cap field, drop search and breadcrumb"`

### Task 6: Grid spacing

**Files:**
- Modify: `crates/typr/src/ui/tokens.rs:132-151`, `crates/typr/src/screens/grid.rs` (the `virtual_list` gap, the tile surface padding)
- Test: `crates/typr/tests/tokens.rs` (or wherever `grid_columns` is tested; check with `rg grid_columns crates/typr/tests`)

**Interfaces:**
- Produces: `pub const GRID_GAP: f32 = 12.0;` in `tokens.rs`, used by `grid_columns` and by the grid's list gap.

- [ ] **Step 1: Write the failing test.**

```rust
#[test]
fn columns_count_a_twelve_point_gap() {
    use typr::ui::tokens::{grid_columns, TILE_MIN_WIDTH, GRID_GAP};
    let two = 32.0 + 2.0 * TILE_MIN_WIDTH + GRID_GAP;
    assert_eq!(grid_columns(two), 2);
    assert_eq!(grid_columns(two - 1.0), 1);
}
```

- [ ] **Step 2: Run it.** `cargo test -p typr --test tokens columns_count`. Expected: it doesn't compile, because `GRID_GAP` is missing.
- [ ] **Step 3: Implement.**
  - Add `GRID_GAP` and use it in `grid_columns`.
  - Give the `virtual_list` that gap between rows and columns. Check the list's gap method in the nsh skill or `nsh_base`; if it has none, pad each cell by `GRID_GAP / 2` and subtract it in `tile_width`.
  - Reduce the tile's inner padding to the `p_3` step, by overriding the `surface` padding on the tile only.
  - Re-measure `tile_estimate` against a screenshot and update its value.
- [ ] **Step 4: Run the tests.** `cargo test -p typr --test tokens --test grid`. Expected: PASS.
- [ ] **Step 5: Visual check.** In the library screenshot, the gap between tiles should be visibly larger than before and smaller than a tile's inner padding. Compare with `/tmp/typr-before`.
- [ ] **Step 6: Commit.** `git commit -am "fix(grid): space tiles by one gap token and tighten their padding"`

### Task 7: Editor header order and Save

**Files:**
- Modify: `crates/typr/src/screens/editor/mod.rs:275-341` (the header row and the Save button) and the preview column (a label)
- Test: `crates/typr/tests/editor.rs`

**Interfaces:**
- No new API. The header row becomes: `Back` · spacer (`flex_1`) · prompt-count badge · Saved/Unsaved badge · `Save` (`Variant::Primary`). Save is disabled when `!dirty()` or when the draft has no content. The Save button below the source is removed. "Attach image…" and "Make it a diagram…" stay below the source. The preview column gets a `Preview` label (`text_xs`, `muted_foreground`).

- [ ] **Step 1: Write the failing tests.**

```rust
#[test]
fn a_clean_card_cannot_be_saved_again() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("Saved")?;
    let _ = app.press("role:button[Save]");
    // Probe records emitted events the way pressing_save_reports_the_text reads them
    assert!(!app.screen().shows("saved="), "a clean editor emitted Save");
    Ok(())
}
#[test]
fn the_header_puts_back_first() -> Result<(), String> {
    let mut app = Headless::start(EngineConfig::new(Root::of::<Probe>())).unwrap();
    app.wait_for("Back")?;
    let texts: Vec<String> = app.screen().texts().iter().map(|t| t.to_string()).collect();
    let at = |l: &str| texts.iter().position(|t| t == l).unwrap_or(usize::MAX);
    assert!(at("Back") < at("Saved") && at("Saved") < at("Save"), "{texts:?}");
    assert!(app.screen().shows("Preview"));
    Ok(())
}
```

  Match `"saved="` to whatever marker the existing `Probe` writes when it hears `EditorEvent::Save`, and keep `pressing_save_reports_the_text` passing (it types first, so the draft is dirty).
- [ ] **Step 2: Run them to see them fail.** `cargo test -p typr --test editor`. Expected: FAIL.
- [ ] **Step 3: Implement.** Re-order the header children as listed above, and move the existing Save button into the header with `.disabled(move || !dirty() || draft.with(|d| !has_content(d)))`. Keep the save error text under the header.
- [ ] **Step 4: Run the tests.** `cargo test -p typr --test editor --test shell`. Expected: PASS.
- [ ] **Step 5: Visual check.** In the editor screenshot in light and dark, the header should read Back … 1 prompt · Saved · Save, with Save visibly disabled while Saved shows.
- [ ] **Step 6: Commit.** `git commit -am "fix(editor): lead the header with back and save from it"`

### Task 8: Trash empty state

**Files:**
- Modify: `crates/typr/src/screens/trash.rs:62`
- Test: `crates/typr/tests/trash.rs`

**Copy:** a title `Trash is empty` (`text_sm`, `foreground`), and under it `Deleted cards stay here until you empty the trash` (`text_xs`, `muted_foreground`). The pair has `gap_1`, `p_4` and the same left edge as the trash rows.

- [ ] **Step 1: Write the failing test.** Update the existing empty-trash assertion, or add `an_empty_trash_says_what_it_holds`, so it waits for both strings above.
- [ ] **Step 2: Run it to see it fail.** `cargo test -p typr --test trash`. Expected: FAIL on "Trash is empty".
- [ ] **Step 3: Implement** the two lines.
- [ ] **Step 4: Run the tests.** `cargo test -p typr --test trash`. Expected: PASS.
- [ ] **Step 5: Commit.** `git commit -am "fix(trash): explain the empty trash"`

### Finish

- [ ] Run `cargo clippy -p typr --all-targets`, `cargo test --workspace` and `nsh test .`. Expected: all clean.
- [ ] Run `shoot.sh /tmp/typr-after` plus the inbox, trash and deck steps in light and dark. Then run the design guide's Finishing checklist and Design review checklist, and report each group as passed or fixed.
- [ ] Not covered by this plan (they were outside the review): hover and pressed states, tab order, screen reader pass, larger text, and the study screen itself.
