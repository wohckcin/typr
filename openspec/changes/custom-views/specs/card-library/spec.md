# Spec Delta — card-library

## ADDED Requirements

### Requirement: A view is a saved filter with a name

The system SHALL let the user save the library's filter state as a named view, made of an optional deck
scope, a search text, a set of tags, a mode, a pinned-only flag and a sort order, SHALL list saved
views beside the decks, and SHALL apply every part of a view at once when it is selected.

#### Scenario: A saved view is applied whole

- **WHEN** a view saved as "Exam week" holds the tag `exam`, the mode active and the sort by due date,
  and the user selects it
- **THEN** the list shows active cards tagged `exam` from the whole library, ordered by due date

#### Scenario: A view survives a restart

- **WHEN** a view is created, the app is closed and reopened
- **THEN** the view is listed with the same name and the same filter

#### Scenario: A view can be renamed and deleted

- **WHEN** the user renames a view and later deletes it
- **THEN** the new name is stored and deleting it removes only the view, not the cards it matched

### Requirement: A card can be pinned

The system SHALL let any card be pinned, SHALL show it as pinned in the list, SHALL list only pinned
cards when a view asks for that, and SHALL keep the pin when the card is moved, archived, trashed and
restored.

#### Scenario: A pinned card is listed without a deck

- **WHEN** a view lists pinned cards and one card in a nested deck is pinned
- **THEN** that card is listed and its neighbours are not

#### Scenario: A pin survives the trash

- **WHEN** a pinned card is trashed and then restored
- **THEN** it is still pinned and its history is unchanged

### Requirement: The list can be ordered

The system SHALL order the list by last edited, created, first side, next due date, review count, or a
shuffle, in either direction where an order exists, and SHALL keep the chosen order when the list
refreshes.

#### Scenario: Ordering by due date puts the soonest first

- **WHEN** the user orders a list by due date
- **THEN** cards with the earliest due prompts come first and cards with no prompt come last

#### Scenario: A shuffle does not persist

- **WHEN** the user shuffles the list, then reloads another view and returns
- **THEN** the list is in the saved view's order again, because a shuffle is not a saved sort

#### Scenario: Ordering is not a schedule change

- **WHEN** any order is chosen
- **THEN** no prompt's due date or interval changes
