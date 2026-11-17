# Design

Motion inventory for every screen (Design Guides, "Motion inventory"). Tokens are from
`crates/typr/src/ui/motion.rs`. Each line names the tool and the token, or says no motion and why, and
what reduced motion does.

Two kinds of line exist. "Plays" lines are in the running app; they were checked on the macOS host and
by the tests named. "No motion" lines are decisions: the change either has nothing to show or the
movement would cost more than it explains.

## Shell and sidebar

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Sidebar row hover | colour transition, `INSTANT` | instant | plays |
| Group count changes | none: a number updating is not motion | n/a | no motion |
| Sidebar keeps its width | none: the sidebar does not collapse, so there is no layout change to follow | n/a | no motion |
| Current row changes | none: the row's own fill changes in place | n/a | no motion |

No fill slides to the current row. A slide needs each row's measured offset, and the sidebar keeps no
row geometry, so it would add a measurement layer for one transition that a fill change already
explains. The line is revisited if the sidebar measures its rows for another reason.

## Library and card grid

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Tile press | scale to `PRESS_SCALE`, `FAST` | no scale | plays |
| Tile hover | colour wash, `INSTANT`; the card does not move | instant | plays |
| Tiles arrive on page entry | none: a recycled virtual cell replays whatever plays in it, and the library reads in one frame | n/a | no motion |
| Recycled cell or data tick | none: a recycled cell plays nothing | n/a | no motion |
| Empty library | none: the empty state draws at once, because nothing was awaited | n/a | no motion |

## Card editor

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Saved / Unsaved badge | none: the word changes in place; a glyph swap would animate a two-word status | n/a | no motion |
| Picture attached | none: the picture is written into the source text as a reference | n/a | no motion |
| Typing | none: a value that updates per keystroke is not animated | n/a | no motion |

## Study session and review

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Reveal the answer | none: the answer appears in place; the reveal is the result, not a transition | n/a | no motion |
| Grade pressed | scale to `PRESS_SCALE`, `FAST`, through the app's button | no scale | plays |
| Next card | none: the next prompt is set in place | n/a | no motion |
| Session complete | none: the summary draws at once | n/a | no motion |

## History

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Row arrives | none: rows are read in one frame | n/a | no motion |
| Filter change | none: the list is rebuilt in place | n/a | no motion |

## Settings

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Switch toggles | thumb slides, `BASE` (in `ui::toggle`) | instant | plays |
| Section change | none: sections are read top to bottom, not swapped | n/a | no motion |
| Save confirmed | none beyond the control's own state | n/a | no motion |

## Keymap panel and palette

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Panel opens and closes | the popup's own placement; no app motion | n/a | no motion |
| Rows narrow as the user types | none: results update in place | n/a | no motion |

## Embeds and markdown

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Embed block loads its text | none: the block's text is read from the library in one frame, so a skeleton would show for no frame | n/a | no motion |

## Diagnostics and transfer

| Change | Motion | Reduced motion | Kind |
|---|---|---|---|
| Import or export in flight | none: the result line states the outcome when it arrives | n/a | no motion |
| Result line appears | the toast's own entrance and exit from `nsh/base` | the platform's | plays |

## Appearance, without pictures

Appearance is checked by the styles a screen resolves to, not by captured images. `tests/appearance.rs`
renders every screen in the light and the dark appearance and reads `screen().span_style(..)`: the same
text keeps its font size and weight, and its colour is the theme's for that appearance, so the two
differ. The library is read through `typr::shell::shell`, the same function the app draws it with, so
there is one library screen and the pass cannot be aimed at a copy. `tests/tokens.rs` refuses a
spacing, type, radius or colour literal in a screen.

The hover wash and the press scale are the exception that needs a picture, because the headless driver
keeps no view style and cannot take one. `desktop.test.tsx` hovers a button on the macOS host and asserts
the screenshot changed, and skips that step where the app opens as a phone. The half-point colours and
the state roles are unit-tested in `ui/button.rs`, where they need no reactive runtime.
