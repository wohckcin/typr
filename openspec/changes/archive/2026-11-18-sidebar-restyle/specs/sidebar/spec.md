## MODIFIED Requirements

### Requirement: The sidebar states the library's state

The sidebar SHALL carry a row that states whether the library is open, drawn as a dot and the state text, and a
closed library SHALL be stated in the danger tone, not in the muted tone used for secondary text. The dot SHALL
differ from the closed library's dot by colour, and the text SHALL name the state, so neither colour alone carries
it.

#### Scenario: A closed library reads as a problem

- **WHEN** the store could not be opened
- **THEN** the state row reads in the danger tone and names the reason on the status
  line below the screen

#### Scenario: An open library has a mark and a name

- **WHEN** the library is open
- **THEN** the state row draws a dot in the success colour and the text `Library open`

#### Scenario: A closed library has a mark and a name

- **WHEN** the library is closed
- **THEN** the state row draws a dot in the danger colour and text beginning `Library closed`

## ADDED Requirements

### Requirement: The state row is the foot of the column

The state row SHALL be the last element of the sidebar, pinned to the bottom of the column and separated from the
groups above it by one hairline, whatever the window's height and however few rows the groups hold. The search row
SHALL be the first element, and the groups SHALL follow it directly.

#### Scenario: The status stays at the bottom

- **WHEN** the sidebar is drawn in a tall window with two decks
- **THEN** the state row is below every group's last row and at the bottom edge of the column

#### Scenario: The groups start under the search row

- **WHEN** the sidebar is drawn
- **THEN** the first group's heading follows the search row with no state row between them
