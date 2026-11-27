# UI Review Fixes: Design

**Date:** 2026-11-27
**Source:** a UI/UX visual review of the running app (macOS, light and dark, at the default window width) using a seeded library. The findings were observed in screenshots taken with the `run-typr` driver.
**Plan:** [docs/plans/2026-11-27-ui-review-fixes.md](../plans/2026-11-27-ui-review-fixes.md)

## Findings

| # | Finding (observed in screenshots) | Task |
|---|---|---|
| 1 | No visible way to start a study session; only the `s` / `n` keys do it | 4 |
| 2 | "Due today" with 0 due cards shows "This deck has no cards yet" and a full-width New card button | 4 |
| 3 | The selected fill shows on Settings and Trash, but not on Inbox, Due today, New cards or decks | 2 |
| 4 | Pressing "Inbox 3" folds its children instead of opening Inbox, and there is no fold mark | 3 |
| 5 | A dead "Dashboard (coming soon)" row sits at the top of the sidebar | 3 |
| 6 | Sidebar leading edges differ (tree rows at x≈20, list rows at x≈33) | 2 |
| 7 | Settings paths overflow the panel's edge | 5 |
| 8 | "New cards a day" field is ~750px wide; Apply is taller than the field | 5 |
| 9 | "Settings / General" breadcrumb with one section; "Find a setting" search over 8 rows; raw "131072 bytes" | 5 |
| 10 | Card tiles have a ~4px gap but ~26px inner padding | 6 |
| 11 | Editor header reads "Saved · Back · 1 prompt"; Save sits under the source, away from the header badge; preview unlabelled | 7 |
| 12 | Search appears twice (sidebar "Search /" only focuses "Search cards") | 3 |
| 13 | Import appears in both the sidebar and Settings; the "Library open" footer is always shown | 3 |
| 14 | The Trash empty state is one bare line | 8 |
| — | `.claude/skills/run-typr/shoot.sh` waits for "Show review counts", which no longer exists | 1 |

**Root cause of finding 3 (verified by reading the code, not by a test):** `crates/typr/src/ui/tree.rs` `render_item` never draws a selected fill, and `nsh_base::TreeEntry` has no `selected` field (fields: `id`, `label`, `depth`, `folder`, `disabled`). Settings and Trash are `list_row`s, which do draw `tokens::row_fill`.

## Decisions

- **Inbox:** it becomes a flat row, with Due today and New cards as flat rows beside it. Removing the fold fixes finding 4 at its cause, so there's no need to add a chevron.
- **Search:** it lives only in the library field. The field's placeholder carries the `/` shortcut.
- **Import:** it lives only in Settings.
- **"Library open" footer:** it shows only when the library is closed.
- **Settings:** the breadcrumb and "Find a setting" are dropped, because 8 rows don't need a search. The size reads in units ("128 KB"). A long path wraps inside the panel and is never cut short.
- **Editor:** Save moves into the header, after Back and the badges, and is disabled while the card is Saved. The preview gets a "Preview" label.
- **Inbox views:** they carry "Review due N" and "Review new N" buttons, each disabled at 0. An empty filter says why it's empty, and a search query's message takes precedence.
- **Grid:** one gap token (12pt) spaces the tiles, and the tile padding drops to the `p_3` step.
- **Trash:** the empty state gets a title and one line explaining what the trash holds.

## Out of scope

The review didn't cover these: hover and pressed states, keyboard tab order, a screen-reader pass, larger text, and the study screen itself.
