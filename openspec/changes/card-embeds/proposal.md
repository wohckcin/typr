# Proposal: embed one card inside another

## Why

The exploration of https://app.mochi.cards/ found `![[name]]` transclusion in the card
editor: a card's content can be pulled into another card's body. Nothing in the 14
changes proposed so far covers it, and `[[references]]` alone is not the same thing — a
reference links, an embed shows.

## What Changes

- A card body may embed another card with `![[Name]]`, and the editor preview and the
  review screen render that card's content where the embed sits.
- An embed of a card that does not exist renders the name in a way that marks it as
  missing, instead of breaking the card or turning it into a link.
- An embed creates no prompt for either card and moves no schedule.
- A card that embeds itself, or an embed chain, stops at one level and draws a marker
  instead of recursing.

## New Capabilities

- `card-embeds`: transclusion between cards.

## Modified Capabilities

None.

## Impact

- `crates/typr-core/src/markdown_ext.rs` (the pre-pass `markdown-coverage` introduces),
  `crates/typr/src/editor.rs` and the review face renderer, `crates/typr/src/library.rs`
  (resolution against the snapshot), tests.
- No schema change; embeds are text.
