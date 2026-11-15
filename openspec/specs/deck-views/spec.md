# deck-views Specification

## Purpose

A deck can be read in one of four ways — as a grid of tiles, as a list of one line per card, as a
notebook of whole card contents, and as a column showing a single card at reading width — and the
deck remembers the one it was left in. Moving the selection, acting on a card and reordering the
deck behave the same way in all four.

## Requirements

### Requirement: A deck is shown in one of four views

The system SHALL offer four views of a deck: a grid of tiles, a list of one line per card, a notebook of full
card contents, and a column that shows one card's contents at reading width. Exactly one view SHALL be active
per deck at a time.

#### Scenario: The deck header names the active view

- **WHEN** a deck is open in the grid view
- **THEN** the deck header states that the grid view is active and offers the other three

#### Scenario: Switching views keeps the selection

- **WHEN** a card is selected and the user switches from grid to list
- **THEN** the same card is selected in the new view

### Requirement: A deck remembers the view it was left in

The system SHALL store the view per deck in the library and SHALL restore it when the deck is next opened and
after a restart.

#### Scenario: The view survives a restart

- **WHEN** a deck is set to the notebook view and the application is closed and reopened
- **THEN** that deck opens in the notebook view

#### Scenario: One deck's view does not change another's

- **WHEN** two decks hold cards and one is set to the list view
- **THEN** the other deck keeps whichever view it held

### Requirement: Selection moves the same way in every view

The system SHALL move the selection to the next and previous card with the movement keys and the arrow keys in
every view, SHALL keep the selection inside the deck, and SHALL scroll the selected card into view.

#### Scenario: The selection stops at the ends

- **WHEN** the last card is selected and the user moves forward
- **THEN** the selection stays at the last card

#### Scenario: Moving the selection scrolls

- **WHEN** the selection moves to a card outside the visible area
- **THEN** the view scrolls so that card is visible

### Requirement: Every view acts on the selected card

The system SHALL offer edit, preview and delete on the selected card from the keyboard in every view, and SHALL
offer add-below and add-above in the notebook view.

#### Scenario: Editing from the list view

- **WHEN** a card is selected in the list view and the user presses the edit binding
- **THEN** that card opens in the editor

#### Scenario: Adding below in the notebook view

- **WHEN** a card is selected in the notebook view and the user adds a card below it
- **THEN** a new card is opened between the selected card and the next one

### Requirement: Moving a card changes its place in the deck

The system SHALL reorder the selected card within its deck when the move keys are used, SHALL persist the new
order, and SHALL NOT change the card's schedule.

#### Scenario: Moving up persists

- **WHEN** the second card is moved up and the library is reopened
- **THEN** that card is still first and its prompt keeps its due date

### Requirement: The column view finds text inside the selected card

The system SHALL offer a find action in the column view that searches the selected card's text, SHALL move to
the next and previous match, and SHALL report when there is no match.

#### Scenario: A match is highlighted and reachable

- **WHEN** the user searches for text present twice in the selected card
- **THEN** each match can be reached in turn and the view states which match is current
