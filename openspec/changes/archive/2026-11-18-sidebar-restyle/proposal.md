# Proposal — sidebar-restyle

## Why

The user supplied a reference screenshot (a "Settings / General" page of another app) and asked for the sidebar
and the settings screen to look like it. The settings screen is `settings-rows-layout`. This change is the sidebar.

Compared row by row, most of the reference's sidebar is already what `crates/typr/src/screens/sidebar.rs` draws:

| Reference | typr today |
|---|---|
| Muted section headings over their rows | `section` draws `group.label()` in `text_xs` / `muted_foreground`, rows below |
| Groups separated by a larger gap than rows | groups `gap_3`, rows `gap_1` |
| The row in force is a filled, rounded row | `tokens::row_fill(selected)`, the hover wash |
| Settings among the utilities | the Utilities group holds Settings and Diagnostics |
| A fixed-width column with a trailing hairline | `sidebar_width()` (220 pt), `border_r_1` |

What differs, and what this change does about it:

- **The status sits in the wrong place.** The reference pins a status row (a dot and the state, "Gateway
  127.0.0.1:3…") to the bottom of the column. typr draws "Library open" directly under the search row, so the
  first thing in the column is a status line and the groups start below it. The state belongs at the bottom, out
  of the way of the navigation, and the status can use the dot the reference shows.
- **The status has no mark but its colour.** "Library open" reads in the muted tone and "Library closed" in the
  danger tone, which is colour alone; the reference's dot is a second cue.
- **There is no success tone.** The dot in the reference is green. typr's theme has no success token
  (`theme.json` holds only `review-backdrop`), and raw colours are not allowed in a screen.
- **The reference's rows carry icons, the column has a collapse button and Settings opens sub-pages.** Out of
  scope below.

## What Changes

- The library state moves from under the search row to a footer pinned to the bottom of the column, separated from
  the groups by a hairline, drawn as a dot and the state text.
- The dot uses a new `success` colour token when the library is open and `destructive` when it is closed, so the
  state is a shape and a colour, and the text still says it.
- `theme.json` and `ui/tokens.rs` gain `success` (light and dark), read through an accessor like every other colour.
- The search row stays first, and the groups follow it directly.

Out of scope, on purpose:

- **Icons on the rows**: the app has no icon family, and the guides ask for one family at one stroke weight; adding
  one is its own decision.
- **A collapse button for the column and the window-control row** (the reference's top bar): the title bar here is
  hidden and drawn by the host.
- **Settings sub-pages in the sidebar** ("General", "Allowances", …): typr's settings are one page
  (`settings-rows-layout`).
- **The deck tree, counts, folds and keyboard** (all unchanged).

## Capabilities

### Modified Capabilities

- `sidebar`: the library's state is a footer at the bottom of the column, marked by a dot as well as its tone.

## Impact

- `crates/typr/src/screens/sidebar.rs` (the column's structure; `library_state` moves and gains a dot),
  `crates/typr/theme.json` and `crates/typr/src/ui/tokens.rs` (`success`).
- `crates/typr/tests/sidebar.rs`, `tests/shell.rs`, `tests/startup.rs`, `desktop.test.tsx`, `library.test.tsx`: the
  state text stays exactly `Library open` / `Library closed…`, so their assertions are unchanged.
- `tests/appearance.rs`: the sidebar is already in the light and dark pass.
- `DESIGN.md`: the sidebar row of the appearance review.
