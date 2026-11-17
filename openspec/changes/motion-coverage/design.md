# Design — motion-coverage

## Context

**The vocabulary.** `crates/typr/src/ui/motion.rs` defines five durations (`INSTANT` 120, `FAST` 180, `BASE` 240,
`SLOW` 360, `SLOWER` 500) and five distances/scales (`RISE` 4, `SLIDE` 8, `ENTER` 16, `PRESS_SCALE` 0.97,
`PRESS_SCALE_ICON` 0.95, `ENTER_SCALE` 0.97). Used today: `INSTANT` (2 sites), `FAST` (2), `BASE` (1), `SLOW` (1),
`PRESS_SCALE` (`ui/button.rs:152`, `:195`). Unused: `RISE`, `SLIDE`, `ENTER`, `ENTER_SCALE`, `SLOWER`,
`PRESS_SCALE_ICON` — verified with `grep -rnw <token> crates/typr/src --include=*.rs | grep -v ui/motion.rs`.

**The three screens.** `DESIGN.md` records zero motion for:

- Card editor — "Saved / Unsaved badge: none: the word changes in place; a glyph swap would animate a two-word
  status"; picture attached: none; typing: none.
- History — "Row arrives: none: rows are read in one frame"; filter change: none.
- Keymap panel and palette — "Panel opens and closes: the popup's own placement; no app motion"; "Rows narrow as
  the user types: none".

**The guide's rules that apply.** Motion tokens belong beside the theme and the controls use them; a screen whose
every change is an instant swap is unfinished; each inventory row names the tool and the token, or says no motion
and why; reduced motion is honoured per row.

The three screens' current reasons are not all wrong — the editor's rejects a *glyph swap*, and the palette's
rejects animating *each keystroke's* filter — but neither rejects a rise or an entrance, which is what the unused
tokens do.

## Goals

- Each of the three screens gains one movement, at the change a reader is most likely to miss without it.
- No new token and no new timing is invented: the existing tokens are used as their doc comments describe.
- The inventory rows and the code agree; a token with no use is deleted.
- Reduced motion removes the movement and keeps the frame the movement was explaining.

## Non-Goals

- Animating per-keystroke updates (typing, the palette's filter, the history's filter): the inventory's reasons
  stand for those rows.
- A new motion system, springs, gestures or shaders.
- Reviewing the four screens that already play one movement.

## Decisions

1. **Keymap panel and palette: enter from the trigger.** The panel arrives with `SLIDE` (8 pt toward the trigger)
   and `ENTER_SCALE` over `SLOW`, which is what those two tokens describe and what the guide calls a popup's
   entrance. The row's "the popup's own placement; no app motion" is replaced. Filtering stays instant: a wave
   per keystroke is noise, and the row keeps that reason.
2. **History: an arriving record enters.** A record added by a review takes `ENTER` over `SLOWER` at the top of
   the list; the existing list does not move. "Rows are read in one frame" is true of the first read and false of
   the row that appears while the reader is looking. The filter change keeps its reason.
3. **Card editor: the saved/unsaved word rises.** `RISE` over `FAST` as the word changes — exactly `RISE`'s doc
   comment ("how far a label rises when it changes"). No glyph swap, so the existing reason's objection does not
   apply; the row is rewritten to say what plays and what the glyph swap's objection covered. Typing stays
   instant.
4. **`PRESS_SCALE_ICON` is used or deleted.** No icon button exists today; the smallest honest answer is deletion
   in this change, and the token returns with the first icon button that needs it. (If the operator prefers, an
   icon button that exists today adopts it — task 4.2 names both.)
5. **A test keeps the vocabulary honest.** `tests/tokens.rs` gains a case that every `pub const` in `ui/motion.rs`
   is named outside that file, so the next unused token fails a gate instead of sitting in a foundations module.

## Reduced motion

Every added movement is removed under reduced motion, and the state change stays: the panel appears placed, the
record appears at the top, the word changes in place. The inventory rows state that per row, as the existing
`plays` rows do.
