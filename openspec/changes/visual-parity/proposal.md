# Proposal — visual parity with Mochi

## Why

The clone is now functionally close: sides, hidden text, masks, templates, views, cramming, names,
references, FSRS, import. It looks like a toolkit demo. Every screen is a stack of default-styled rows
with 44 px of height, one type size, no surface treatment, and no empty state, so a Mochi user opening it
sees a different application even where it does the same thing.

Parity is not a coat of paint: it is the four things that make Mochi readable at a glance — a card that
looks like an object, a review canvas that holds one thing, a sidebar that groups what you have, and
typography that separates a question from its answer.

## What Changes

- A visual language module: spacing scale, type scale, radii, elevation, focus ring, and one card
  surface, all derived from the kit's tokens rather than literals.
- The review surface becomes a centred card on a dim canvas, with the answer separated from the question
  by space and type weight, not by a border.
- The library becomes a list of card objects with a name, a preview line and its metadata; the sidebar
  groups Decks, Tags and Views under headings, with counts.
- The editor gains a toolbar for the actions that are currently loose buttons, and an inline preview that
  shows the masked form.
- Every screen gets an empty state that says what to do next.
- A snapshot baseline per scene is committed and compared in CI, so a visual regression is a failing
  test rather than a surprise.

## Capabilities

### New Capabilities

- `visual-language`: the tokens every screen draws with, the shape of a card, the shape of the review
  surface, and the visual baseline that enforces them.

### Modified Capabilities

None.

## Impact

- New `crates/typr/src/design.rs`: the token set and the constructors every screen uses (`card_surface`,
  `page_header`, `empty_state`, `review_canvas`).
- `crates/typr/src/{library,editor,study,history,settings}.rs`: rebuilt on those constructors; no
  behaviour changes.
- New `crates/typr/tests/visual.rs` and `crates/typr/tests/baselines/*.png`: per-scene baselines compared
  by pixel difference with a stated tolerance.
- `.github/workflows/ci.yml`: the baseline comparison joins the scene loop.
- No dependency changes; the kit already provides the tokens, radii and shadows.
