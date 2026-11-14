# Proposal — diagram cards

## Why

Mochi has a card type built from an image: you load a diagram, drag rectangles over the parts you want
to be asked about, and each rectangle, or each merged group of rectangles, becomes its own review with
its own history. Anatomy plates, network topologies, circuit symbols and maps are all studied this way.

typr can show an image inside markdown and copies attachments into its data folder, but a card is only
text. There is no way to ask "which artery is this", no way to hide part of a picture, and no way to
author one without editing the picture in another program.

## What Changes

- A card gains a kind: `text` (what exists today) or `diagram` (an image, an optional caption, and a
  set of masks).
- A mask is a rectangle stored as fractions of the image, so it holds at any display size. Masks carry
  a group number, and a group is one review.
- The editor gains a diagram mode: choose an image, drag over it to add rectangles, assign or merge
  them into groups, and write a caption.
- Review shows the image with the reviewed group's rectangles covered, and one reveal uncovers them.
- Each mask group is a prompt keyed `mask:<n>`, scheduled independently, exactly like a cloze group.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. The additions sit beside the live text, for the same reason as `review-reverse`: an open change
already modifies the prompts and session-reveal requirements, and two changes cannot both archive a
rewrite of one block.

## Impact

- `crates/typr-core/src/store/schema.rs`: migration adds `card.kind`, `card.image`, `card.caption`
  and a `mask` table, and extends `studyable_prompt` so a diagram card queues when it has a group.
- `crates/typr-core/src/store/{mod,db}.rs`: `CardSummary` grows `kind` and `mask_groups`; commands add
  and remove masks and set the group of one; `sync_prompts` inserts `mask:<n>` per group.
- `crates/typr-core/src/session.rs`: a face becomes either text or an image with rectangles.
- `crates/typr/src/editor.rs`: a diagram pane with drag-to-add, group assignment and merge.
- `crates/typr/src/attachments.rs`: already copies and resolves files; the diagram image reuses it.
- No dependency changes. Existing cards migrate as `text` and keep their schedules.
