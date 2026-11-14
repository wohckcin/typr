# Design — custom views

## Context

`Snapshot::cards_matching(query, tag, deck, mode)` is the one filter the library uses; the deck tree,
the search box, the tag control and the three modes all feed it. `Snapshot::subtree` gives a deck's
cards including descendants, and `folded` is applied once into `search_text` at write time so filtering
is a substring test.

`Snapshot` is already published to the views as an `Arc` and republished on writes only, so adding the
view list to it costs nothing per frame.

## Goals / Non-Goals

**Goals**

- Save exactly the state the library already has, plus a sort, and restore it.
- One filter type used by the sidebar, the list and any later consumer (cramming, export).
- Ordering that never touches scheduling.

**Non-Goals**

- Starting a study session from a view; that is `cramming`, next on the roadmap.
- Sorting by a field name (templates arrive before this change, but field ordering is not asked for).
- Sharing a view, or exporting it.

## Decisions

**`ViewSpec` lives in core and replaces the loose arguments.** One struct
`ViewSpec { deck: Option<Id>, query: String, tags: Vec<String>, mode: CardMode, pinned_only: bool, sort:
Sort }` with `Sort` a small enum keeps the filter honest: a saved view, the live sidebar state and a
test all build the same value, and adding a filter means adding a field rather than a parameter.

**Views are rows, not settings blobs.** `view(id, name UNIQUE, deck, query, tags, mode, pinned_only,
sort_key, sort_dir)` gives stable ids for selection and a natural place for `cramming` to reference.
Tags are stored joined by a separator chosen so it cannot appear in a tag, since the tag set is already
normalised to first-casing, comma-free words.

**Sorts run in the library over the snapshot, not in SQL.** The list is already fully in memory as
`Arc<Snapshot>` and sorts on a few thousand rows are microseconds; doing it in SQL would put the sort in
the store's queue and make the list wait a round trip for every sort click, and the due date it needs
is a snapshot field already.

**A shuffle is not a saved sort.** Mochi treats cramming and shuffling as moments, not settings, and a
saved random order would mean a list that reshuffles on every restart. `Sort::Shuffled` therefore lives
in the library's transient state, and selecting a view clears it.

**"Due date" sorts by the earliest prompt of the card, cards without prompts last.** That is the
reading a user expects from a card list, and it reuses `prompt.due_at` values already carried in
`CardSummary`-adjacent queries; no new index is needed because the sort happens in Rust.

**Pinned is a column on `card`, not a view.** A pin is a property of the card, so it survives deletion of
whichever view happened to show it, and it survives the trash and the archive as the spec requires.

## Risks / Trade-offs

- `cards_matching`'s signature change touches every caller and test; that is a mechanical sweep, and the
  compile error is the checklist.
- A view's deck scope is an id, so deleting a deck leaves a view pointing at a missing deck. The rule
  chosen: a view whose deck is gone falls back to the whole library and says so, rather than going
  empty, because an empty list reads as data loss.
- Sorting by "first side" needs the preview text, which `CardSummary` carries; it is truncated at eighty
  characters, so two cards whose previews share a prefix order by the remainder. Acceptable for a list
  order, and stated rather than hidden.
