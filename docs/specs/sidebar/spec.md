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
row. The column SHALL be fixed width and SHALL NOT resize. The navigation group SHALL hold
Dashboard, then Inbox with Due today and New cards nested under it. The decks group SHALL be
headed `Decks` and SHALL end with a `New Deck` row. The utilities group SHALL hold Import,
Settings and Trash, in that order.

#### Scenario: Groups appear in order
- **WHEN** the library screen is drawn with two decks
- **THEN** the navigation group comes first, the deck group second, and the utility
  group last, with a gap between groups

#### Scenario: An empty group is absent
- **WHEN** a group holds no row to draw
- **THEN** neither its heading nor its gap is drawn

#### Scenario: The rows read as the reference layout
- **WHEN** the library screen is drawn for a new library
- **THEN** the rows read, top to bottom: Dashboard, Inbox, Due today, New cards, Flashcards,
  Notes, New Deck, Import, Settings, Trash

#### Scenario: Views, tags and diagnostics are not rows
- **WHEN** the library holds a saved view and a tagged card
- **THEN** the sidebar draws no view row, no tag row, no `All cards` row and no `Diagnostics` row

### Requirement: A row is one target

Every sidebar row SHALL be a single press target at least 24 points tall, spanning the
column's width minus the inset, with a rounded fill on hover and on selection. A row's
label SHALL start at the same horizontal offset as every other row at the same depth in the same
group, so a column of labels reads straight.

#### Scenario: Pressing anywhere in a row acts
- **WHEN** the pointer presses a row 2 points from its left edge
- **THEN** the row's action runs, exactly as it does when the label is pressed

#### Scenario: Labels line up
- **WHEN** the sidebar draws the Import, Settings and Trash rows
- **THEN** all three labels begin at the same horizontal offset

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

The search row SHALL be drawn as a rounded field with a search mark, the word `Search`, and the
chord that focuses the search field, and that chord SHALL be read from the keymap table rather
than written into the row. Pressing the row SHALL focus the library's search field.

#### Scenario: The row shows what to press
- **WHEN** the sidebar is drawn and the keymap binds search to a chord
- **THEN** the search row draws that chord as a keycap

#### Scenario: Pressing the row focuses search
- **WHEN** the search row is pressed
- **THEN** the library screen is shown and its search field holds focus

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
- **THEN** the Dashboard row follows the search row with no state row between them

### Requirement: A row carries a leading icon

Every navigation, deck and utility row SHALL draw an icon before its label. The icon SHALL be
decorative: a screen reader SHALL hear the row's label once and not a name for the icon. A deck
named `Notes` SHALL draw the notes icon, and every other deck SHALL draw the deck icon.

#### Scenario: Each row has its mark
- **WHEN** the sidebar is drawn for a new library
- **THEN** every row from Dashboard to Trash draws an icon before its label

#### Scenario: The icon is not announced
- **WHEN** a screen reader reads the Inbox row
- **THEN** it hears `Inbox` and its count, and no name for the icon

### Requirement: The Inbox holds the cards that are waiting

The Inbox row SHALL select every studyable card that is due now or has at least one prompt that
was never scheduled. A card that is both SHALL appear once. A card with no studyable prompt (a
note) SHALL never appear. The Inbox SHALL be the selection in force when the library screen first opens.

#### Scenario: Due and new cards are both in the Inbox
- **WHEN** one card is due, another has never been scheduled, and a third is due next week
- **THEN** the Inbox shows the first two cards and not the third

#### Scenario: A card both due and new appears once
- **WHEN** a card has one prompt due and another never scheduled
- **THEN** the Inbox shows that card once and counts it once

#### Scenario: The Inbox opens first
- **WHEN** the library screen opens
- **THEN** the Inbox row is selected and the grid shows the Inbox's cards

#### Scenario: Trashed and archived cards stay out
- **WHEN** a due card is trashed and another sits in an archived deck
- **THEN** neither card is in the Inbox

### Requirement: Due today and New cards narrow the Inbox

The Due today row SHALL select the studyable cards with a prompt due now. The New cards row SHALL
select the studyable cards with a prompt that was never scheduled. Both rows SHALL be nested under
Inbox and SHALL fold away with it.

#### Scenario: Due today shows only due cards
- **WHEN** one card is due and another has never been scheduled
- **THEN** Due today shows the due card and New cards shows the other

#### Scenario: Folding the Inbox hides its children
- **WHEN** the Inbox row is folded
- **THEN** the Due today and New cards rows are not drawn, and the Inbox row still shows its count

### Requirement: An Inbox row states how many cards it holds

The Inbox, Due today and New cards rows SHALL each show, right-aligned, how many cards they select
at the moment the sidebar was last refreshed, and SHALL show no number when the count is zero.

#### Scenario: Counts match the rows' cards
- **WHEN** two cards are due and three are new, none both
- **THEN** Inbox shows `5`, Due today shows `2` and New cards shows `3`

#### Scenario: An empty Inbox is quiet
- **WHEN** nothing is due and nothing is new
- **THEN** the Inbox, Due today and New cards rows draw no count

### Requirement: Dashboard is shown but not yet available

The Dashboard row SHALL be drawn in the disabled style and SHALL NOT be selectable or open any
screen. It SHALL tell the user and a screen reader that it is coming later. The disabled style
SHALL NOT rely on colour alone.

#### Scenario: Pressing Dashboard does nothing
- **WHEN** the Dashboard row is pressed
- **THEN** the selection and the screen do not change

#### Scenario: Dashboard says why
- **WHEN** a screen reader reaches the Dashboard row
- **THEN** it hears that the row is unavailable and coming later

### Requirement: A deck can be made from the sidebar

The `New Deck` row SHALL turn into a name field when pressed. Enter with a non-blank name SHALL
create a top-level deck with that name (trimmed), select it, and show it in the deck tree. Escape,
or Enter with a blank name, SHALL cancel and restore the row without creating anything.

#### Scenario: Naming a deck creates it
- **WHEN** the user presses `New Deck`, types ` Spanish ` and presses Enter
- **THEN** a top-level deck named `Spanish` exists, is in the deck tree and is selected

#### Scenario: Escape cancels
- **WHEN** the user presses `New Deck`, types `Spa` and presses Escape
- **THEN** no deck is created and the `New Deck` row is drawn again

#### Scenario: A blank name creates nothing
- **WHEN** the user presses `New Deck` and presses Enter with the field empty
- **THEN** no deck is created

#### Scenario: A closed library cannot make decks
- **WHEN** the library is closed
- **THEN** the `New Deck` row is disabled

### Requirement: The utility rows act

The Import row SHALL open the import file dialog and SHALL report the import's outcome as a toast,
without changing the selection. The Settings row SHALL open the settings screen. The Trash row SHALL
open the trash screen.

#### Scenario: Import from the sidebar
- **WHEN** the user presses Import and picks an archive
- **THEN** the archive is imported, a toast states what was imported, and the selected row is unchanged

#### Scenario: Cancelling import changes nothing
- **WHEN** the user presses Import and dismisses the dialog
- **THEN** nothing is imported and no toast appears

#### Scenario: Trash opens its screen
- **WHEN** the Trash row is pressed
- **THEN** the trash screen is shown and the Trash row is selected
