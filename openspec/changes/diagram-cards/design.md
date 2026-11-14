# Design — diagram cards

## Context

`attachments` (`crates/typr/src/attachments.rs`) already copies a chosen file under the data folder and
resolves a name to a path plus an `Image` through `ImageCache`, refusing absolute paths and anything
that escapes the root. Markdown images use it through `Markdown::images(..)`.

A card today is markdown plus `sides`, and `studyable_prompt` admits it when `c.sides >= 2`. Prompts
are rows keyed `(card_id, key)`. Reviewing happens through `Session`, whose faces are strings.

Rune gives the three primitives a mask editor needs: `El::on_drag` hands a gesture's local points and
its spanning rect (`Drag { phase, x, y, w, h, start_x, start_y }`, phases `Down`/`Move`/`Up`),
`El::on_layout` reports an element's `[x, y, w, h]`, and `El::absolute(left, top)` plus `El::pin(..)`
place out-of-flow children, which is how a covering panel sits over part of an image.

## Goals / Non-Goals

**Goals**

- A second card kind whose reviews are regions of an image.
- Rectangles stored resolution-independently, so a scene render and a large window agree.
- Authoring inside typr: choose a file, drag regions, group them, merge, delete.
- Prompts, queues, grades and history reused without new scheduling code.

**Non-Goals**

- Rotating, cropping or zooming the image, and free-form masks (only rectangles).
- Occlusion-style cards where the answer is another image.
- Importing Mochi's diagram cards, which waits for `mochi-import-export`.

## Decisions

**A mask group is a prompt, a mask is a row.** `mask(card_id, group_no, x, y, w, h)` with fractions in
`[0, 1]` keeps one row per rectangle and one prompt per group, so two rectangles in group 1 share one
schedule while remaining independently editable. Storing the group on the prompt instead would force a
prompt rewrite whenever a rectangle is dragged into another group.

**Fractions, not pixels.** The same card renders at 400 px in a list and 900 px in a review; a pixel
rectangle would drift between them and between the controller scene renders and the window. Fractions
also survive a future zoom control.

**Diagram is a card kind, not a separate table for content.** `card.kind` with `card.image` and
`card.caption` keeps every existing relation (deck, tags, trash, prompts, history, drafts) working
unchanged; a parallel `diagram` table would need its own copies of all of them.

**A face becomes an enum: text or image.** `Face::Text { hidden, shown }` is the face `cloze-deletions`
introduces, and `Face::Image { image, caption, masks, revealed }` is what a mask group needs. The
session keeps counting faces, so reveal, `read_only`, the grade gate and reshuffle are untouched.

**Covering is a filled rectangle, not a re-rendered image.** Drawing an opaque panel over each masked
region costs two elements per rectangle and cannot corrupt the picture; cutting the pixels would need
an image pipeline for no visible difference.

**Authoring geometry comes from `on_layout` of the image pane.** The drag's points are already in the
pane's own coordinates, so the fraction is `x / pane_w`, and the pane's width is the measured value
rather than a constant. Without that, a window resize would change the stored fractions.

**Group numbers are permanent and user-visible.** `mask:<n>` mirrors `cloze:<n>`, for the same reason:
renaming a group drops its schedule, and a surrogate id would need a mapping table nobody asked for.

## Risks / Trade-offs

- The view grows a third studyability branch and a join to `mask`; the agreement test between the view
  and `Snapshot` is what will catch a divergence.
- Dragging is the only authoring gesture, so a user without a pointer cannot create a mask. The task
  list requires a keyboard fallback that adds a rectangle at the centre of the image, sized by arrow
  keys, because a card type that is mouse-only would regress the app's keyboard posture.
- A diagram card whose image file is missing must render a placeholder and stay reviewable; the
  alternative, refusing the review, strands a real card behind a deleted file.
- Old libraries migrate as `text`; nothing else changes, but a `card.kind` default of `text` must be
  written by the migration, not assumed at read time.
