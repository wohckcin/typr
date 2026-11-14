# Spec Delta — card-library

## ADDED Requirements

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
