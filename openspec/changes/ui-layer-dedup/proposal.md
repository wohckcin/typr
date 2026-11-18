# Proposal — ui-layer-dedup

## Why

The `ui` layer wraps `nsh/base` as the NSH guides ask (behaviour from Base, look from the app), but it and the
shell carry copies that must change together, and helpers nothing calls. Each item below was checked by grep on
2026-11-18.

- **Two store writers.** `apply` and `quiet_apply` (`crates/typr/src/shell.rs:282-342`) are the same 30 lines
  except that `apply` re-reads the snapshot. The grade writer (`:586-607`) repeats the error handling a third time.
  A fix to the slow-command timing or the stoppage flag has to be made three times.
- **The library view drawn twice.** The editor's "card went away" branch (`shell.rs:638-652`) copies the
  `Screen::Library` branch (`:768-782`). `shell`'s "A screen exists once" requirement forbids a second copy.
- **Three styled text fields.** `Field`, `field_reachable` and `SubmitField` (`crates/typr/src/ui/input.rs`)
  repeat the same 12 style lines. `SubmitField` has no caller.
- **Two source editors.** `source` and `source_with` (`crates/typr/src/ui/source.rs:10-59`) differ by an
  `on_change` and a handle; only one has an accessibility label.
- **Badge and Keycap** (`crates/typr/src/ui/status.rs:17-28`, `:59-70`) have the same body.
- **Unused or one-line helpers:** `skeleton`, `spinner_label` (`status.rs:73-84`), `separator`
  (`surface.rs:28`), `hint` (`overlay.rs:55`), `PRESS_MS` (`button.rs:203`) have no caller; `tree::item` /
  `tree::Items` (`tree.rs:64-69`) and `tokens::flush` (`tokens.rs:147`) only rename `TreeItem::new`, `Vec` and
  `px(0.0)`.

Out of this change on purpose: the unused motion tokens (`SLOWER`, `RISE`, `SLIDE`, `ENTER`, `ENTER_SCALE`,
`PRESS_SCALE_ICON`), which `motion-coverage` plans to use; `TOAST_MS`, which `use-base-toast` handles; and the
`tokens::` accessors over `theme()`, which the guides require and `tests/tokens.rs` enforces.

## What Changes

- One `write(command, republish)` in the shell; `apply`, `quiet_apply` and the grade path call it.
- One `library_view()` closure drawn by both branches.
- `SubmitField` deleted; the field style is one private function the two remaining fields share.
- `source` folded into `source_with` (optional `on_change`), keeping the accessibility label.
- `Keycap` and `Badge` share one body; `Keycap` keeps its name, which `DESIGN.md` records.
- The unused helpers deleted; the one-line wrappers inlined at their callers.

Two intended behaviour changes, each a consequence of the merge (design decisions 2 and 3):

- **A slow grade is recorded.** The grade path skips the timing today; through `write` it lands on the
  diagnostics screen like any slow command (scenario "A slow grade is recorded").
- **The editor's fallback library uses the library's spacing.** It drew with `gap_2` where the library screen
  uses `gap_3`; one `library_view()` means one gap (scenario "A card gone under the editor").

Nothing else changes for the user. Net about -170 lines.

## Capabilities

### Modified Capabilities

- `shell`: the store has one writer, and the library is drawn by one function in every branch.

## Impact

`crates/typr/src/shell.rs`, `crates/typr/src/ui/{input,source,status,surface,overlay,button,tree,tokens}.rs`,
`crates/typr/src/screens/{sidebar,editor/diagram}.rs` (inlined wrappers), callers of `source(..)`. Tests: existing
suites must pass unchanged, plus one test for the slow grade.
