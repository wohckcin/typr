# Design — sidebar-restyle

## Context

`sidebar(state, pick)` (`crates/typr/src/screens/sidebar.rs`) returns one column: `view().w(sidebar_width()).p_3()
.gap_3().border_r_1()`, with `search_row`, `library_state`, then a `dynamic` that builds the groups. `library_state`
is a `text` whose string and colour follow `state.library_open` and `state.notice`; "Library open" is muted, "Library
closed…" is `destructive`. The column does not fill the window's height with anything: its content ends where the
groups end.

Tests and hosts read the state text: `tests/sidebar.rs:117`, `tests/shell.rs:90` and `tests/startup.rs:83` wait for
`Library closed…`; `desktop.test.tsx:15` filters texts equal to `Library open` or starting with `Library closed`;
`library.test.tsx:9` matches `/^Library closed/`. The text must therefore stay one text node with those exact words.

`theme.json` holds `review-backdrop` only; the base palette comes from the `nsh/base` theme, whose tokens
`ui/tokens.rs` reads through accessors (`tests/tokens.rs` enforces that a screen names a token, not a colour).

## Decisions

1. **The column is a flex column; the groups take the free height, the footer sits last.** The column keeps its
   `w`, padding and trailing hairline; the groups' `dynamic` gets `flex_1`, and the footer follows it, so the footer
   is at the bottom for any window height and for any number of rows, without measuring. Rejected: absolute
   positioning with a bottom inset — it would overlap a long deck list instead of making room.
2. **The footer is `state_row(state)`: a hairline above, then a row of a dot and the existing text.** The text node
   is the current `library_state` text, moved and unchanged, so every assertion above still finds it. The hairline
   is a `border_t_1` on the footer, the one place that boundary is drawn, with the groups' own region having no
   border below.
3. **The dot is a small circle view coloured by state.** `tokens::success()` when the library is open,
   `tokens::destructive()` when it is closed. Its size is a `ui` constant beside the other control geometry
   (`ui/size.rs`), not a literal in the screen. The dot is decoration for a sighted reader; the text carries the
   name, so a screen reader is not told twice (the dot has no accessible name).
4. **`success` is a theme token.** `theme.json` gains `success` in `light` and `dark` (a green with contrast on
   both backgrounds, checked in the screenshots), and `tokens::success()` is the accessor, like the others.
   Rejected: reusing `selection` or `ring` for "open" — neither says "fine".
5. **Nothing else about the rows changes.** The headings, gaps, row fill, tree, counts and the keyboard are what the
   reference already shows; they are left alone so the change is only the footer and its dot.

## Risks

- **A hidden footer in a short window.** If the groups' region is shorter than its content, the footer must not be
  pushed off the column. The groups' region scrolls (`overflow_y_scroll` is already how the tree behaves in the
  base element); checked at a 390 × 600 viewport.
- **The success colour in light mode.** A green dot on the near-white page can fall under non-text contrast (3:1).
  The light value is chosen and measured, not copied from the dark one.
- **The title-bar inset.** The shell already pads the work area below the host's title bar; the footer adds no
  inset of its own at the bottom.
