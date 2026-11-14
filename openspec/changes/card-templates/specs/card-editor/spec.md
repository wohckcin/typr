# Spec Delta — card-editor

## ADDED Requirements

### Requirement: A card's text can come from a template and its fields

The system SHALL let a card carry named fields and SHALL render the card's text by replacing each
`<<Field>>` placeholder in the template with that field's value, SHALL treat a placeholder with no
value as empty text, and SHALL leave a card without a template exactly as its own markdown.

#### Scenario: Fields fill the template

- **WHEN** a card has the template `<<Word>>\n---\n<<Meaning>>` and fields `Word: gato`,
  `Meaning: cat`
- **THEN** its text is a two-sided card with "gato" and "cat", and it holds one `fwd` prompt

#### Scenario: An unfilled field is empty

- **WHEN** a template names `<<Example>>` and the card has no value for it
- **THEN** the placeholder renders as nothing and the card still saves

#### Scenario: A card without a template keeps its markdown

- **WHEN** a card in a deck with no template is edited
- **THEN** its markdown is what is stored and no field table is shown

### Requirement: A section can be shown only when a field has a value

The system SHALL render `{{#Field}}…{{/Field}}` only when the named field has a non-empty value, SHALL
render nothing in its place when it does not, and SHALL NOT let an unclosed section hide the rest of
the card.

#### Scenario: A filled section appears

- **WHEN** the field `Example` holds "el gato duerme" and the template wraps it in a section
- **THEN** the rendered card contains that sentence

#### Scenario: An empty section disappears

- **WHEN** the same card has no `Example` value
- **THEN** the section and its formatting are gone, and the sides around it are unchanged

#### Scenario: An unclosed section is not swallowed

- **WHEN** a template opens a section and never closes it
- **THEN** the text after the opening marker is rendered as ordinary text

### Requirement: Some fields take their value from the card

The system SHALL provide fields whose value is computed rather than typed, at least `Name` (the card's
first side), and `Deck` (the card's deck name), and SHALL mark them as not editable.

#### Scenario: A computed field renders

- **WHEN** a template uses `<<Deck>>` on a card in the deck "Spanish"
- **THEN** the rendered text contains "Spanish"

#### Scenario: A computed field is not editable

- **WHEN** the field table lists `Name` and `Deck`
- **THEN** neither offers a text field to type into

### Requirement: Sides come from the rendered card

The system SHALL compute a card's sides from its rendered text, so a template holding a `---` line gives
every card that inherits it two sides without any card typing a separator, and SHALL recompute them
whenever a field, a template or a deck template changes.

#### Scenario: One separator for the whole deck

- **WHEN** fifty cards inherit a deck template containing a `---` line
- **THEN** each of them is a two-sided card and each holds the prompt its text deserves

#### Scenario: Emptying a field can make a card a note

- **WHEN** a template's second side is only `<<Answer>>` and a card's value is cleared
- **THEN** the rendered text has one side, the card keeps its history, and it stops queueing
