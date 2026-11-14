# card-library Specification

## Purpose
Owns the user's durable study material — the decks, cards and tags that exist independently of any review — and the operations that organize, find and retire it.

## Requirements

### Requirement: Decks form a hierarchy

The system SHALL store decks in a parent/child hierarchy with one root deck that has no parent, and SHALL allow any deck to nest under any other deck without a depth limit. Two decks under the same parent SHALL NOT share a name, and decks under different parents MAY share one.

#### Scenario: Creating a nested deck

- **WHEN** the user creates a deck named "Verbs" inside a deck named "Spanish"
- **THEN** "Verbs" appears as a child of "Spanish" and its cards are not listed under "Spanish" directly

#### Scenario: Duplicate names under one parent are refused

- **WHEN** the user creates a second deck named "Verbs" under "Spanish"
- **THEN** the creation is refused and the hierarchy is unchanged

#### Scenario: Rejecting a cycle

- **WHEN** the user moves a deck into one of its own descendants
- **THEN** the move is refused and the hierarchy is unchanged

### Requirement: Every card belongs to exactly one deck

The system SHALL require each card to belong to one deck at creation and SHALL move the card, together with its review history, when it is reassigned. Moving a card SHALL NOT reset its scheduling state.

#### Scenario: Moving a card between decks

- **WHEN** the user moves a card that has been reviewed from "Spanish" to "Verbs"
- **THEN** the card appears in "Verbs" and its due date is unchanged

### Requirement: Studyable cards exclude archived and trashed material

The system SHALL treat a card as studyable only when the card is not archived, not in the trash, and no deck holding it — its own deck or any ancestor — is archived. Study queues SHALL contain only studyable cards, and the card list SHALL show a trashed card only in the trash view.

#### Scenario: Archiving a deck removes its cards from study

- **WHEN** a deck holding a card that is due today is archived
- **THEN** that card is absent from the due queue and its due date is preserved

#### Scenario: A trashed card is not studyable

- **WHEN** a card is moved to the trash
- **THEN** it appears in no study queue and not in its deck's card list

### Requirement: Cards are listed with their content searchable

The system SHALL list the cards of a selected deck and its descendants and SHALL provide a text search over card content and tags. Search SHALL match case-insensitively and SHALL ignore diacritics, so that "cancion" finds "canción".

#### Scenario: Searching ignores accents and case

- **WHEN** the user searches "cancion" and a card contains "Canción"
- **THEN** that card appears in the results

#### Scenario: Search scope follows the selected deck

- **WHEN** the user searches while a deck is selected
- **THEN** results are limited to cards in that deck and its descendants

### Requirement: Tags classify cards

The system SHALL support zero or more tags per card, SHALL allow filtering the card list by tag, and SHALL treat tags as case-insensitive for matching while preserving the casing first used for that tag across the library. Tags SHALL be set on the card rather than parsed out of its text, so `#` in a heading or inside a side is content.

#### Scenario: Filtering by tag

- **WHEN** the user filters by "grammar" and a card is tagged "Grammar"
- **THEN** that card appears in the filtered list

#### Scenario: A heading is not a tag

- **WHEN** a card's text starts with `# Grammar`
- **THEN** the card has no tag and is not matched by a "grammar" tag filter

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

### Requirement: Trash is reversible until emptied

The system SHALL move deleted cards to a trash area rather than destroying them, SHALL allow restoring a card from trash with its review history intact, and SHALL destroy trashed cards, with their review history, only when the user explicitly empties the trash.

#### Scenario: Restoring from trash

- **WHEN** the user deletes a card and later restores it
- **THEN** the card returns to its original deck with its scheduling state and history unchanged

#### Scenario: Emptying trash is irreversible

- **WHEN** the user empties the trash
- **THEN** the trashed cards and their review history are gone and cannot be restored

### Requirement: Library data survives restarts in one local database

The system SHALL persist all decks, cards, tags, settings and review history in one SQLite database in the platform's application data directory for this app, SHALL create that database on first launch, and SHALL make every saved change durable with no further step.

#### Scenario: Restart preserves edits

- **WHEN** the user creates a card, closes the app, and launches it again
- **THEN** the card is present with the same content, deck and scheduling state

#### Scenario: Unreadable store is reported, not silently discarded

- **WHEN** the store file exists but cannot be opened or is not a database this app wrote
- **THEN** the app reports the failure with the file path and does not overwrite it

### Requirement: The library says which direction a card is waiting on

The system SHALL show, for each card that is enabled in reverse, that both directions are in play, and
SHALL offer a control that toggles reverse for that one card without opening it.

#### Scenario: A card shows its directions

- **WHEN** a two-sided card is enabled in reverse and its tile is drawn
- **THEN** the tile says the card is asked in both directions, and a card without reverse does not

#### Scenario: The tile toggles one card

- **WHEN** the user toggles reverse off from the card's tile
- **THEN** that card stops queueing in reverse and its deck setting is unchanged

### Requirement: A deck carries its own review settings

The system SHALL let a deck enable or disable reverse for itself, SHALL let a deck inherit the setting
from its parent, and SHALL show which of the three states a deck is in.

#### Scenario: A deck is switched to inherit

- **WHEN** a deck that had reverse enabled is set back to inherit while its parent has reverse off
- **THEN** the deck's cards stop queueing in reverse and the deck reports that it inherits

#### Scenario: The library default is the last word

- **WHEN** the library default is switched on and no deck or card in the library sets a value
- **THEN** every card that carries a prompt queues in reverse

### Requirement: A deck carries a default template

The system SHALL let a deck hold a template that its cards inherit, SHALL let a card override it with
its own, SHALL let either be cleared, and SHALL show on the card which template is in force.

#### Scenario: A card inherits its deck's template

- **WHEN** a deck's template names `<<Word>>` and `<<Meaning>>` and a card in it has those two fields
- **THEN** the card renders through that template, and its editor says it inherits it

#### Scenario: A card's own template wins

- **WHEN** a card has its own template while its deck has a different one
- **THEN** the card renders through its own and the deck's other cards are unaffected

#### Scenario: Clearing a deck template leaves the cards readable

- **WHEN** a deck's template is cleared while its cards hold fields
- **THEN** each card renders its own markdown and the fields stay stored

### Requirement: Fields survive editing and export

The system SHALL keep a card's fields when its text, tags or deck change, SHALL keep them when it is
moved to the trash and restored, and SHALL search them as part of the card's content.

#### Scenario: A moved card keeps its fields

- **WHEN** a card with fields is moved to another deck with a different template
- **THEN** its fields are still there and the new template renders with them

#### Scenario: Search reaches a field value

- **WHEN** the user searches for a value that appears only in a field
- **THEN** the card is listed
