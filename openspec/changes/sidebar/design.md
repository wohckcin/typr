# Design: sidebar as the app's navigator

## Context

Measured on https://app.mochi.cards/ at a 1485-point window, dark theme: the sidebar is
225 points wide on `rgb(36,36,38)` with no dividing border; a row is 211 points wide at
7 points of inset, 25 points tall, `2px 12px` padding, 5-point radius, label 13/500 in
60% white starting at x=53 (a 20-point icon column and a 14-point gap); the search row
is full-bleed at 28 points tall with a keycap at x=178; the account row is 53 points
tall with a second line coloured `rgb(225,98,89)` when offline; a section label is
11/700 uppercase at 30% white inside a 27-point band inset 14; a count badge is 18x16 at
x=188, 12/600, 8-point radius, `rgba(255,255,255,0.14)`; groups are separated by about
20 points and rows stack with no vertical gap; the only state rule is
`:active:not(.no-active) { background-color: var(--fn7) }`, so state travels through
per-element CSS variables.

typr today: `fn deck_sidebar<S: Host>` in `crates/typr/src/library.rs:467` builds
`col().key("decks").w(200.0).padding(8.0).gap(2.0).fill(t.card)`; `fn deck_row` at
`crates/typr/src/library.rs:400` indents with `let inset = 10.0 + depth as f32 * 12.0;`
and `.px(inset)`; rename, move and archive are wired from the same rows. The project
already owns the tools this needs: `rune_kit::text_input`, `rune_kit::Button`,
`rune_kit::Kit`, and `rune::El::on_press`.

## Goals / Non-Goals

Goals: groups, an icon column, per-deck due counts, collapse, an active fill, a chord on
the search row, and a state row. No new dependency, no schema change.

Non-Goals: resize, drag to reorder, emoji or per-deck icon authoring, a second-level
hover flyout, badges for new cards distinct from due cards, and persisting collapse
across launches.

## Decisions

**The sidebar is its own module, `crates/typr/src/sidebar.rs`.** `library.rs` is 695
lines and holds the list, the filters and the sidebar. The module takes the snapshot and
the sidebar state and returns `El`; `library::view` composes it. The deck list's
interaction callbacks still call `library::Host`, so nothing about rename/move/archive
changes.

**Groups are data.** `pub enum Group { Navigation, Decks, Utilities }` and
`fn group_of(row) -> Group` order the rows; a group with no rows is skipped. That is what
makes the tag section disappear while the library has no tags, which is measured
behaviour, not a preference.

**Geometry constants carry the measurements.** `const WIDTH: f32 = 225.0;`,
`const ROW_HEIGHT: f32 = 25.0;`, `const INSET: f32 = 7.0;`, `const ICON: f32 = 20.0;`,
`const GAP_AFTER_ICON: f32 = 14.0;`, `const GROUP_GAP: f32 = 20.0;`. typr's 200 points
become 225 so the deck names have the width Mochi gives them.

**Icons are a coloured monogram, not an image set.** typr ships no icon assets and adding
an icon font to a two-crate workspace for seven glyphs is not worth the weight. `fn icon(kind, name) -> El`
draws a 20-point box holding either a 1-2 letter monogram or a geometric mark from the
theme's glyph set, tinted from a hash of the deck name, so two decks are distinguishable
at a glance. The requirement only promises an aligned label column, so this stays a
reversible visual decision.

**Due counts come from one query, not from the snapshot.** The snapshot's `CardSummary`
carries no schedule (a deliberate design decision), so the sidebar cannot count from it.
Add `Command::DueCounts { now_ms, offset_ms }` returning `Vec<(Id, u32)>` for every deck
with at least one due prompt, computed over the existing `studyable_prompt` view and a
`WITH RECURSIVE subtree(id)` CTE — the same shape `due_prompts` already uses with its
`deck` argument. `State` refreshes the counts when it refreshes the snapshot, so a row's
number is as fresh as the list it labels. A count of zero renders nothing, per the
requirement.

**Collapse is view state on `Library`.** `pub collapsed: HashSet<Id>`, and
`fn deck_rows(&self) -> Vec<(Deck, usize)>` walks the tree with the existing
`fn deck_tree` order while skipping descendants of a collapsed deck. Not persisted: the
requirement says so, because persisting it needs a settings read at startup and this
change is about the column's structure. A file a user closed is a one-frame decision
they can redo; the schema does not need a column for it yet.

**The active fill is the theme's selection surface, not a new token.** `tok::theme(dark)`
already carries row fills; the row uses the selected surface with the label at full
contrast. The move-destination tint (`tok::accent(1, true).alpha(0.35)`) keeps its
meaning, so a row can be "selected" or "a drop target" without ambiguity.

**The search chord is looked up.** `sidebar::search_chord(keymap) -> Chord` reads the
binding `keymap-panel` introduces. Until that change lands, the module draws a constant
and a test fails if the two disagree — the honest order is `keymap-panel` first; this
change declares the dependency rather than duplicating the chord.

**The state row states the library, not the account.** Mochi's row is an account row
because Mochi is a synced service. typr has no account; the equivalent fact is whether
the library opened, so the row states that and takes the danger tone when it did not.

## Risks / Trade-offs

- A due count per deck means one query per snapshot refresh; on a 5000-card library the
  subtree CTE runs once, not per deck, so the cost is one indexed scan.
- Monogram icons are less legible than emoji; the label column still aligns, and swapping
  in authored icons later touches one function.
- Collapse is lost on quit; a user who folded a large tree rediscovers it once per launch.
- 225 points is 25 wider than today, which narrows the card area by that much.

## Migration Plan

No schema change. Ships as `crates/typr/src/sidebar.rs` plus the query, behind the same
screen; a scene per state proves the layout. Reverting means deleting the module and
restoring `deck_sidebar` from `git`.

## Open Questions

- Whether counts should include new cards next to due cards (`1 · 3 new`), which needs a
  second number per row and a rule for which is louder.
- Whether collapsing should persist once `settings-screen` exists, which owns a settings
  surface.
