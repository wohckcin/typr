## MODIFIED Requirements

### Requirement: One table declares every binding

The system SHALL declare each command's key chord and surface in one table, SHALL dispatch keys through that
table, and SHALL draw keycap hints from it, so that no chord is handled anywhere else. A chord SHALL reach the
toolkit as the toolkit's own keystroke text, written by `nsh_core::keymap::Keystroke`.

#### Scenario: A chord in the table works everywhere it applies

- **WHEN** the table binds a chord on the library surface and the library is open
- **THEN** pressing the chord runs that command

#### Scenario: A chord absent from the table does nothing

- **WHEN** a key that the table does not bind is pressed on a surface
- **THEN** nothing happens and no error is shown

#### Scenario: Every chord is a keystroke the toolkit parses

- **WHEN** any chord of the table is turned into its keystroke text
- **THEN** `Keystroke::parse_with` accepts that text
