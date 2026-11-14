# Spec Delta — card-library

## ADDED Requirements

### Requirement: The library says which direction a card is waiting on

The system SHALL show, for each card that is enabled in reverse, that both directions are in play, and
SHALL offer a control that toggles reverse for that one card without opening it.

#### Scenario: A card shows its directions

- **WHEN** a two-sided card is enabled in reverse and its list row is drawn
- **THEN** the row says the card is asked in both directions, and a card without reverse does not

#### Scenario: The row toggles one card

- **WHEN** the user toggles reverse off from the card's row
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
