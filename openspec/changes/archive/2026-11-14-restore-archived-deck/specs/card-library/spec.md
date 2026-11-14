# Spec Delta — card-library

## MODIFIED Requirements

### Requirement: Archiving removes a card from study without deleting it

The system SHALL allow a card or a whole deck to be archived. An archived card SHALL NOT appear in
any study queue, SHALL retain its content and review history, and SHALL remain visible in an archived
view from which it can be restored. A deck SHALL be restorable from the same archived view, and
restoring it SHALL return the cards it holds to the views and queues they belong in without changing
their scheduling state.

#### Scenario: Archived card leaves the due queue

- **WHEN** a card that is due today is archived
- **THEN** it is omitted from due today and its due date is preserved for the day it is restored

#### Scenario: A card under an archived deck is reachable and the deck comes back

- **WHEN** a deck holding a scheduled card is archived, and the user then restores that deck
- **THEN** the card is visible in the archived view while the deck is archived, and after the deck is
  restored the card is in the active view again with its due date unchanged
