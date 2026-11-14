# Design: card embeds

## Context

`markdown-coverage` introduces `crates/typr-core/src/markdown_ext.rs` with
`preprocess(text) -> Cow<str>` that tracks fenced blocks and code spans, because
pulldown-cmark 0.13.6 has no embed syntax. `card-names-and-references` introduces card
names and `[[references]]`. `Snapshot` already carries every card's markdown, so
resolution needs no query.

## Goals / Non-Goals

Goals: `![[Name]]` renders another card's body, marks a missing target, and never
recurses. Non-Goals: editing the embedded card from the embed, backlinks for embeds
(`card-names-and-references` owns links), embeds across libraries, and rendering embeds
inside code spans or fences (both are literal text).

## Decisions

**The pre-pass rewrites an embed into a marker the renderer understands.** `preprocess`
replaces `![[Name]]` (outside fences and code spans) with a fenced block whose info string
is `typr-embed` and whose body is the name. The renderer, which already walks fenced
blocks for syntax colouring, sees `typr-embed` and asks `fn resolve(name) -> Option<&str>`
for the body. That keeps pulldown-cmark untouched and gives one place where an embed can
be recognised without a second parser.

**Resolution happens where the snapshot is available, never in core.** `core` knows how to
find an embed (`pub fn embeds(text: &str) -> Vec<String>`), not what a card is; the desktop
crate resolves names against `Snapshot` and passes the result into the renderer. That keeps
`typr-core` free of the render tree, per `project-structure`.

**An embed renders as an inset block, not as a link.** The body is drawn inside a bordered
block with the theme's secondary surface and a small caption naming the source card. A
missing target draws the name in the danger tone with the caption "no such card".

**One level, then a marker.** The renderer carries a depth of one; an embed met at depth
one draws the marker `Name (embedded card)` instead of its body. Self-embedding is the
depth-one case with the same card, so one rule covers both.

**Embeds are text, so nothing migrates.** No column, no migration, and `sides` is computed
after the pre-pass so an embed containing `---` cannot add a side — the pre-pass replaces
the embed with a single opaque block first.

## Risks / Trade-offs

- A renamed card breaks embeds silently unless the missing marker is loud; the marker is
  the mitigation and `card-names-and-references` will own rename repair.
- Rendering an embedded card's body multiplies the text on screen; the inset style and the
  caption keep the boundary visible.
- Deeply nested embeds are impossible by construction, which is a limit users may ask to
  lift later.

## Migration Plan

Text only. Reverting leaves `![[Name]]` rendering literally, which is what pulldown-cmark
already does.

## Open Questions

- Whether the editor should offer completion for `![[`.
- Whether an embed should be clickable once a card can be opened from a link
  (`card-names-and-references`).
