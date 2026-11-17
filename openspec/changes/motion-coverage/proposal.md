# Proposal — motion-coverage

## Why

`DESIGN.md` holds 26 motion rows: 6 play, 20 read "no motion". Three screens have **none at all** —
card editor (3 rows), history (2), keymap panel and palette (2) — and the design guides call a screen whose
every change is an instant swap unfinished.

The foundations for the missing movement already exist and are unused. In `crates/typr/src/ui/motion.rs`, these
tokens are defined and appear nowhere else in `crates/typr/src`:

| Token | Its stated job | Uses outside `motion.rs` |
|---|---|---|
| `RISE` (4 pt) | "how far a label rises when it changes" | 0 |
| `SLIDE` (8 pt) | "how far a popup slides toward its trigger" | 0 |
| `ENTER` (16 pt) | "how far content travels when it enters a page" | 0 |
| `ENTER_SCALE` (0.97) | "the scale a surface starts at when it enters" | 0 |
| `SLOWER` (500 ms) | "a list's rows arriving as a wave" | 0 |
| `PRESS_SCALE_ICON` (0.95) | the scale an icon button presses to | 0 |

So the app has the vocabulary for a rise, a slide, an entrance and a row wave, and the three screens that need
them choose "none" — while `RISE`'s own doc comment describes the exact change the card editor's saved/unsaved
label makes.

## What Changes

- Each of the three all-instant screens names the one change a reader would benefit from following, and plays it
  with the token already defined for it — or records a refusal with a reason the guide accepts.
- A motion token used by no screen is used or deleted: five of the six have a screen that wants them,
  `PRESS_SCALE_ICON` gets a use or goes.
- `DESIGN.md`'s rows for those screens state the tool, the token and what reduced motion does, in the form the
  guide's "Motion inventory" asks for.

## Capabilities

### New Capabilities

- `motion`: which state changes move, which token each uses, what reduced motion does to it, and that a token
  with no use does not exist.

### Modified Capabilities

None. `visual-language` already requires motion tokens to come from the theme; this change says which changes
use them.

## Impact

- `crates/typr/src/screens/keymap.rs` (the panel's entrance), `screens/history.rs` (a row's entrance),
  `screens/editor/mod.rs` (the saved/unsaved label).
- `crates/typr/src/ui/motion.rs`: `PRESS_SCALE_ICON` used or deleted.
- `DESIGN.md`: the three screens' rows rewritten.
- `crates/typr/tests/tokens.rs`: a case that every token in `ui/motion.rs` is named outside it.
