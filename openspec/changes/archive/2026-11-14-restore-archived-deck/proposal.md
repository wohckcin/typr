# Proposal — restore an archived deck

## Why

`card-library` requires that a whole deck can be archived, and the app archives one. It also requires
that an archived **card** stay visible and restorable. Nothing in the spec says a deck can be brought
back, yet the app offers exactly that: the review that closed finding 13 added an `Unarchive` action,
because an archived deck's cards were reachable in the archived view and could be restored
individually, but the deck itself could not. A behaviour a user can reach and the spec cannot describe
is a requirement that has not been written down.

## What Changes

- The archiving requirement gains the deck half of restoration: an archived deck stays visible in the
  archived view and can be restored, with the cards it holds and their scheduling state unchanged.
- One scenario states what restoration preserves.

No code changes. The behaviour shipped in the fix for finding 13 (`d0e1b09`, `unarchive-deck`) and is
held by `a_card_under_an_archived_deck_is_reachable_and_the_deck_can_come_back`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `card-library`: the requirement "Archiving removes a card from study without deleting it" is
  extended to cover restoring an archived deck. Restoring a card is unchanged.

## Impact

- Spec text only: `openspec/specs/card-library/spec.md` once this change is archived.
- No code, API, dependency or data-format change. The store already accepts
  `Command::ArchiveDeck { archived: false }`, and the desktop crate already sends it from
  `Host::unarchive_deck`.
