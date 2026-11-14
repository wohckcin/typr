# Design — restore an archived deck

## Context

The app archives and restores decks already. `Command::ArchiveDeck { id, archived }` is one store
command for both directions, `Host::archive_deck` / `Host::unarchive_deck` in the desktop crate send
it, and `a_card_under_an_archived_deck_is_reachable_and_the_deck_can_come_back` exercises the round
trip. The gap is in the spec, not the code: `card-library` requires archiving a deck and restoring a
card, and says nothing about restoring a deck.

## Goals / Non-Goals

**Goals**

- Make the requirement match what a user can do: an archived deck is visible in the archived view and
  can be brought back.
- Say what restoration preserves, so a future change cannot quietly drop a schedule while it tidies a
  deck.

**Non-Goals**

- No new behaviour, screen, command or column. Nothing in the store changes.
- No change to card archiving or to the trash.

## Decisions

**Restoration is stated as part of the archiving requirement, not as a new requirement.** Archiving a
deck and restoring it are one operation with one flag, and a separate requirement would repeat the
"does not delete anything" guarantee in a second place. The existing scenario keeps its job, and one
more scenario states the deck round trip.

**The scenario asserts the schedule survives.** Restoring a deck touches `deck.archived` only, so a
due date is the observable proof that nothing else moved. The test grades the card before archiving
and compares the stored due date afterwards.

## Risks / Trade-offs

- The scenario names an archived view and a restore action that already exist, so it cannot fail
  today; its job is to fail when someone removes one of them.
- A reader could take "the cards it holds return to the views and queues they belong in" as a promise
  about newly created decks. It is not: it describes the cards as they were.

## Migration

None. Spec text only.
