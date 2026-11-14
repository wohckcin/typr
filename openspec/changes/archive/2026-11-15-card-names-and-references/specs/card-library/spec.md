# Spec Delta — card-library

## ADDED Requirements

### Requirement: Every card has a name

The system SHALL give each card a name, computed from its name template when it has one and otherwise
from its first side, SHALL keep names unique within a deck when it can do so by adding a suffix, SHALL
show the name wherever a card is listed, and SHALL update it when the text or fields it derives from
change.

#### Scenario: A card without a name template is named by its first side

- **WHEN** a card's first side is "What is mitosis?" and it has no name template
- **THEN** its name is "What is mitosis?" and the list shows that

#### Scenario: A name template wins

- **WHEN** a card's name template is `<<Word>> (verb)` and the field `Word` is "hablar"
- **THEN** its name is "hablar (verb)"

#### Scenario: Two cards may share a name in different decks

- **WHEN** two decks each hold a card named "Introduction"
- **THEN** both names stand and a reference to "Introduction" resolves inside the referencing deck first

### Requirement: A card can reference another

The system SHALL treat `[[Name]]` in a card's text as a reference, SHALL resolve it to the card with
that name preferring the referencing card's own deck and then its ancestors, SHALL render a resolved
reference as a control that opens the target card, and SHALL NOT alter a card's text when resolving.

#### Scenario: A reference opens the card it names

- **WHEN** a card contains `[[Krebs cycle]]` and a card with that name exists
- **THEN** the reference is drawn as a control, and using it opens the named card in the editor

#### Scenario: An unresolved reference stays text

- **WHEN** the reference names no card
- **THEN** it is drawn as ordinary text and reported as unresolved in the editor

#### Scenario: Resolution prefers the nearest deck

- **WHEN** two decks hold "Introduction" and a card in one of them references it
- **THEN** the reference opens the card in the referencing card's own deck

### Requirement: Every reference is listed on its target

The system SHALL list, on each card, the cards that reference it, SHALL update that list when either
side changes, and SHALL keep the list after a reference is renamed away by marking it unresolved.

#### Scenario: Backlinks appear on the target

- **WHEN** three cards reference "Krebs cycle"
- **THEN** that card's editor lists all three, and each entry opens the referencing card

#### Scenario: Renaming the target keeps the history of the link

- **WHEN** a referenced card is renamed so that no reference matches
- **THEN** the referencing cards still hold their `[[…]]` text and the target's backlink list is empty

### Requirement: Tags nest by path

The system SHALL treat a slash inside a tag as a path separator, SHALL list a tag's children under it,
and SHALL, when filtering by a tag, include the cards of its descendants.

#### Scenario: Filtering a parent includes its children

- **WHEN** cards are tagged `spanish/verbs` and `spanish/verbs/past` and the user filters by
  `spanish/verbs`
- **THEN** both cards are listed

#### Scenario: A tag path is one tag

- **WHEN** the user types `spanish/verbs` as a tag
- **THEN** it is stored as that one tag with two segments, not as two tags
