# Proposal

## Why

The store in `typr-core` already carries out every card and deck write, but several decisions about
those writes are still made in the desktop crate (`crates/typr/src/shell.rs`, `screens/sidebar.rs`,
`screens/editor`). Any other front end would have to make them again, and they can only be tested
through the slow headless shell. These decisions are:

- which deck a new card goes to
- which card is selected after a trash
- what makes a deck name acceptable
- what makes a card empty
- that a save also drops the stored draft

Some of those UI-only rules already let the store accept data the app would refuse:

- `CreateDeck` and `RenameDeck` store a name untrimmed, and blank names get through.
- `SaveCard` stores whitespace-only text.
- A save and its draft clear are two separate writes, so a failure between them leaves a stale draft
  that comes back on the next open.
- A new card made with no deck selected goes to the first deck in the snapshot, even when that deck
  is archived.

## What Changes

- **Card edits are decided in `typr-core`:**
  - A new card's deck is resolved in the backend: the selected deck when it is open, otherwise the
    first deck that is not archived. Today the first deck is used even when it is archived.
  - The card selected after a trash is computed in the backend from the shown list.
  - Whether a card has content is a backend rule. `SaveCard` refuses a text card whose text is
    empty or whitespace with a new store error, and the editor asks the same backend function
    before it enables Save.
  - Saving a card drops its stored draft in the same transaction. The editor's second
    `SetDraft { text: None }` write after a save goes away.
- **Deck edits are decided in `typr-core`:**
  - `CreateDeck` and `RenameDeck` trim the name and refuse a blank one with a store error.
  - The sidebar passes the typed name through unchanged. A blank name still creates nothing.
- **The desktop crate stops making these decisions:**
  - `shell.rs`'s `new_card` and `delete_card` closures and the sidebar's deck-name handling call the
    backend and only translate its answer into screen state: the route and the selection.
  - The single store writer (timing, slow-command log, re-publish, stoppage flag) stays in the
    desktop crate, because it feeds UI signals.

No new user-facing screens or controls. Renaming, archiving or deleting a deck from the UI is out of
scope: there is no UI for these yet, and the store commands already exist.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `project-structure`: adds a requirement that the rules behind creating, editing and deleting
  cards and decks live in the backend, and that the desktop crate only maps gestures onto them.
- `card-library`: adds requirements that a deck name is trimmed and cannot be blank, and that a
  new card with no deck selected goes to the first open deck, never an archived one.
- `card-editor`: adds requirements that the library refuses an empty text card from any caller, and
  that a save drops the stored draft in the same write.

## Impact

- **`crates/typr-core`:**
  - A new module for the card and deck edit rules, with its tests under the existing per-module
    test layout.
  - `store/db.rs`: `CreateDeck`, `RenameDeck` and `SaveCard` gain their checks, and `SaveCard`
    clears the draft.
  - `store/mod.rs`: two new `StoreError` variants.
- **`crates/typr`:**
  - `shell.rs`: the `new_card`, `delete_card`, new-deck and `on_save` paths.
  - `screens/sidebar.rs`: the name is no longer trimmed there.
  - `screens/editor/mod.rs`: `has_content` moves to the backend and is re-exported or called from
    there.
- **Tests:** the existing headless shell, sidebar and editor tests must keep passing unchanged.
  Backend tests are added for each moved rule.
- **Data:** no schema change. Rows already holding untrimmed or blank deck names are left as they
  are; only new writes are checked.
