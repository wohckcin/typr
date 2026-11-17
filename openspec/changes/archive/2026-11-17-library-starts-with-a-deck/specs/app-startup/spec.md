## ADDED Requirements

### Requirement: A library with no deck is given one

The app SHALL give a library that holds no deck one deck when it opens, so the library's empty screen has
somewhere to put the card its action makes, and SHALL NOT add another deck to a library that already holds one.

#### Scenario: The library does not exist yet

- **WHEN** the app opens on a data folder that holds no database
- **THEN** the sidebar lists one deck
- **AND** the grid states the deck holds no cards and offers the action that makes one

#### Scenario: The empty deck's action

- **WHEN** the deck holds no cards and the user presses the action the empty state offers
- **THEN** a card is created in that deck and its editor opens
- **AND** the card is in the library the next time it is opened

#### Scenario: A library that already holds a deck

- **WHEN** the app opens on a library that holds a deck, archived ones included
- **THEN** no deck is written
- **AND** the sidebar lists the decks the library already had
