# sidebar Specification

## Purpose

Give the library screen a fixed column that says where the user is and what is waiting:
navigation rows, the deck tree with the prompts due under each deck, and the utilities,
all drawn as one list of rows that share a height and a label column, with the search
row's chord read from the keymap table and a state row that names the reason when the
library is not open.

## Requirements

### Requirement: The sidebar states where the user is and what is waiting

The sidebar SHALL be a column of groups: navigation, decks, and utilities. Each group
SHALL be separated from the next by a gap, and each SHALL render only when it holds a
row. The column SHALL be fixed width and SHALL NOT resize.

#### Scenario: Groups appear in order
- **WHEN** the library screen is drawn with two decks
- **THEN** the navigation group comes first, the deck group second, and the utility
  group last, with a gap between groups

#### Scenario: An empty group is absent
- **WHEN** the library holds no tags
- **THEN** no tag section is drawn at all

### Requirement: A row is one target

Every sidebar row SHALL be a single press target at least 24 points tall, spanning the
column's width minus the inset, with a rounded fill on hover and on selection. A row's
label SHALL start at the same horizontal offset for every row, so a column of labels
reads straight.

#### Scenario: Pressing anywhere in a row acts
- **WHEN** the pointer presses a row 2 points from its left edge
- **THEN** the row's action runs, exactly as it does when the label is pressed

#### Scenario: Labels line up
- **WHEN** the sidebar draws a deck row and a utility row
- **THEN** both labels begin at the same horizontal offset

### Requirement: A deck row states how much work it holds

A deck row SHALL show the number of prompts due in that deck's subtree at the moment
the sidebar was last refreshed, right-aligned, and SHALL show no number when the count
is zero.

#### Scenario: A deck with work shows its count
- **WHEN** one card under `Spanish` is due and another is not
- **THEN** the `Spanish` row shows `1`

#### Scenario: A quiet deck shows nothing
- **WHEN** no card under a deck is due
- **THEN** that deck's row draws no count

### Requirement: Nested decks can be folded away

A deck row whose deck has children SHALL offer a control that collapses its subtree,
and collapsing SHALL hide every descendant row. The collapsed state SHALL survive a
refresh of the library and SHALL NOT be written to the database.

#### Scenario: A collapsed deck hides its children
- **WHEN** a deck with one child is collapsed
- **THEN** the child row is not drawn, and the parent row still shows its own count

#### Scenario: Collapse survives a reload of the snapshot
- **WHEN** the library refreshes while a deck is collapsed
- **THEN** the deck is still collapsed

### Requirement: The current deck is marked by its row

The row of the deck the library screen filters on SHALL be drawn with a filled
background, and its label SHALL be at full contrast. Colour alone SHALL NOT be the
only difference.

#### Scenario: The selected deck and its siblings differ
- **WHEN** deck `A` is selected and deck `B` is not
- **THEN** the `A` row carries a background fill while the `B` row does not

### Requirement: The search row carries its chord

The search row SHALL draw the chord that focuses the search field, and that chord SHALL
be read from the keymap table rather than written into the row.

#### Scenario: The row shows what to press
- **WHEN** the sidebar is drawn and the keymap binds search to a chord
- **THEN** the search row draws that chord as a keycap

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
