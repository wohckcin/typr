# Spec Delta — review-scheduling

## ADDED Requirements

### Requirement: A reverse review is its own prompt

The system SHALL give every card that carries a review variant a second prompt keyed `rev`, SHALL keep
its phase, interval, due date and history separate from every other prompt of that card, and SHALL
grade it from the same two binary grades.

#### Scenario: The two directions are counted separately

- **WHEN** a two-sided card is reviewed forward and remembered, then reviewed in reverse
- **THEN** `fwd` holds one record and `rev` holds none, and the card's review count is one

#### Scenario: A reverse grade schedules only itself

- **WHEN** the `rev` prompt is graded remembered while `fwd` is due
- **THEN** `rev` moves to its new interval and `fwd` keeps its own due date

### Requirement: Reverse is enabled per card, deck or library

The system SHALL resolve whether a card is reviewed in reverse from the card's own setting, else the
nearest ancestor deck that sets one, else the library default, and SHALL default to off. Changing any
of those settings SHALL affect future reviews only: no prompt row, record or interval SHALL be deleted
or recomputed, and a `rev` prompt that stops being enabled SHALL cease to queue while keeping its
history.

#### Scenario: A card setting beats its deck

- **WHEN** a card inside a deck that enables reverse disables it for itself
- **THEN** only `fwd` queues for that card, and the card's deck still enables reverse for its siblings

#### Scenario: Turning reverse off keeps the history

- **WHEN** the library default was on, a card has been reviewed in reverse twice, and the default is
  switched off
- **THEN** the `rev` prompt disappears from both queues and its two records remain readable

#### Scenario: Enabling reverse on a deck reaches its descendants

- **WHEN** reverse is enabled on a deck that holds nested decks with cards
- **THEN** the cards in those nested decks queue in reverse too, unless one of them sets its own value
