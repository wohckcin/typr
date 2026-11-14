# Design — hidden text

## Context

`crates/typr-core/src/sides.rs` already splits a card on a `---` line that is outside a fence, and
`crates/typr-core/src/store/db.rs:392` `sync_prompt(card, sides)` inserts the single `fwd` prompt when
`sides >= 2`. Studyability is decided in the `studyable_prompt` view (migration V3,
`crates/typr-core/src/store/schema.rs:36`): `c.sides >= 2`, which is why a one-sided card never queues.
`Session::Entry` (`crates/typr-core/src/session.rs:11`) carries `card`, `sides: Vec<String>` and
`overdue`, and the session reveals those sides one at a time.

Two constraints from the current design matter here. A prompt is a row keyed `(card_id, key)`, so a
second variant is additive and needs no new table. And `review-scheduling` requires that a card which
becomes a note keeps its prompt but stops queueing, so studyability cannot be "a prompt row exists".

## Goals / Non-Goals

**Goals**

- Read hidden text out of markdown with the same fence awareness `sides.rs` already has.
- Give each group a prompt whose key is stable across edits, so editing a card never resets a group's
  schedule.
- Make one face able to unmask, without changing how sides reveal.
- Leave every card without hidden text behaving exactly as it does today.

**Non-Goals**

- Typed hidden-text answers (Mochi's `Type hidden text` preference). A later change, once the mask is
  in place.
- Diagram masks, reverse reviews, template fields.
- Any change to the scheduler, the queues, or the cap.

## Decisions

**Hidden text wins over sides.** If a card holds hidden text, the separator stops splitting it and
there is no `fwd` prompt. Two independent multi-prompt schemes on one card would multiply the review
surface (sides × groups) and no Mochi screen shows that, so the parser reports one shape per card:
`Shape::Sides(Vec<String>)` or `Shape::Hidden { faces, groups }`.

**Unnumbered spans form one group keyed `cloze:0`.** Mochi's simple form masks every unnumbered span
at once, so one prompt hides them together; numbering is how a card asks for separate reviews. Group
numbers come from the text and map straight to `cloze:<n>`, which keeps the key stable: inserting a
new group does not renumber the others, and deleting one leaves its history as history.

**Studyability becomes `sides >= 2 OR has_cloze`.** Migration V4 adds `card.has_cloze INTEGER NOT NULL
DEFAULT 0`, kept in step by `sync_prompts`, and recreates the view with
`AND (c.sides >= 2 OR c.has_cloze = 1)`. The alternative, "a prompt row exists", would resurrect the
old bug where a card turned into a note kept queueing.

**A face carries `hidden` and `shown`.** `Entry` becomes `{ card, prompt, faces: Vec<Face>, overdue }`
with `Face { hidden: String, shown: String }`. For sides the two strings are equal; for a group they
differ only inside the masked span. Reveal swaps in `shown`, so `revealed`, `read_only` and the grade
gate stay exactly as they are and every existing session test keeps its meaning.

**The mask is `[ … ]`, keeping the span's word count.** A bare `____` loses length cues in languages
where that is a hint, and repeating the answer's length leaks it. `[ … ]` marks one span, and the
preview labels its group: `[ … ]` plus a caption listing which groups the card holds.

**Parse once, carry the result.** `Shape` is computed when a card is saved and when a session is
built, never per frame: `sync_prompts` needs it for keys, the session needs it for faces, and the
editor needs it for the preview.

## Risks / Trade-offs

- Existing libraries gain a column and a new view definition; the migration must be tested against a
  pre-V4 file, not only a fresh one.
- A user who types `{{ }}` in prose to mean braces now loses a prompt to a group they did not intend.
  An unclosed brace stays text, and the preview shows the mask before saving, which is the mitigation.
- `cloze:<n>` numbering is user-visible in history and in the data file. Renaming a group drops its
  schedule; the alternative, a surrogate id, would need a mapping table for a feature nobody asked for.
- A card whose group is deleted keeps that prompt row (history is preserved by design), so the review
  count can exceed the groups on screen. `reset_history` remains the way out, and the diagnostics work
  already treats orphaned prompts as acceptable.
