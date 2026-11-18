# Spec Delta

## ADDED Requirements

### Requirement: The library refuses an empty text card

The library SHALL refuse to save a text card whose text is empty or only whitespace, whichever
caller asks, and SHALL leave the stored card unchanged. The editor's Save SHALL be enabled by the
same rule the library applies.

#### Scenario: A whitespace save is refused by the library

- **WHEN** a text card holding `hola` is saved as `  \n `
- **THEN** the save is refused with an error saying the card is empty and the card still reads
  `hola`

#### Scenario: The editor and the library agree

- **WHEN** the editor's text is only whitespace
- **THEN** Save is disabled, and the same text sent straight to the library is refused

### Requirement: A save drops the stored draft in the same write

Saving a card SHALL remove its stored draft in the same transaction as the save, so a save never
leaves a draft behind and a refused save never loses one.

#### Scenario: A saved card reopens clean

- **WHEN** a card with a stored draft is saved and then reopened
- **THEN** the editor shows the saved text with no unsaved marker

#### Scenario: A refused save keeps the draft

- **WHEN** a save is refused
- **THEN** the stored draft is still there on the next open
